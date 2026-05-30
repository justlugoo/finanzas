use tauri::State;
use crate::error::AppResult;
use crate::models::{FuelFillup, FuelFillupInput, VehicleFuelStatus};
use crate::services::fillups as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn fillup_create(
    state: State<'_, DbState>,
    input: FuelFillupInput,
) -> AppResult<FuelFillup> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn fillups_list(
    state: State<'_, DbState>,
    vehicle_id: Option<i64>,
) -> AppResult<Vec<FuelFillup>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, vehicle_id).await
}

#[tauri::command]
pub async fn vehicle_fuel_status(
    state: State<'_, DbState>,
    vehicle_id: i64,
) -> AppResult<VehicleFuelStatus> {
    let conn = get_conn(&state).await?;
    svc::vehicle_fuel_status(&conn, vehicle_id).await
}
