use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::{Local, Datelike, NaiveDate, Duration};
use crate::error::{AppError, AppResult};

pub struct DbState(pub Arc<Mutex<Option<libsql::Database>>>);

// ── Tipos compartidos ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct Budget {
    pub category: String,
    pub monthly_amount: i64,
}

#[derive(Serialize, Debug)]
pub struct Transaction {
    pub id: i64,
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
    pub created_at: String,
}

#[derive(Deserialize, Debug)]
pub struct TransactionInput {
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct TransactionFilter {
    pub period: Option<Period>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub search_note: Option<String>,
    pub only_extraordinary: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Custom { start: String, end: String },
}

#[derive(Serialize, Debug)]
pub struct PeriodSummary {
    pub total_income: i64,
    pub total_expenses: i64,
    pub balance: i64,
    pub extraordinary_income: i64,
    pub extraordinary_expenses: i64,
    pub transactions_count: i64,
}

#[derive(Serialize, Debug)]
pub struct CategoryProgress {
    pub category: String,
    pub monthly_target: i64,
    pub current_amount: i64,
    pub percentage: f64,
    pub is_over: bool,
    pub kind: String,
}

#[derive(Serialize, Debug)]
pub struct CategoryComparison {
    pub category: String,
    pub current: i64,
    pub previous: i64,
    pub delta_pct: f64,
}

#[derive(Serialize, Debug)]
pub struct MonthComparison {
    pub current_month_total: i64,
    pub previous_month_total: i64,
    pub delta_amount: i64,
    pub delta_percentage: f64,
    pub by_category: Vec<CategoryComparison>,
}

#[derive(Serialize, Debug)]
pub struct CsvExport {
    pub content: String,
    pub suggested_filename: String,
}

#[derive(Serialize, Debug)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: String,
    pub created_at: String,
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as u32
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn period_to_dates(period: &Period) -> (String, String) {
    let today = Local::now().date_naive();

    let (start, end) = match period {
        Period::Daily => (today, today),
        Period::Weekly => {
            let days = today.weekday().num_days_from_monday() as i64;
            (today - Duration::days(days), today)
        }
        Period::Monthly => {
            let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            (first, today)
        }
        Period::Yearly => {
            let first = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            (first, today)
        }
        Period::Custom { start, end } => {
            return (start.clone(), end.clone());
        }
    };
    (start.format("%Y-%m-%d").to_string(), end.format("%Y-%m-%d").to_string())
}

async fn get_conn(state: &State<'_, DbState>) -> AppResult<libsql::Connection> {
    let guard = state.0.lock().await;
    guard
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("base de datos no inicializada".into()))?
        .connect()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

fn spawn_sync(arc: Arc<Mutex<Option<libsql::Database>>>) {
    tauri::async_runtime::spawn(async move {
        let guard = arc.lock().await;
        if let Some(db) = guard.as_ref() {
            if let Err(e) = db.sync().await {
                eprintln!("[finanzas] background sync error: {e}");
            }
        }
    });
}

fn row_to_transaction(row: &libsql::Row) -> Result<Transaction, libsql::Error> {
    Ok(Transaction {
        id: row.get(0)?,
        date: row.get(1)?,
        kind: row.get(2)?,
        category: row.get(3)?,
        amount: row.get(4)?,
        note: row.get(5)?,
        is_extraordinary: row.get::<i64>(6)? != 0,
        goal_id: row.get(7)?,
        created_at: row.get(8)?,
    })
}

// ── Comandos ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_budgets(state: State<'_, DbState>) -> AppResult<Vec<Budget>> {
    let conn = get_conn(&state).await?;
    let mut rows = conn
        .query("SELECT category, monthly_amount FROM budgets ORDER BY category", ())
        .await?;

    let mut budgets = Vec::new();
    while let Some(row) = rows.next().await? {
        budgets.push(Budget {
            category: row.get(0)?,
            monthly_amount: row.get(1)?,
        });
    }
    Ok(budgets)
}

