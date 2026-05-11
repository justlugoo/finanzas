use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::{Local, Datelike, NaiveDate, Duration};
use crate::error::{AppError, AppResult};

pub struct DbState {
    pub db:   Arc<RwLock<Option<libsql::Database>>>,
    /// None mientras el sync está en curso; se recrea en el primer get_conn.
    pub conn: Arc<tokio::sync::Mutex<Option<libsql::Connection>>>,
}

/// Guard que mantiene el mutex de la conexión bloqueado y derefa a Connection.
pub struct ConnGuard(tokio::sync::OwnedMutexGuard<Option<libsql::Connection>>);

impl std::ops::Deref for ConnGuard {
    type Target = libsql::Connection;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("conn inicializado en get_conn")
    }
}

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
    pub is_debt: bool,
}

#[derive(Serialize, Debug)]
pub struct CurrentBalance {
    pub total_income: i64,
    pub total_expenses: i64,
    pub balance: i64,
}

#[derive(Serialize, Debug)]
pub struct TransactionPage {
    pub transactions: Vec<Transaction>,
    pub total_count: i64,
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
    #[serde(default)]
    pub is_debt: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct TransactionFilter {
    pub period: Option<Period>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub search_note: Option<String>,
    pub only_extraordinary: Option<bool>,
    pub only_debt: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
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
    pub is_debt_goal: bool,
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

fn format_cop_simple(n: i64) -> String {
    let s = n.abs().to_string();
    let mut chars: Vec<char> = Vec::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { chars.push('.'); }
        chars.push(c);
    }
    format!("${}", chars.into_iter().rev().collect::<String>())
}

fn send_notification(app: &tauri::AppHandle, title: &str, body: &str) {
    // En Linux, GNOME silencia las notificaciones D-Bus de apps sin .desktop
    // registrado (modo dev). notify-send las muestra siempre.
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        let _ = std::process::Command::new("notify-send")
            .arg("--app-name=Finanzas")
            .arg(title)
            .arg(body)
            .spawn();
    }
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_notification::NotificationExt;
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            eprintln!("[finanzas] notification error: {e}");
        }
    }
}

fn scale_monthly(monthly: i64, period: &Period) -> i64 {
    if monthly == 0 { return 0; }
    let today = Local::now().date_naive();
    let dim = days_in_month(today.year(), today.month()) as f64;
    match period {
        Period::Daily   => (monthly as f64 / dim).round() as i64,
        Period::Weekly  => (monthly as f64 * 7.0 / dim).round() as i64,
        Period::Monthly => monthly,
        Period::Yearly  => monthly * 12,
        Period::Custom { start, end } => {
            let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap_or(today);
            let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").unwrap_or(today);
            let days = ((e - s).num_days() + 1).max(1) as f64;
            (monthly as f64 * days / 30.0).round() as i64
        }
    }
}

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

    let precio: i64 = {
        let mut price_rows = conn
            .query(
                "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
                (),
            )
            .await?;
        price_rows
            .next()
            .await?
            .map(|r| r.get(0).unwrap_or(15881))
            .unwrap_or(15881)
        // price_rows drops here — cursor cerrado antes del INSERT
    };

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

