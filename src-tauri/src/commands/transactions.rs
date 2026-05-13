use tauri::State;
use crate::error::AppResult;
use crate::models::{
    CategoryProgress, CsvExport, CurrentBalance, ImportResult, MonthComparison,
    Period, PeriodSummary, Transaction, TransactionFilter, TransactionInput, TransactionPage,
};
use crate::services::transactions as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn create_transaction(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    input: TransactionInput,
) -> AppResult<Transaction> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, &app, input).await
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, DbState>,
    filter: TransactionFilter,
) -> AppResult<TransactionPage> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, filter).await
}

#[tauri::command]
pub async fn get_current_balance(state: State<'_, DbState>) -> AppResult<CurrentBalance> {
    let conn = get_conn(&state).await?;
    svc::get_balance(&conn).await
}

#[tauri::command]
pub async fn update_transaction(
    state: State<'_, DbState>,
    id: i64,
    input: TransactionInput,
) -> AppResult<Transaction> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, id, input).await
}

#[tauri::command]
pub async fn delete_transaction(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, id).await
}

#[tauri::command]
pub async fn get_period_summary(
    state: State<'_, DbState>,
    period: Period,
) -> AppResult<PeriodSummary> {
    let conn = get_conn(&state).await?;
    svc::get_period_summary(&conn, period).await
}

#[tauri::command]
pub async fn get_category_progress(
    state: State<'_, DbState>,
    period: Period,
) -> AppResult<Vec<CategoryProgress>> {
    let conn = get_conn(&state).await?;
    svc::get_category_progress(&conn, period).await
}

#[tauri::command]
pub async fn get_month_comparison(state: State<'_, DbState>) -> AppResult<MonthComparison> {
    let conn = get_conn(&state).await?;
    svc::get_month_comparison(&conn).await
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, DbState>,
    kind: Option<String>,
) -> AppResult<Vec<String>> {
    let conn = get_conn(&state).await?;
    svc::list_categories(&conn, kind).await
}

#[tauri::command]
pub async fn export_transactions_csv(
    state: State<'_, DbState>,
    filter: TransactionFilter,
) -> AppResult<CsvExport> {
    let conn = get_conn(&state).await?;
    svc::export_csv(&conn, filter).await
}

#[tauri::command]
pub async fn import_transactions_csv(
    state: State<'_, DbState>,
    csv_content: String,
) -> AppResult<ImportResult> {
    let conn = get_conn(&state).await?;
    svc::import_csv(&conn, csv_content).await
}

#[tauri::command]
pub async fn delete_transactions_bulk(
    state: State<'_, DbState>,
    ids: Vec<i64>,
) -> AppResult<i64> {
    let conn = get_conn(&state).await?;
    svc::delete_bulk(&conn, ids).await
}