#[tauri::command]
pub async fn create_transaction(
    state: State<'_, DbState>,
    input: TransactionInput,
) -> AppResult<Transaction> {
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    if !matches!(input.kind.as_str(), "ingreso" | "gasto") {
        return Err(AppError::ValidationError(
            "tipo debe ser 'ingreso' o 'gasto'".into(),
        ));
    }

    let conn = get_conn(&state).await?;
    conn.execute(
        "INSERT INTO transactions (date, type, category, amount, note, is_extraordinary, goal_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            input.date.clone(),
            input.kind.clone(),
            input.category.clone(),
            input.amount,
            input.note.clone(),
            input.is_extraordinary as i64,
            input.goal_id
        ],
    )
    .await?;

    let id = conn.last_insert_rowid();
    let mut rows = conn
        .query(
            "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at \
             FROM transactions WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound("transacción recién insertada no encontrada".into()))?;
    let tx = row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    spawn_sync(Arc::clone(&state.0));
    Ok(tx)
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, DbState>,
    filter: TransactionFilter,
) -> AppResult<Vec<Transaction>> {
    let conn = get_conn(&state).await?;

    let mut sql = "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at \
                   FROM transactions WHERE 1=1"
        .to_string();
    let mut params: Vec<libsql::Value> = Vec::new();

    if let Some(period) = &filter.period {
        let (start, end) = period_to_dates(period);
        sql.push_str(" AND date >= ? AND date <= ?");
        params.push(start.into());
        params.push(end.into());
    }
    if let Some(kind) = &filter.kind {
        sql.push_str(" AND type = ?");
        params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category {
        sql.push_str(" AND category = ?");
        params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        sql.push_str(" AND is_extraordinary = 1");
    }
    if let Some(note) = &filter.search_note {
        sql.push_str(" AND note LIKE ?");
        params.push(format!("%{note}%").into());
    }
    sql.push_str(" ORDER BY date DESC, id DESC");

    let mut rows = conn.query(&sql, params).await?;
    let mut txs = Vec::new();
    while let Some(row) = rows.next().await? {
        txs.push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(txs)
}

#[tauri::command]
pub async fn update_transaction(
    state: State<'_, DbState>,
    id: i64,
    input: TransactionInput,
) -> AppResult<Transaction> {
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    if !matches!(input.kind.as_str(), "ingreso" | "gasto") {
        return Err(AppError::ValidationError(
            "tipo debe ser 'ingreso' o 'gasto'".into(),
        ));
    }

    let conn = get_conn(&state).await?;
    let affected = conn
        .execute(
            "UPDATE transactions \
             SET date=?, type=?, category=?, amount=?, note=?, is_extraordinary=?, goal_id=? \
             WHERE id=?",
            libsql::params![
                input.date.clone(),
                input.kind.clone(),
                input.category.clone(),
                input.amount,
                input.note.clone(),
                input.is_extraordinary as i64,
                input.goal_id,
                id
            ],
        )
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound(format!("transacción {id} no existe")));
    }

    let mut rows = conn
        .query(
            "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at \
             FROM transactions WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("transacción {id} no existe")))?;
    let tx = row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    spawn_sync(Arc::clone(&state.0));
    Ok(tx)
}

#[tauri::command]
pub async fn delete_transaction(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    let affected = conn
        .execute(
            "DELETE FROM transactions WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound(format!("transacción {id} no existe")));
    }

    spawn_sync(Arc::clone(&state.0));
    Ok(())
}

#[tauri::command]
pub async fn get_period_summary(
    state: State<'_, DbState>,
    period: Period,
) -> AppResult<PeriodSummary> {
    let conn = get_conn(&state).await?;
    let (start, end) = period_to_dates(&period);

    let mut rows = conn
        .query(
            "SELECT
                COALESCE(SUM(CASE WHEN type='ingreso' THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN type='gasto'   THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN type='ingreso' AND is_extraordinary=1 THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN type='gasto'   AND is_extraordinary=1 THEN amount ELSE 0 END), 0),
                COUNT(*)
             FROM transactions WHERE date >= ? AND date <= ?",
            libsql::params![start, end],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("summary sin resultados".into()))?;

    let total_income: i64 = row.get(0)?;
    let total_expenses: i64 = row.get(1)?;
    let extraordinary_income: i64 = row.get(2)?;
    let extraordinary_expenses: i64 = row.get(3)?;
    let transactions_count: i64 = row.get(4)?;

    Ok(PeriodSummary {
        total_income,
        total_expenses,
        balance: total_income - total_expenses,
        extraordinary_income,
        extraordinary_expenses,
        transactions_count,
    })
}

#[tauri::command]
pub async fn get_category_progress(
    state: State<'_, DbState>,
    period: Period,
) -> AppResult<Vec<CategoryProgress>> {
    let conn = get_conn(&state).await?;
    let (start, end) = period_to_dates(&period);

    let mut rows = conn
        .query(
            "SELECT
                b.category,
                b.monthly_amount,
                COALESCE((
                    SELECT SUM(amount) FROM transactions
                    WHERE category = b.category AND date >= ? AND date <= ?
                ), 0) AS current_amount,
                COALESCE((
                    SELECT type FROM transactions
                    WHERE category = b.category AND date >= ? AND date <= ?
                    GROUP BY type ORDER BY COUNT(*) DESC LIMIT 1
                ), 'gasto') AS inferred_kind
             FROM budgets b
             WHERE b.monthly_amount > 0
                OR EXISTS (
                    SELECT 1 FROM transactions
                    WHERE category = b.category AND date >= ? AND date <= ?
                )
             ORDER BY b.category",
            libsql::params![start.clone(), end.clone(), start.clone(), end.clone(), start, end],
        )
        .await?;

    let mut progress = Vec::new();
    while let Some(row) = rows.next().await? {
        let category: String = row.get(0)?;
        let monthly_target: i64 = row.get(1)?;
        let current_amount: i64 = row.get(2)?;
        let kind: String = row.get(3)?;

        let percentage = if monthly_target > 0 {
            (current_amount as f64 / monthly_target as f64) * 100.0
        } else {
            0.0
        };

        progress.push(CategoryProgress {
            is_over: monthly_target > 0 && current_amount > monthly_target,
            category,
            monthly_target,
            current_amount,
            percentage,
            kind,
        });
    }
    Ok(progress)
}

#[tauri::command]
pub async fn get_month_comparison(state: State<'_, DbState>) -> AppResult<MonthComparison> {
    let conn = get_conn(&state).await?;
    let today = Local::now().date_naive();

    let curr_first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let curr_last = NaiveDate::from_ymd_opt(
        today.year(), today.month(), days_in_month(today.year(), today.month()),
    ).unwrap();

    let (prev_year, prev_month) = if today.month() == 1 {
        (today.year() - 1, 12u32)
    } else {
        (today.year(), today.month() - 1)
    };
    let prev_first = NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap();
    let prev_last = NaiveDate::from_ymd_opt(
        prev_year, prev_month, days_in_month(prev_year, prev_month),
    ).unwrap();

    let cs = curr_first.format("%Y-%m-%d").to_string();
    let ce = curr_last.format("%Y-%m-%d").to_string();
    let ps = prev_first.format("%Y-%m-%d").to_string();
    let pe = prev_last.format("%Y-%m-%d").to_string();

    let mut rows = conn
        .query(
            "SELECT
                category,
                COALESCE(SUM(CASE WHEN date >= ? AND date <= ? THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN date >= ? AND date <= ? THEN amount ELSE 0 END), 0)
             FROM transactions
             WHERE type = 'gasto'
               AND ((date >= ? AND date <= ?) OR (date >= ? AND date <= ?))
             GROUP BY category
             ORDER BY 2 DESC",
            libsql::params![
                cs.clone(), ce.clone(),
                ps.clone(), pe.clone(),
                cs.clone(), ce.clone(),
                ps.clone(), pe.clone()
            ],
        )
        .await?;

    let mut by_category: Vec<CategoryComparison> = Vec::new();
    let mut current_month_total: i64 = 0;
    let mut previous_month_total: i64 = 0;

    while let Some(row) = rows.next().await? {
        let category: String = row.get(0)?;
        let current: i64 = row.get(1)?;
        let previous: i64 = row.get(2)?;
        let delta_pct = if previous > 0 {
            (current - previous) as f64 / previous as f64 * 100.0
        } else {
            0.0
        };
        current_month_total += current;
        previous_month_total += previous;
        by_category.push(CategoryComparison { category, current, previous, delta_pct });
    }

    let delta_amount = current_month_total - previous_month_total;
    let delta_percentage = if previous_month_total > 0 {
        delta_amount as f64 / previous_month_total as f64 * 100.0
    } else {
        0.0
    };

    Ok(MonthComparison {
        current_month_total,
        previous_month_total,
        delta_amount,
        delta_percentage,
        by_category,
    })
}

const INCOME_DEFAULTS: &[&str] = &[
    "Carrera cuñada", "Carrera mamá", "Eventual", "Mesada", "Otro ingreso",
];
const EXPENSE_DEFAULTS: &[&str] = &[
    "Gasolina", "Imprevisto", "Mantenimiento", "Otro gasto", "Parqueadero", "Social/Salidas",
];

#[tauri::command]
pub async fn list_categories(
    state: State<'_, DbState>,
    kind: Option<String>,
) -> AppResult<Vec<String>> {
    let conn = get_conn(&state).await?;
    let mut cats: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    match &kind {
        None => {
            let mut rows = conn.query("SELECT category FROM budgets ORDER BY category", ()).await?;
            while let Some(row) = rows.next().await? { cats.insert(row.get(0)?); }
            let mut rows = conn.query("SELECT DISTINCT category FROM transactions ORDER BY category", ()).await?;
            while let Some(row) = rows.next().await? { cats.insert(row.get(0)?); }
        }
        Some(k) => {
            let mut rows = conn
                .query(
                    "SELECT DISTINCT category FROM transactions WHERE type = ? ORDER BY category",
                    libsql::params![k.clone()],
                )
                .await?;

            let mut has_history = false;
            while let Some(row) = rows.next().await? {
                cats.insert(row.get(0)?);
                has_history = true;
            }

            // Siempre incluir las categorías convencionales del tipo
            let defaults: &[&str] = if k == "ingreso" { INCOME_DEFAULTS } else { EXPENSE_DEFAULTS };
            for d in defaults { cats.insert(d.to_string()); }

            // Si no hay historial ni convenciones coinciden, igual devuelve las convencionales
            let _ = has_history;
        }
    }

    Ok(cats.into_iter().collect())
}

#[tauri::command]
pub async fn list_active_goals(state: State<'_, DbState>) -> AppResult<Vec<Goal>> {
    let conn = get_conn(&state).await?;
    let mut rows = conn
        .query(
            "SELECT id, name, target_amount, target_date, status, created_at \
             FROM goals WHERE status = 'activo' ORDER BY name",
            (),
        )
        .await?;

    let mut goals = Vec::new();
    while let Some(row) = rows.next().await? {
        goals.push(Goal {
            id: row.get(0)?,
            name: row.get(1)?,
            target_amount: row.get(2)?,
            target_date: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
        });
    }
    Ok(goals)
}

#[tauri::command]
pub async fn export_transactions_csv(
    state: State<'_, DbState>,
    filter: TransactionFilter,
) -> AppResult<CsvExport> {
    let conn = get_conn(&state).await?;

    let mut sql = "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at \
                   FROM transactions WHERE 1=1"
        .to_string();
    let mut params: Vec<libsql::Value> = Vec::new();

    if let Some(period) = &filter.period {
        let (start, end) = period_to_dates(period);
        sql.push_str(" AND date >= ? AND date <= ?");
        params.push(start.into());
        params.push(end.into());
    }
    if let Some(kind) = &filter.kind {
        sql.push_str(" AND type = ?");
        params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category {
        sql.push_str(" AND category = ?");
        params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        sql.push_str(" AND is_extraordinary = 1");
    }
    if let Some(note) = &filter.search_note {
        sql.push_str(" AND note LIKE ?");
        params.push(format!("%{note}%").into());
    }
    sql.push_str(" ORDER BY date DESC, id DESC");

    let mut rows = conn.query(&sql, params).await?;
    let mut csv = String::from(
        "ID,Fecha,Tipo,Categoría,Monto (COP),Nota,Extraordinario,ID Objetivo,Creado en\n",
    );

    while let Some(row) = rows.next().await? {
        let tx = row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            tx.id,
            tx.date,
            tx.kind,
            csv_escape(&tx.category),
            tx.amount,
            csv_escape(tx.note.as_deref().unwrap_or("")),
            if tx.is_extraordinary { "Sí" } else { "No" },
            tx.goal_id.map(|id| id.to_string()).unwrap_or_default(),
            tx.created_at,
        ));
    }

    let today = Local::now().format("%Y-%m-%d").to_string();
    Ok(CsvExport {
        content: csv,
        suggested_filename: format!("transacciones_{today}.csv"),
    })
}

