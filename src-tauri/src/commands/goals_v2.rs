//! Comandos v2 de objetivos (`goals`). **No registrados en `invoke_handler!`
//! todavía.** Los abonos van por `meta_add_payment`, no por aquí.

use tauri::State;
use crate::error::AppResult;
use crate::models::{DebtGoalInput, DebtGoalResult, GoalDetailV2, GoalInputV2, GoalWithProgressV2};
use crate::services::goals_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn goal_list_v2(state: State<'_, DbState>, kind: Option<String>) -> AppResult<Vec<GoalWithProgressV2>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, kind.as_deref()).await
}

#[tauri::command]
pub async fn goal_create_v2(state: State<'_, DbState>, input: GoalInputV2) -> AppResult<GoalWithProgressV2> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn goal_update_v2(state: State<'_, DbState>, id: String, input: GoalInputV2) -> AppResult<GoalWithProgressV2> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, &id, input).await
}

#[tauri::command]
pub async fn goal_delete_v2(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}

#[tauri::command]
pub async fn goal_create_debt_v2(state: State<'_, DbState>, input: DebtGoalInput) -> AppResult<DebtGoalResult> {
    let conn = get_conn(&state).await?;
    svc::create_debt(&conn, input).await
}

#[tauri::command]
pub async fn goal_get_detail_v2(state: State<'_, DbState>, id: String) -> AppResult<GoalDetailV2> {
    let conn = get_conn(&state).await?;
    svc::get_detail(&conn, &id).await
}
