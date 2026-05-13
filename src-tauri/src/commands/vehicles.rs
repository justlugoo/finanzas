use tauri::State;
use crate::error::AppResult;
use crate::models::{Vehicle, VehicleInput};
use crate::services::vehicles as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn list_vehicles(state: State<'_, DbState>) -> AppResult<Vec<Vehicle>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn create_vehicle(
    state: State<'_, DbState>,
    input: VehicleInput,
) -> AppResult<Vehicle> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn update_vehicle(
    state: State<'_, DbState>,
    id: i64,
    input: VehicleInput,
) -> AppResult<Vehicle> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, id, input).await
}

#[tauri::command]
pub async fn delete_vehicle(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, id).await
}
