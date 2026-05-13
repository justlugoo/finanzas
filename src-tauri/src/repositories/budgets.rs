use crate::error::{AppError, AppResult};
use crate::models::Budget;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<Budget>> {
    let mut rows = conn.query(
        "SELECT category, monthly_amount, route_id, type, is_fixed FROM budgets ORDER BY category",
        (),
    ).await?;

    let mut budgets = Vec::new();
    while let Some(row) = rows.next().await? {
        budgets.push(Budget {
            category: row.get(0)?,
            monthly_amount: row.get(1)?,
            route_id: row.get(2).ok(),
            r#type: row.get(3)?,
            is_fixed: row.get::<i64>(4).unwrap_or(0) != 0,
        });
    }
    Ok(budgets)
}

pub async fn find_by_category(
    conn: &libsql::Connection,
    category: &str,
) -> AppResult<Option<Budget>> {
    let mut rows = conn.query(
        "SELECT category, monthly_amount, route_id, type, is_fixed FROM budgets WHERE category = ?",
        libsql::params![category.to_string()],
    ).await?;

    match rows.next().await? { Some(row) => {
        Ok(Some(Budget {
            category: row.get(0)?,
            monthly_amount: row.get(1)?,
            route_id: row.get(2).ok(),
            r#type: row.get(3)?,
            is_fixed: row.get::<i64>(4).unwrap_or(0) != 0,
        }))
    } _ => {
        Ok(None)
    }}
}

pub async fn get_monthly_limit(
    conn: &libsql::Connection,
    category: &str,
) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT monthly_amount FROM budgets WHERE category = ?",
        libsql::params![category.to_string()],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0))
}

pub async fn insert(
    conn: &libsql::Connection,
    category: &str,
    monthly_amount: i64,
    kind: &str,
    is_fixed: bool,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO budgets (category, monthly_amount, type, is_fixed) VALUES (?, ?, ?, ?)",
        libsql::params![
            category.to_string(),
            monthly_amount,
            kind.to_string(),
            is_fixed as i64
        ],
    ).await.map_err(|e| {
        if e.to_string().to_lowercase().contains("unique") {
            AppError::ValidationError(format!("la categoría '{category}' ya existe"))
        } else {
            AppError::DatabaseError(e.to_string())
        }
    })?;
    Ok(())
}

pub async fn update_amount(
    conn: &libsql::Connection,
    category: &str,
    monthly_amount: i64,
) -> AppResult<u64> {
    let affected = conn.execute(
        "UPDATE budgets SET monthly_amount = ? WHERE category = ?",
        libsql::params![monthly_amount, category.to_string()],
    ).await?;
    Ok(affected)
}

pub async fn update_route(
    conn: &libsql::Connection,
    category: &str,
    route_id: Option<i64>,
) -> AppResult<u64> {
    let affected = conn.execute(
        "UPDATE budgets SET route_id = ? WHERE category = ?",
        libsql::params![route_id, category.to_string()],
    ).await?;
    Ok(affected)
}

pub async fn update_fixed(
    conn: &libsql::Connection,
    category: &str,
    is_fixed: bool,
) -> AppResult<u64> {
    let affected = conn.execute(
        "UPDATE budgets SET is_fixed = ? WHERE category = ? AND type = 'ingreso'",
        libsql::params![is_fixed as i64, category.to_string()],
    ).await?;
    Ok(affected)
}

pub async fn delete(conn: &libsql::Connection, category: &str) -> AppResult<u64> {
    let affected = conn.execute(
        "DELETE FROM budgets WHERE category = ?",
        libsql::params![category.to_string()],
    ).await?;
    Ok(affected)
}

pub async fn list_categories_by_kind(
    conn: &libsql::Connection,
    kind: &str,
) -> AppResult<Vec<String>> {
    let mut rows = conn.query(
        "SELECT category FROM budgets WHERE type = ? ORDER BY category",
        libsql::params![kind.to_string()],
    ).await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? { cats.push(row.get(0)?); }
    Ok(cats)
}

pub async fn list_all_categories(conn: &libsql::Connection) -> AppResult<Vec<String>> {
    let mut rows = conn.query("SELECT category FROM budgets ORDER BY category", ()).await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? { cats.push(row.get(0)?); }
    Ok(cats)
}

pub async fn list_route_categories(conn: &libsql::Connection) -> AppResult<Vec<String>> {
    let mut rows = conn.query(
        "SELECT category FROM budgets WHERE route_id IS NOT NULL",
        (),
    ).await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? { cats.push(row.get::<String>(0)?); }
    Ok(cats)
}
