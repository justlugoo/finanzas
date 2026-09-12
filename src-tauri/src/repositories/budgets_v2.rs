use crate::error::AppResult;
use crate::models::{BudgetV2, CategoryBudgetRow};
use libsql::Connection;

pub fn row_to_budget(row: &libsql::Row) -> Result<BudgetV2, libsql::Error> {
    Ok(BudgetV2 { category_id: row.get(0)?, monthly_cop: row.get(1)?, updated_at: row.get(2)? })
}

/// UPSERT — crear o actualizar el presupuesto mensual base de una categoría.
pub async fn set_monthly(conn: &Connection, category_id: &str, monthly_cop: i64) -> AppResult<BudgetV2> {
    conn.execute(
        "INSERT INTO budgets (category_id, monthly_cop) VALUES (?, ?) \
         ON CONFLICT(category_id) DO UPDATE SET monthly_cop = excluded.monthly_cop, updated_at = datetime('now')",
        libsql::params![category_id.to_string(), monthly_cop],
    )
    .await?;
    let mut rows = conn
        .query(
            "SELECT category_id, monthly_cop, updated_at FROM budgets WHERE category_id = ?",
            libsql::params![category_id.to_string()],
        )
        .await?;
    let row = rows.next().await?.expect("recién insertado/actualizado");
    row_to_budget(&row).map_err(|e| crate::error::AppError::DatabaseError(e.to_string()))
}

pub async fn get(conn: &Connection, category_id: &str) -> AppResult<Option<BudgetV2>> {
    let mut rows = conn
        .query(
            "SELECT category_id, monthly_cop, updated_at FROM budgets WHERE category_id = ?",
            libsql::params![category_id.to_string()],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row_to_budget(&row).map_err(|e| crate::error::AppError::DatabaseError(e.to_string()))?),
        None => None,
    })
}

/// Presupuesto distinto para un mes puntual (`YYYY-MM`). Si no hay fila,
/// el llamador debe usar `budgets.monthly_cop` — sección 5 de schema-v2.md.
pub async fn set_override(conn: &Connection, category_id: &str, year_month: &str, amount_cop: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO budget_overrides (category_id, year_month, amount_cop) VALUES (?, ?, ?) \
         ON CONFLICT(category_id, year_month) DO UPDATE SET amount_cop = excluded.amount_cop",
        libsql::params![category_id.to_string(), year_month.to_string(), amount_cop],
    )
    .await?;
    Ok(())
}

pub async fn get_override(conn: &Connection, category_id: &str, year_month: &str) -> AppResult<Option<i64>> {
    let mut rows = conn
        .query(
            "SELECT amount_cop FROM budget_overrides WHERE category_id = ? AND year_month = ?",
            libsql::params![category_id.to_string(), year_month.to_string()],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

/// Todas las categorías (no de sistema) con su presupuesto mensual base —
/// para la lista "Presupuestos mensuales" de Configuración.
pub async fn list_with_categories(conn: &Connection) -> AppResult<Vec<CategoryBudgetRow>> {
    let mut rows = conn
        .query(
            "SELECT c.id, c.name, c.kind, c.is_fixed, c.route_id, c.is_system, c.code, \
                    c.archived_at, c.created_at, c.updated_at, COALESCE(b.monthly_cop, 0) \
             FROM categories c LEFT JOIN budgets b ON b.category_id = c.id \
             WHERE c.is_system = 0 AND c.archived_at IS NULL ORDER BY c.name",
            (),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let category = crate::repositories::categories::row_to_category(&row)
            .map_err(|e| crate::error::AppError::DatabaseError(e.to_string()))?;
        let monthly_cop: i64 = row.get(10)?;
        out.push(CategoryBudgetRow { category, monthly_cop });
    }
    Ok(out)
}

/// Presupuesto efectivo de una categoría para un `year_month` dado:
/// override si existe, si no `budgets.monthly_cop`, si no 0.
pub async fn effective_monthly(conn: &Connection, category_id: &str, year_month: Option<&str>) -> AppResult<i64> {
    if let Some(ym) = year_month
        && let Some(amount) = get_override(conn, category_id, ym).await? {
            return Ok(amount);
        }
    Ok(get(conn, category_id).await?.map(|b| b.monthly_cop).unwrap_or(0))
}