async fn get_conn(state: &State<'_, DbState>) -> AppResult<ConnGuard> {
    // Espera la DB (inicio en frío). Timeout = 3 s.
    for _ in 0..20u8 {
        if state.db.read().await.is_some() { break; }
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    let mut guard = state.conn.clone().lock_owned().await;
    if guard.is_none() {
        let db_guard = state.db.read().await;
        let db = db_guard.as_ref()
            .ok_or_else(|| AppError::DatabaseError("base de datos no inicializada".into()))?;
        let conn = db.connect().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        crate::db::apply_pragmas(&conn).await?;
        *guard = Some(conn);
    }
    Ok(ConnGuard(guard))
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
        is_debt: row.get::<i64>(9).unwrap_or(0) != 0,
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
    app: tauri::AppHandle,
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

    // INSERT + RETURNING: obtiene la fila recién insertada sin un SELECT separado.
    // El cursor vive en su bloque y se cierra antes de que autogas u otros
    // cursores se abran.
    let tx = {
        let insert_q = conn.query(
            "INSERT INTO transactions \
             (date, type, category, amount, note, is_extraordinary, goal_id, is_debt) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             RETURNING id, date, type, category, amount, note, \
                       is_extraordinary, goal_id, created_at, is_debt",
            libsql::params![
                input.date.clone(),
                input.kind.clone(),
                input.category.clone(),
                input.amount,
                input.note.clone(),
                input.is_extraordinary as i64,
                input.goal_id,
                input.is_debt as i64
            ],
        ).await;

        let mut rows = match insert_q {
            Ok(r) => r,
            Err(e) => {
                if auto_gas { let _ = conn.execute("ROLLBACK", ()).await; }
                return Err(AppError::DatabaseError(e.to_string()));
            }
        };
        let row = match rows.next().await {
            Ok(Some(r)) => r,
            Ok(None) => {
                if auto_gas { let _ = conn.execute("ROLLBACK", ()).await; }
                return Err(AppError::DatabaseError("RETURNING vacío tras INSERT".into()));
            }
            Err(e) => {
                if auto_gas { let _ = conn.execute("ROLLBACK", ()).await; }
                return Err(AppError::DatabaseError(e.to_string()));
            }
        };
        row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?
        // rows drops aquí — cursor cerrado antes de autogas
    };
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

    // ── Auto-crear objetivo de deuda ──────────────────────────────────────
    if input.is_debt && input.kind == "gasto" {
        let debt_name = match &input.note {
            Some(n) if !n.trim().is_empty() => format!("Deuda: {}", n.trim()),
            _ => format!("Deuda: {}", input.category),
        };
        let _ = conn.execute(
            "INSERT INTO goals (name, target_amount, is_debt_goal) VALUES (?, ?, 1)",
            libsql::params![debt_name, input.amount],
        ).await;
    }

    // ── Trigger 1: presupuesto excedido — cursores secuenciales, no anidados ─
    if input.kind == "gasto" {
        let today = Local::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        let today_str = today.format("%Y-%m-%d").to_string();

        let spent: i64 = {
            match conn.query(
                "SELECT COALESCE(SUM(amount), 0) FROM transactions \
                 WHERE category = ? AND type = 'gasto' AND date >= ? AND date <= ?",
                libsql::params![input.category.clone(), month_start, today_str],
            ).await {
                Ok(mut rows) => rows.next().await.ok().flatten()
                    .and_then(|r| r.get(0).ok()).unwrap_or(0),
                Err(_) => 0,
            }
            // rows drops aquí
        };

        let limit: i64 = {
            match conn.query(
                "SELECT monthly_amount FROM budgets WHERE category = ?",
                libsql::params![input.category.clone()],
            ).await {
                Ok(mut rows) => rows.next().await.ok().flatten()
                    .and_then(|r| r.get(0).ok()).unwrap_or(0),
                Err(_) => 0,
            }
            // rows drops aquí
        };

        if limit > 0 && spent > limit {
            send_notification(
                &app,
                "⚠ Presupuesto excedido",
                &format!(
                    "Llevas {} en {} — límite mensual: {}",
                    format_cop_simple(spent),
                    input.category,
                    format_cop_simple(limit),
                ),
            );
        }
    }

    // ── Trigger 2: objetivo completado — cursores secuenciales, no anidados ─
    if let Some(gid) = input.goal_id {
        let goal_info: Option<(String, i64)> = {
            match conn.query(
                "SELECT name, target_amount FROM goals WHERE id = ? AND status != 'completado'",
                libsql::params![gid],
            ).await {
                Ok(mut rows) => rows.next().await.ok().flatten().map(|r| {
                    let name: String = r.get(0).unwrap_or_default();
                    let target: i64 = r.get(1).unwrap_or(0);
                    (name, target)
                }),
                Err(_) => None,
            }
            // rows drops aquí
        };

        if let Some((goal_name, target)) = goal_info {
            if target > 0 {
                let current: i64 = {
                    match conn.query(
                        "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE goal_id = ?",
                        libsql::params![gid],
                    ).await {
                        Ok(mut rows) => rows.next().await.ok().flatten()
                            .and_then(|r| r.get(0).ok()).unwrap_or(0),
                        Err(_) => 0,
                    }
                    // rows drops aquí
                };

                if current >= target {
                    let _ = conn.execute(
                        "UPDATE goals SET status = 'completado' WHERE id = ?",
                        libsql::params![gid],
                    ).await;
                    send_notification(
                        &app,
                        "🎯 Objetivo completado",
                        &format!("¡Alcanzaste tu meta: {}!", goal_name),
                    );
                }
            }
        }
    }

    Ok(tx)
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, DbState>,
    filter: TransactionFilter,
) -> AppResult<TransactionPage> {
    let conn = get_conn(&state).await?;

    let mut where_sql = " WHERE 1=1".to_string();
    let mut base_params: Vec<libsql::Value> = Vec::new();

    if let Some(period) = &filter.period {
        let (start, end) = period_to_dates(period);
        where_sql.push_str(" AND date >= ? AND date <= ?");
        base_params.push(start.into());
        base_params.push(end.into());
    }
    if let Some(kind) = &filter.kind {
        where_sql.push_str(" AND type = ?");
        base_params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category {
        if cat == "Carrera" {
            where_sql.push_str(" AND (category = 'Carrera mamá' OR category = 'Carrera cuñada')");
        } else {
            where_sql.push_str(" AND category = ?");
            base_params.push(cat.clone().into());
        }
    }
    if filter.only_extraordinary == Some(true) {
        where_sql.push_str(" AND is_extraordinary = 1");
    }
    if filter.only_debt == Some(true) {
        where_sql.push_str(" AND is_debt = 1");
    }
    if let Some(note) = &filter.search_note {
        where_sql.push_str(" AND note LIKE ?");
        base_params.push(format!("%{note}%").into());
    }

    let total_count: i64 = {
        let count_sql = format!("SELECT COUNT(*) FROM transactions{where_sql}");
        let mut rows = conn.query(&count_sql, base_params.clone()).await?;
        let row = rows.next().await?.ok_or_else(|| AppError::DatabaseError("count query failed".into()))?;
        row.get(0)?
    };

    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(200).max(1);
    let offset    = (page - 1) * page_size;

    let select_sql = format!(
        "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
         FROM transactions{where_sql} ORDER BY date DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut params = base_params;
    params.push(page_size.into());
    params.push(offset.into());

    let mut rows = conn.query(&select_sql, params).await?;
    let mut transactions = Vec::new();
    while let Some(row) = rows.next().await? {
        transactions.push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }

    Ok(TransactionPage { transactions, total_count })
}

#[tauri::command]
pub async fn get_current_balance(state: State<'_, DbState>) -> AppResult<CurrentBalance> {
    let conn = get_conn(&state).await?;
    let mut rows = conn
        .query(
            "SELECT
                COALESCE(SUM(CASE WHEN type='ingreso' THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN type='gasto'   THEN amount ELSE 0 END), 0)
             FROM transactions",
            (),
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("balance sin resultados".into()))?;

    let total_income: i64 = row.get(0)?;
    let total_expenses: i64 = row.get(1)?;

    Ok(CurrentBalance {
        total_income,
        total_expenses,
        balance: total_income - total_expenses,
    })
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

    let tx = {
        let mut rows = conn.query(
            "UPDATE transactions \
             SET date=?, type=?, category=?, amount=?, note=?, is_extraordinary=?, goal_id=?, is_debt=? \
             WHERE id=? \
             RETURNING id, date, type, category, amount, note, \
                       is_extraordinary, goal_id, created_at, is_debt",
            libsql::params![
                input.date.clone(),
                input.kind.clone(),
                input.category.clone(),
                input.amount,
                input.note.clone(),
                input.is_extraordinary as i64,
                input.goal_id,
                input.is_debt as i64,
                id
            ],
        ).await?;
        let row = rows.next().await?
            .ok_or_else(|| AppError::NotFound(format!("transacción {id} no existe")))?;
        row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?
    };

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
             ORDER BY b.category",
            libsql::params![start.clone(), end.clone(), start, end],
        )
        .await?;

    let mut progress = Vec::new();
    while let Some(row) = rows.next().await? {
        let category: String = row.get(0)?;
        let monthly_raw: i64 = row.get(1)?;
        let current_amount: i64 = row.get(2)?;
        let kind: String = row.get(3)?;
        let monthly_target = scale_monthly(monthly_raw, &period);

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
            drop(rows);
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
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
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
            is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
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

    let mut sql = "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
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
        if cat == "Carrera" {
            sql.push_str(" AND (category = 'Carrera mamá' OR category = 'Carrera cuñada')");
        } else {
            sql.push_str(" AND category = ?");
            params.push(cat.clone().into());
        }
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
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
             FROM goals WHERE status = ? ORDER BY name",
            libsql::params![s.clone()],
        )
        .await?
    } else {
        conn.query(
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
             FROM goals ORDER BY name",
            (),
        )
        .await?
    };

    // Drain the cursor fully before opening any nested queries on the same
    // connection — libsql doesn't support concurrent active statements.
    let mut goals: Vec<Goal> = Vec::new();
    while let Some(row) = rows.next().await? {
        goals.push(Goal {
            id: row.get(0)?,
            name: row.get(1)?,
            target_amount: row.get(2)?,
            target_date: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
            is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
        });
    }
    drop(rows);

    let mut result = Vec::new();
    for goal in goals {
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
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal FROM goals WHERE id = ?",
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
        is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
    };

    let result = build_goal_progress(&conn, goal).await?;
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
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal FROM goals WHERE id = ?",
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
        is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
    };

    let result = build_goal_progress(&conn, goal).await?;
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

    Ok(())
}

