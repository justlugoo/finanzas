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
    #[serde(default)]
    pub gas_km: Option<f64>,
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

#[derive(Deserialize, Debug)]
pub struct GoalInput {
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GoalWithProgress {
    pub goal: Goal,
    pub current_amount: i64,
    pub percentage: f64,
    pub monthly_required: Option<f64>,
    pub projected_completion_date: Option<String>,
    pub on_track: bool,
}

#[derive(Serialize, Debug)]
pub struct GoalDetail {
    pub goal: GoalWithProgress,
    pub contributions: Vec<Transaction>,
}

#[derive(Serialize, Debug)]
pub struct GasPrice {
    pub id: i64,
    pub date: String,
    pub price_per_gallon: i64,
    pub source: String,
}

#[derive(Serialize, Debug)]
pub struct WeeklyGasPoint {
    pub week_start: String,
    pub avg_price: f64,
    pub entry_count: i64,
}

#[derive(Serialize, Debug)]
pub struct TripCostResult {
    pub km: f64,
    pub cost: f64,
    pub price_per_gallon: i64,
    pub consumo_km_galon: f64,
}

#[derive(Serialize, Debug)]
pub struct RoutesCost {
    pub precio_galon: i64,
    pub carrera_mama: i64,
    pub carrera_cunada: i64,
    pub universidad: i64,
    pub consumo_km_galon: f64,
    pub km_universidad: f64,
    pub km_carrera_mama: f64,
    pub km_carrera_cunada: f64,
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

async fn read_config_f64(conn: &libsql::Connection, key: &str, default: f64) -> f64 {
    let Ok(mut rows) = conn
        .query("SELECT value FROM config WHERE key = ?", libsql::params![key])
        .await
    else {
        return default;
    };
    rows.next()
        .await
        .ok()
        .flatten()
        .and_then(|r| r.get::<String>(0).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

async fn insert_auto_gas(
    conn: &libsql::Connection,
    date: &str,
    context: &str,
    km: f64,
) -> AppResult<()> {
    let consumo = read_config_f64(conn, "consumo_moto_km_galon", 350.0).await;

    let mut price_rows = conn
        .query(
            "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
            (),
        )
        .await?;
    let precio: i64 = price_rows
        .next()
        .await?
        .map(|r| r.get(0).unwrap_or(15881))
        .unwrap_or(15881);

    let gas_cost = ((km / consumo) * precio as f64).round() as i64;
    let gas_note = format!("Auto: Gasolina {} ({:.1}km)", context, km);

    conn.execute(
        "INSERT INTO transactions (date, type, category, amount, note, is_extraordinary, goal_id) \
         VALUES (?, 'gasto', 'Gasolina', ?, ?, 0, NULL)",
        libsql::params![date.to_string(), gas_cost, gas_note],
    )
    .await?;

    Ok(())
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
    let mut last_err = String::new();
    for attempt in 0..3u8 {
        if attempt > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        let guard = state.0.lock().await;
        let db = guard
            .as_ref()
            .ok_or_else(|| AppError::DatabaseError("base de datos no inicializada".into()))?;
        match db.connect() {
            Ok(conn) => return Ok(conn),
            Err(e) => { last_err = e.to_string(); }
        }
    }
    Err(AppError::DatabaseError(last_err))
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

async fn build_goal_progress(conn: &libsql::Connection, goal: Goal) -> AppResult<GoalWithProgress> {
    let mut rows = conn
        .query(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE goal_id = ?",
            libsql::params![goal.id],
        )
        .await?;
    let current_amount: i64 = rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    let percentage = if goal.target_amount > 0 {
        (current_amount as f64 / goal.target_amount as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let mut avg_rows = conn
        .query(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions \
             WHERE goal_id = ? AND date >= date('now', '-3 months')",
            libsql::params![goal.id],
        )
        .await?;
    let sum_3m: i64 = avg_rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);
    let avg_monthly = sum_3m as f64 / 3.0;

    let today = Local::now().date_naive();

    let monthly_required: Option<f64> = if current_amount < goal.target_amount {
        goal.target_date.as_deref().and_then(|td| {
            NaiveDate::parse_from_str(td, "%Y-%m-%d").ok().and_then(|target_date| {
                let months = (target_date.year() - today.year()) * 12
                    + (target_date.month() as i32 - today.month() as i32);
                if months > 0 {
                    Some((goal.target_amount - current_amount) as f64 / months as f64)
                } else {
                    None
                }
            })
        })
    } else {
        None
    };

    let projected_completion_date: Option<String> = if current_amount >= goal.target_amount {
        None
    } else if avg_monthly > 0.0 {
        let remaining = (goal.target_amount - current_amount) as f64;
        let months_needed = (remaining / avg_monthly).ceil() as i32;
        let raw_month = today.month() as i32 + months_needed;
        let years_add = (raw_month - 1) / 12;
        let final_month = ((raw_month - 1) % 12 + 1) as u32;
        let final_year = today.year() + years_add;
        NaiveDate::from_ymd_opt(final_year, final_month, 1)
            .map(|d| d.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    let on_track = match (&goal.target_date, &projected_completion_date) {
        (Some(td), Some(pcd)) => pcd <= td,
        _ => true,
    };

    Ok(GoalWithProgress {
        goal,
        current_amount,
        percentage,
        monthly_required,
        projected_completion_date,
        on_track,
    })
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

    let is_carrera = input.kind == "ingreso"
        && matches!(input.category.as_str(), "Carrera mamá" | "Carrera cuñada");
    let has_gas_km = !is_carrera && input.gas_km.map(|km| km > 0.0).unwrap_or(false);
    let auto_gas = is_carrera || has_gas_km;

    let conn = get_conn(&state).await?;

    // Determine km for gas insertion (read from config for carreras, from input for gastos)
    let gas_km_val: f64 = if is_carrera {
        let km_key = if input.category == "Carrera mamá" {
            "km_carrera_mama_redondo"
        } else {
            "km_carrera_cunada_redondo"
        };
        let km_default = if input.category == "Carrera mamá" { 8.0f64 } else { 16.0f64 };
        read_config_f64(&conn, km_key, km_default).await
    } else {
        input.gas_km.unwrap_or(0.0)
    };

    if auto_gas {
        conn.execute("BEGIN", ()).await?;
    }

    let main_insert = conn
        .execute(
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
        .await;

    if let Err(e) = main_insert {
        if auto_gas { let _ = conn.execute("ROLLBACK", ()).await; }
        return Err(AppError::DatabaseError(e.to_string()));
    }

    let id = conn.last_insert_rowid();

    if auto_gas {
        if let Err(e) = insert_auto_gas(&conn, &input.date, &input.category, gas_km_val).await {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(e);
        }
        if let Err(e) = conn.execute("COMMIT", ()).await {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(AppError::DatabaseError(e.to_string()));
        }
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

    if affected > 0 {
        spawn_sync(Arc::clone(&state.0));
    }
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
    "Carrera", "Carrera cuñada", "Carrera mamá", "Eventual", "Mesada", "Otro ingreso",
];
const EXPENSE_DEFAULTS: &[&str] = &[
    "Imprevisto", "Mantenimiento", "Otro gasto", "Parqueadero", "Social/Salidas",
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

            if k == "gasto" {
                cats.remove("Carrera mamá");
                cats.remove("Carrera cuñada");
            }

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

// ── Goal commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_goals(
    state: State<'_, DbState>,
    status: Option<String>,
) -> AppResult<Vec<GoalWithProgress>> {
    let conn = get_conn(&state).await?;

    let mut rows = if let Some(ref s) = status {
        conn.query(
            "SELECT id, name, target_amount, target_date, status, created_at \
             FROM goals WHERE status = ? ORDER BY name",
            libsql::params![s.clone()],
        )
        .await?
    } else {
        conn.query(
            "SELECT id, name, target_amount, target_date, status, created_at \
             FROM goals ORDER BY name",
            (),
        )
        .await?
    };

    let mut result = Vec::new();
    while let Some(row) = rows.next().await? {
        let goal = Goal {
            id: row.get(0)?,
            name: row.get(1)?,
            target_amount: row.get(2)?,
            target_date: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
        };
        result.push(build_goal_progress(&conn, goal).await?);
    }
    Ok(result)
}

#[tauri::command]
pub async fn create_goal(
    state: State<'_, DbState>,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_amount <= 0 {
        return Err(AppError::ValidationError("el monto objetivo debe ser mayor que 0".into()));
    }

    let conn = get_conn(&state).await?;
    conn.execute(
        "INSERT INTO goals (name, target_amount, target_date) VALUES (?, ?, ?)",
        libsql::params![input.name.trim().to_string(), input.target_amount, input.target_date],
    )
    .await?;

    let id = conn.last_insert_rowid();
    let mut rows = conn
        .query(
            "SELECT id, name, target_amount, target_date, status, created_at FROM goals WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound("objetivo recién creado no encontrado".into()))?;
    let goal = Goal {
        id: row.get(0)?,
        name: row.get(1)?,
        target_amount: row.get(2)?,
        target_date: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
    };

    let result = build_goal_progress(&conn, goal).await?;
    spawn_sync(Arc::clone(&state.0));
    Ok(result)
}

#[tauri::command]
pub async fn update_goal(
    state: State<'_, DbState>,
    id: i64,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_amount <= 0 {
        return Err(AppError::ValidationError("el monto objetivo debe ser mayor que 0".into()));
    }
    let status = input.status.as_deref().unwrap_or("activo");
    if !matches!(status, "activo" | "completado" | "pausado") {
        return Err(AppError::ValidationError("estado inválido".into()));
    }

    let conn = get_conn(&state).await?;
    let affected = conn
        .execute(
            "UPDATE goals SET name = ?, target_amount = ?, target_date = ?, status = ? WHERE id = ?",
            libsql::params![
                input.name.trim().to_string(),
                input.target_amount,
                input.target_date,
                status.to_string(),
                id
            ],
        )
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound(format!("objetivo {id} no existe")));
    }

    let mut rows = conn
        .query(
            "SELECT id, name, target_amount, target_date, status, created_at FROM goals WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("objetivo {id} no existe")))?;
    let goal = Goal {
        id: row.get(0)?,
        name: row.get(1)?,
        target_amount: row.get(2)?,
        target_date: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
    };

    let result = build_goal_progress(&conn, goal).await?;
    spawn_sync(Arc::clone(&state.0));
    Ok(result)
}

#[tauri::command]
pub async fn delete_goal(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    conn.execute(
        "UPDATE transactions SET goal_id = NULL WHERE goal_id = ?",
        libsql::params![id],
    )
    .await?;

    let affected = conn
        .execute("DELETE FROM goals WHERE id = ?", libsql::params![id])
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound(format!("objetivo {id} no existe")));
    }

    spawn_sync(Arc::clone(&state.0));
    Ok(())
}

#[tauri::command]
pub async fn get_goal_detail(state: State<'_, DbState>, id: i64) -> AppResult<GoalDetail> {
    let conn = get_conn(&state).await?;

    let mut rows = conn
        .query(
            "SELECT id, name, target_amount, target_date, status, created_at FROM goals WHERE id = ?",
            libsql::params![id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("objetivo {id} no existe")))?;
    let goal = Goal {
        id: row.get(0)?,
        name: row.get(1)?,
        target_amount: row.get(2)?,
        target_date: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
    };

    let goal_with_progress = build_goal_progress(&conn, goal).await?;

    let mut tx_rows = conn
        .query(
            "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at \
             FROM transactions WHERE goal_id = ? ORDER BY date DESC, id DESC",
            libsql::params![id],
        )
        .await?;

    let mut contributions = Vec::new();
    while let Some(row) = tx_rows.next().await? {
        contributions
            .push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }

    Ok(GoalDetail { goal: goal_with_progress, contributions })
}

// ── Gas / Config commands ──────────────────────────────────────────────────

#[tauri::command]
pub async fn get_current_gas_price(state: State<'_, DbState>) -> AppResult<Option<GasPrice>> {
    let conn = get_conn(&state).await?;
    let mut rows = conn
        .query(
            "SELECT id, date, price_per_gallon, source FROM gas_prices ORDER BY date DESC LIMIT 1",
            (),
        )
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(Some(GasPrice {
            id: row.get(0)?,
            date: row.get(1)?,
            price_per_gallon: row.get(2)?,
            source: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn list_gas_prices(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> AppResult<Vec<GasPrice>> {
    let conn = get_conn(&state).await?;
    let limit = limit.unwrap_or(30);

    let mut rows = conn
        .query(
            "SELECT id, date, price_per_gallon, source FROM gas_prices ORDER BY date DESC LIMIT ?",
            libsql::params![limit],
        )
        .await?;

    let mut prices = Vec::new();
    while let Some(row) = rows.next().await? {
        prices.push(GasPrice {
            id: row.get(0)?,
            date: row.get(1)?,
            price_per_gallon: row.get(2)?,
            source: row.get(3)?,
        });
    }
    Ok(prices)
}

#[tauri::command]
pub async fn register_gas_price_manual(
    state: State<'_, DbState>,
    price: i64,
) -> AppResult<GasPrice> {
    if price < 1000 || price > 100_000 {
        return Err(AppError::ValidationError(
            "el precio debe estar entre 1.000 y 100.000 COP/galón".into(),
        ));
    }

    let today = Local::now().format("%Y-%m-%d").to_string();
    let conn = get_conn(&state).await?;

    conn.execute(
        "INSERT OR REPLACE INTO gas_prices (date, price_per_gallon, source) VALUES (?, ?, 'manual')",
        libsql::params![today.clone(), price],
    )
    .await?;

    let mut rows = conn
        .query(
            "SELECT id, date, price_per_gallon, source FROM gas_prices WHERE date = ?",
            libsql::params![today],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("precio no encontrado tras insertar".into()))?;

    let result = GasPrice {
        id: row.get(0)?,
        date: row.get(1)?,
        price_per_gallon: row.get(2)?,
        source: row.get(3)?,
    };

    spawn_sync(Arc::clone(&state.0));
    Ok(result)
}

#[tauri::command]
pub async fn get_weekly_gas_comparison(
    state: State<'_, DbState>,
) -> AppResult<Vec<WeeklyGasPoint>> {
    let conn = get_conn(&state).await?;

    let mut rows = conn
        .query(
            "SELECT
                date(date, '-' || ((strftime('%w', date) + 6) % 7) || ' days') AS week_start,
                AVG(price_per_gallon) AS avg_price,
                COUNT(*) AS entry_count
             FROM gas_prices
             GROUP BY week_start
             ORDER BY week_start DESC
             LIMIT 8",
            (),
        )
        .await?;

    let mut points = Vec::new();
    while let Some(row) = rows.next().await? {
        points.push(WeeklyGasPoint {
            week_start: row.get(0)?,
            avg_price: row.get(1)?,
            entry_count: row.get(2)?,
        });
    }
    Ok(points)
}

#[tauri::command]
pub async fn calculate_trip_cost(
    state: State<'_, DbState>,
    km: f64,
) -> AppResult<TripCostResult> {
    if km <= 0.0 {
        return Err(AppError::ValidationError("los kilómetros deben ser mayor que 0".into()));
    }

    let conn = get_conn(&state).await?;

    let mut conf_rows = conn
        .query("SELECT value FROM config WHERE key = 'consumo_moto_km_galon'", ())
        .await?;
    let consumo_km_galon: f64 = conf_rows
        .next()
        .await?
        .and_then(|r| r.get::<String>(0).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(350.0);

    let mut price_rows = conn
        .query(
            "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
            (),
        )
        .await?;
    let price_per_gallon: i64 = price_rows
        .next()
        .await?
        .map(|r| r.get(0).unwrap_or(15881))
        .unwrap_or(15881);

    let cost = km / consumo_km_galon * price_per_gallon as f64;

    Ok(TripCostResult { km, cost, price_per_gallon, consumo_km_galon })
}

#[tauri::command]
pub async fn get_config_value(
    state: State<'_, DbState>,
    key: String,
) -> AppResult<Option<String>> {
    let conn = get_conn(&state).await?;
    let mut rows = conn
        .query("SELECT value FROM config WHERE key = ?", libsql::params![key])
        .await?;
    Ok(rows.next().await?.and_then(|r| r.get(0).ok()))
}

#[tauri::command]
pub async fn get_route_costs(state: State<'_, DbState>) -> AppResult<RoutesCost> {
    let conn = get_conn(&state).await?;

    let mut rows = conn
        .query(
            "SELECT key, value FROM config \
             WHERE key IN ('consumo_moto_km_galon','km_carrera_mama_redondo',\
                           'km_carrera_cunada_redondo','km_universidad_redondo')",
            (),
        )
        .await?;

    let mut consumo = 350.0f64;
    let mut km_mama = 8.0f64;
    let mut km_cunada = 16.0f64;
    let mut km_uni = 11.4f64;

    while let Some(row) = rows.next().await? {
        let key: String = row.get(0)?;
        let val: String = row.get(1)?;
        match key.as_str() {
            "consumo_moto_km_galon"       => consumo   = val.parse().unwrap_or(350.0),
            "km_carrera_mama_redondo"     => km_mama   = val.parse().unwrap_or(8.0),
            "km_carrera_cunada_redondo"   => km_cunada = val.parse().unwrap_or(16.0),
            "km_universidad_redondo"      => km_uni    = val.parse().unwrap_or(11.4),
            _ => {}
        }
    }

    let mut price_rows = conn
        .query(
            "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
            (),
        )
        .await?;
    let precio_galon: i64 = price_rows
        .next()
        .await?
        .map(|r| r.get(0).unwrap_or(15881))
        .unwrap_or(15881);

    let calc = |km: f64| -> i64 { ((km / consumo) * precio_galon as f64).round() as i64 };

    Ok(RoutesCost {
        precio_galon,
        carrera_mama: calc(km_mama),
        carrera_cunada: calc(km_cunada),
        universidad: calc(km_uni),
        consumo_km_galon: consumo,
        km_universidad: km_uni,
        km_carrera_mama: km_mama,
        km_carrera_cunada: km_cunada,
    })
}

#[tauri::command]
pub async fn update_budget(
    state: State<'_, DbState>,
    category: String,
    monthly_amount: i64,
) -> AppResult<Budget> {
    if monthly_amount < 0 {
        return Err(AppError::ValidationError("el monto no puede ser negativo".into()));
    }

    let conn = get_conn(&state).await?;
    let affected = conn
        .execute(
            "UPDATE budgets SET monthly_amount = ? WHERE category = ?",
            libsql::params![monthly_amount, category.clone()],
        )
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría '{category}' no existe en presupuestos")));
    }

    spawn_sync(Arc::clone(&state.0));
    Ok(Budget { category, monthly_amount })
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
