use crate::error::{AppError, AppResult};
use crate::models::{BudgetV2, CategoryBudgetRow};
use crate::repositories;
use libsql::Connection;

pub async fn list_with_categories(conn: &Connection) -> AppResult<Vec<CategoryBudgetRow>> {
    repositories::budgets_v2::list_with_categories(conn).await
}

pub async fn set_monthly(conn: &Connection, category_id: &str, monthly_cop: i64) -> AppResult<BudgetV2> {
    if monthly_cop < 0 {
        return Err(AppError::ValidationError("el presupuesto no puede ser negativo".into()));
    }
    repositories::budgets_v2::set_monthly(conn, category_id, monthly_cop).await
}

pub async fn set_override(conn: &Connection, category_id: &str, year_month: &str, amount_cop: i64) -> AppResult<()> {
    if amount_cop < 0 {
        return Err(AppError::ValidationError("el presupuesto no puede ser negativo".into()));
    }
    repositories::budgets_v2::set_override(conn, category_id, year_month, amount_cop).await
}
