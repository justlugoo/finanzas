//! Comandos v2 de movimientos (`entries`). **No registrados en
//! `invoke_handler!` todavía** — ver nota en `commands/meta_payments.rs`.

use tauri::State;
use crate::error::AppResult;
use crate::models::{
    AccountBalances, CategoryProgressV2, CsvExport, Entry, EntryFilter, EntryInput, EntryPage,
    MonthComparisonV2, PeriodSummaryV2, PeriodV2,
};
use crate::services::entries as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn entry_create(state: State<'_, DbState>, input: EntryInput) -> AppResult<Entry> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn entry_list(state: State<'_, DbState>, filter: EntryFilter) -> AppResult<EntryPage> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, filter).await
}

#[tauri::command]
pub async fn entry_get(state: State<'_, DbState>, id: String) -> AppResult<Entry> {
    let conn = get_conn(&state).await?;
    svc::get_by_id(&conn, &id).await
}

#[tauri::command]
pub async fn entry_update(
    state: State<'_, DbState>,
    id: String,
    occurred_on: String,
    amount_cop: i64,
    note: Option<String>,
    is_extraordinary: bool,
) -> AppResult<Entry> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, &id, &occurred_on, amount_cop, note.as_deref(), is_extraordinary).await
}

#[tauri::command]
pub async fn entry_delete(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}

#[tauri::command]
pub async fn entry_delete_bulk(state: State<'_, DbState>, ids: Vec<String>) -> AppResult<i64> {
    let conn = get_conn(&state).await?;
    svc::delete_bulk(&conn, ids).await
}

#[tauri::command]
pub async fn get_account_balances(state: State<'_, DbState>) -> AppResult<AccountBalances> {
    let conn = get_conn(&state).await?;
    svc::account_balances(&conn).await
}

#[tauri::command]
pub async fn get_period_summary_v2(state: State<'_, DbState>, period: PeriodV2) -> AppResult<PeriodSummaryV2> {
    let conn = get_conn(&state).await?;
    svc::period_summary(&conn, &period).await
}

#[tauri::command]
pub async fn get_category_progress_v2(
    state: State<'_, DbState>,
    period: PeriodV2,
) -> AppResult<Vec<CategoryProgressV2>> {
    let conn = get_conn(&state).await?;
    let range = crate::utils::resolve_period_range(&period);
    let year_month = match &period {
        PeriodV2::Month { year, month } => Some(format!("{year:04}-{month:02}")),
        _ => None,
    };
    svc::category_progress(&conn, &range.start, &range.end, year_month.as_deref()).await
}

#[tauri::command]
pub async fn entry_export_csv(state: State<'_, DbState>, filter: EntryFilter) -> AppResult<CsvExport> {
    let conn = get_conn(&state).await?;
    svc::export_csv(&conn, filter).await
}

#[tauri::command]
pub async fn get_month_comparison_v2(state: State<'_, DbState>) -> AppResult<MonthComparisonV2> {
    let conn = get_conn(&state).await?;
    svc::month_comparison(&conn).await
}
