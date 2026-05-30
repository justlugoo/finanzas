use tauri::State;
use crate::error::AppResult;
use crate::models::{LoanInput, LoanPaymentInput, LoanUpdateInput, LoanWithBalance};
use crate::services::loans as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn loan_create(
    state: State<'_, DbState>,
    input: LoanInput,
) -> AppResult<LoanWithBalance> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn loan_list(state: State<'_, DbState>) -> AppResult<Vec<LoanWithBalance>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn loan_get(state: State<'_, DbState>, id: i64) -> AppResult<LoanWithBalance> {
    let conn = get_conn(&state).await?;
    svc::get(&conn, id).await
}

#[tauri::command]
pub async fn loan_add_payment(
    state: State<'_, DbState>,
    input: LoanPaymentInput,
) -> AppResult<LoanWithBalance> {
    let conn = get_conn(&state).await?;
    svc::add_payment(&conn, input).await
}

#[tauri::command]
pub async fn loan_update(
    state: State<'_, DbState>,
    id: i64,
    input: LoanUpdateInput,
) -> AppResult<LoanWithBalance> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, id, input).await
}

#[tauri::command]
pub async fn loan_delete(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, id).await
}

#[tauri::command]
pub async fn loans_total_pending(state: State<'_, DbState>) -> AppResult<i64> {
    let conn = get_conn(&state).await?;
    svc::total_pending(&conn).await
}