// ── CSV Import ─────────────────────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') { current.push('"'); chars.next(); }
                else { in_quotes = false; }
            }
            '"' => { in_quotes = true; }
            ',' if !in_quotes => { fields.push(std::mem::take(&mut current)); }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

fn is_valid_date(s: &str) -> bool {
    s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

#[tauri::command]
pub async fn import_transactions_csv(
    state: State<'_, DbState>,
    csv_content: String,
) -> AppResult<ImportResult> {
    let conn = get_conn(&state).await?;

    // Categorías válidas
    let mut valid_cats: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut cat_rows = conn.query("SELECT category FROM budgets", ()).await?;
    while let Some(row) = cat_rows.next().await? { valid_cats.insert(row.get(0)?); }

    let mut imported = 0i64;
    let mut skipped = 0i64;
    let mut errors: Vec<String> = Vec::new();

    let mut lines = csv_content.lines();
    lines.next(); // saltar encabezado

    for (i, line) in lines.enumerate() {
        let row_num = i + 2;
        if line.trim().is_empty() { continue; }

        let fields = parse_csv_line(line);

        // Detectar si la primera columna es un ID numérico (formato export) o una fecha
        let offset = if fields[0].trim().parse::<i64>().is_ok() { 1usize } else { 0 };

        if fields.len() < offset + 4 {
            skipped += 1;
            errors.push(format!("Fila {row_num}: columnas insuficientes"));
            continue;
        }

        let date = fields[offset].trim().to_string();
        if !is_valid_date(&date) {
            skipped += 1;
            errors.push(format!("Fila {row_num}: fecha inválida '{date}'"));
            continue;
        }

        let kind = fields[offset + 1].trim().to_lowercase();
        if kind != "ingreso" && kind != "gasto" {
            skipped += 1;
            errors.push(format!("Fila {row_num}: tipo inválido '{kind}'"));
            continue;
        }

        let category = fields[offset + 2].trim().to_string();
        if !valid_cats.contains(&category) {
            skipped += 1;
            errors.push(format!("Fila {row_num}: categoría '{category}' no existe en presupuestos"));
            continue;
        }

        let amount: i64 = match fields[offset + 3].trim().parse() {
            Ok(a) if a > 0 => a,
            _ => {
                skipped += 1;
                errors.push(format!("Fila {row_num}: monto inválido '{}'", fields[offset + 3].trim()));
                continue;
            }
        };

        let note: Option<String> = fields.get(offset + 4)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let is_extraordinary: bool = fields.get(offset + 5)
            .map(|s| matches!(s.trim(), "Sí" | "Si" | "1" | "true"))
            .unwrap_or(false);

        match conn.execute(
            "INSERT INTO transactions (date, type, category, amount, note, is_extraordinary, goal_id) \
             VALUES (?, ?, ?, ?, ?, ?, NULL)",
            libsql::params![date, kind, category, amount, note, is_extraordinary as i64],
        ).await {
            Ok(_) => imported += 1,
            Err(e) => {
                skipped += 1;
                errors.push(format!("Fila {row_num}: {e}"));
            }
        }
    }

    if imported > 0 { spawn_sync(Arc::clone(&state.0)); }

    Ok(ImportResult { imported, skipped, errors })
}

#[tauri::command]
pub async fn has_turso_credentials() -> bool {
    crate::credentials::has_credentials()
}

#[tauri::command]
pub async fn set_turso_credentials(url: String, token: String) -> AppResult<()> {
    let path = crate::credentials::credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!("turso_url = \"{url}\"\nturso_auth_token = \"{token}\"\n");
    std::fs::write(&path, content)?;
    Ok(())
}
