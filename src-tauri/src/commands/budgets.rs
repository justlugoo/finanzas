use tauri::State;
use crate::error::AppResult;
use crate::models::Budget;
use crate::services::budgets as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn list_budgets(state: State<'_, DbState>) -> AppResult<Vec<Budget>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn create_budget(
    state: State<'_, DbState>,
    category: String,
    monthly_amount: i64,
    kind: String,
    is_fixed: Option<bool>,
) -> AppResult<Budget> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, category, monthly_amount, kind, is_fixed).await
}

#[tauri::command]
pub async fn update_budget(
    state: State<'_, DbState>,
    category: String,
    monthly_amount: i64,
) -> AppResult<Budget> {
    let conn = get_conn(&state).await?;
    svc::update_amount(&conn, category, monthly_amount).await
}

#[tauri::command]
pub async fn update_budget_route(
    state: State<'_, DbState>,
    category: String,
    route_id: Option<i64>,
) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::update_route(&conn, category, route_id).await
}

#[tauri::command]
pub async fn update_budget_fixed(
    state: State<'_, DbState>,
    category: String,
    is_fixed: bool,
) -> AppResult<Budget> {
    let conn = get_conn(&state).await?;
    svc::update_fixed(&conn, category, is_fixed).await
}

#[tauri::command]
pub async fn delete_budget(state: State<'_, DbState>, category: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, category).await
}
