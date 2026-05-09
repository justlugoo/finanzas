use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::{Local, Datelike, NaiveDate, Duration};
use crate::error::{AppError, AppResult};

pub struct DbState(pub Arc<Mutex<Option<libsql::Database>>>);

// ── Tipos compartidos ──────────────────────────────────────────────────────

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

// ── Helpers ────────────────────────────────────────────────────────────────

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
