//! Comandos v2 de préstamos (`loans`). **No registrados en `invoke_handler!`
//! todavía.** Los cobros van por `meta_add_payment`, no por aquí.

use tauri::State;
use crate::error::AppResult;
use crate::models::{LoanInputV2, LoanWithBalanceV2};
use crate::services::loans_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn loan_create_v2(state: State<'_, DbState>, input: LoanInputV2) -> AppResult<LoanWithBalanceV2> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn loan_list_v2(state: State<'_, DbState>) -> AppResult<Vec<LoanWithBalanceV2>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn loan_get_v2(state: State<'_, DbState>, id: String) -> AppResult<LoanWithBalanceV2> {
    let conn = get_conn(&state).await?;
    svc::get(&conn, &id).await
}

#[tauri::command]
pub async fn loan_update_v2(
    state: State<'_, DbState>,
    id: String,
    person_name: String,
    principal_cop: i64,
) -> AppResult<LoanWithBalanceV2> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, &id, &person_name, principal_cop).await
}

#[tauri::command]
pub async fn loan_delete_v2(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}

#[tauri::command]
pub async fn loans_total_pending_v2(state: State<'_, DbState>) -> AppResult<i64> {
    let conn = get_conn(&state).await?;
    svc::total_pending(&conn).await
}
