use tauri::State;
use crate::error::AppResult;
use crate::models::{GasPrice, RoutesCost, WeeklyGasPoint};
use crate::services::gas as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn get_current_gas_price(state: State<'_, DbState>) -> AppResult<Option<GasPrice>> {
    let conn = get_conn(&state).await?;
    svc::get_current(&conn).await
}

#[tauri::command]
pub async fn list_gas_prices(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> AppResult<Vec<GasPrice>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, limit).await
}

#[tauri::command]
pub async fn register_gas_price_manual(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    price: i64,
) -> AppResult<GasPrice> {
    let conn = get_conn(&state).await?;
    svc::register_manual(&conn, &app, price).await
}

#[tauri::command]
pub async fn get_weekly_gas_comparison(
    state: State<'_, DbState>,
) -> AppResult<Vec<WeeklyGasPoint>> {
    let conn = get_conn(&state).await?;
    svc::get_weekly_comparison(&conn).await
}

#[tauri::command]
pub async fn get_route_costs(state: State<'_, DbState>) -> AppResult<RoutesCost> {
    let conn = get_conn(&state).await?;
    svc::get_route_costs(&conn).await
}
