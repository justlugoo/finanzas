//! Comandos v2 de vehículos. **No registrados en `invoke_handler!` todavía.**

use tauri::State;
use crate::error::AppResult;
use crate::models::{VehicleInputV2, VehicleV2};
use crate::services::vehicles_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn vehicle_list_v2(state: State<'_, DbState>) -> AppResult<Vec<VehicleV2>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn vehicle_create_v2(state: State<'_, DbState>, input: VehicleInputV2) -> AppResult<VehicleV2> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn vehicle_update_v2(state: State<'_, DbState>, id: String, input: VehicleInputV2) -> AppResult<VehicleV2> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, &id, input).await
}

#[tauri::command]
pub async fn vehicle_delete_v2(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}