#[tauri::command]
pub async fn get_goal_detail(state: State<'_, DbState>, id: i64) -> AppResult<GoalDetail> {
    let conn = get_conn(&state).await?;

    let mut rows = conn
        .query(
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal FROM goals WHERE id = ?",
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
        is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
    };

    let goal_with_progress = build_goal_progress(&conn, goal).await?;

    let mut tx_rows = conn
        .query(
            "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
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
    app: tauri::AppHandle,
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

    // Leer precio anterior antes de insertar
    let prev_price: Option<i64> = {
        let mut rows = conn
            .query("SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1", ())
            .await?;
        rows.next().await?.and_then(|r| r.get(0).ok())
    };

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

    // ── Trigger 3: cambio significativo de precio ─────────────────────────
    if let Some(anterior) = prev_price {
        if anterior > 0 {
            let delta = (price - anterior).abs() as f64 / anterior as f64;
            if delta > 0.05 {
                let direccion = if price > anterior { "subió" } else { "bajó" };
                let delta_pct = (delta * 100.0).round() as i64;
                send_notification(
                    &app,
                    "⛽ Precio de gasolina",
                    &format!(
                        "El precio {} {}% → {}/gal",
                        direccion, delta_pct, format_cop_simple(price),
                    ),
                );
            }
        }
    }

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

    Ok(Budget { category, monthly_amount })
}

// ── Autostart ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> bool {
    // On Linux the plugin's is_enabled() checks whether the *current* executable
    // is registered, so it returns false when running the debug binary even though
    // the release binary is correctly registered.  Check the .desktop file directly.
    #[cfg(target_os = "linux")]
    {
        if let Some(path) = dirs::home_dir().map(|h| h.join(".config/autostart/Finanzas.desktop")) {
            return path.exists();
        }
    }
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub async fn set_autostart_enabled(_app: tauri::AppHandle, enabled: bool) -> AppResult<()> {
    // On Linux manage the .desktop file directly so we can always write the
    // release binary path, regardless of whether we are currently running as
    // debug or release.
    #[cfg(target_os = "linux")]
    {
        let desktop = dirs::home_dir()
            .map(|h| h.join(".config/autostart/Finanzas.desktop"))
            .ok_or_else(|| AppError::IoError("No se pudo determinar el directorio home".into()))?;

        if !enabled {
            let _ = std::fs::remove_file(&desktop);
            return Ok(());
        }

        // Derive release binary from current exe: target/{debug|release}/finanzas
        // → target/release/finanzas
        let exe = std::env::current_exe().map_err(|e| AppError::IoError(e.to_string()))?;
        let release_bin = exe
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("release/finanzas"))
            .unwrap_or_else(|| exe.clone());

        if !release_bin.exists() {
            return Err(AppError::DatabaseError(
                "No se encontró el binario release. Ejecuta `cargo build --release` una vez para generarlo.".to_string(),
            ));
        }

        let content = format!(
            "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Finanzas\nComment=Finanzas startup script\nExec={} --autostart\nStartupNotify=false\nTerminal=false\n",
            release_bin.display()
        );
        std::fs::write(&desktop, &content).map_err(|e| AppError::IoError(e.to_string()))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let al = _app.autolaunch();
        if enabled {
            al.enable().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        } else {
            al.disable().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }
}

// ── Backup ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn backup_database() -> AppResult<String> {
    let src = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("finanzas")
        .join("local.db");
    if !src.exists() {
        return Err(AppError::NotFound(
            "archivo de base de datos local no encontrado".into(),
        ));
    }

    let today = Local::now().format("%Y-%m-%d").to_string();
    let dest_dir = dirs::document_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Documents"));
    let dest = dest_dir.join(format!("finanzas_backup_{today}.db"));

    std::fs::copy(&src, &dest)?;

    Ok(dest.to_string_lossy().to_string())
}

// ── Factory reset ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn factory_reset(state: State<'_, DbState>) -> AppResult<()> {
    {
        let conn = get_conn(&state).await?;
        conn.execute("DELETE FROM transactions", libsql::params![]).await?;
        conn.execute("DELETE FROM goals",        libsql::params![]).await?;
        conn.execute("DELETE FROM gas_prices",   libsql::params![]).await?;
        conn.execute(
            "INSERT INTO gas_prices (date, price_per_gallon, source) VALUES (date('now'), 15881, 'manual')",
            libsql::params![],
        ).await?;
    }
    Ok(())
}

// ── Bulk delete transactions ────────────────────────────────────────────────

#[tauri::command]
pub async fn delete_transactions_bulk(state: State<'_, DbState>, ids: Vec<i64>) -> AppResult<i64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM transactions WHERE id IN ({placeholders})");
    let params: Vec<libsql::Value> = ids.iter().map(|&id| libsql::Value::Integer(id)).collect();

    let affected = {
        let conn = get_conn(&state).await?;
        conn.execute(&sql, params).await?
    };
    Ok(affected as i64)
}
