//! Comandos v2 de presupuestos. **No registrados en `invoke_handler!` todavía.**

use tauri::State;
use crate::error::AppResult;
use crate::models::{BudgetV2, CategoryBudgetRow};
use crate::services::budgets_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn budget_list_with_categories(state: State<'_, DbState>) -> AppResult<Vec<CategoryBudgetRow>> {
    let conn = get_conn(&state).await?;
    svc::list_with_categories(&conn).await
}

#[tauri::command]
pub async fn budget_set_monthly(state: State<'_, DbState>, category_id: String, monthly_cop: i64) -> AppResult<BudgetV2> {
    let conn = get_conn(&state).await?;
    svc::set_monthly(&conn, &category_id, monthly_cop).await
}

#[tauri::command]
pub async fn budget_set_override(
    state: State<'_, DbState>,
    category_id: String,
    year_month: String,
    amount_cop: i64,
) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::set_override(&conn, &category_id, &year_month, amount_cop).await
}
