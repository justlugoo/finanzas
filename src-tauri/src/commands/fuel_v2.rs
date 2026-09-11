//! Comandos de combustible sobre el esquema v2. **No registrados todavía**
//! en `lib.rs` — operan sobre tablas (`vehicles`, `fuel_adjustments`) que no
//! existen en la base real hasta correr la migración 001.

use tauri::State;
use crate::error::AppResult;
use crate::models::{
    FillupInputV2, FillupResult, FillupV2, FillupWithExpenseInput, FillupWithExpenseResult,
    FuelAdjustment, FuelLevelResetInput, TankLevel, Trip, TripInputV2,
};
use crate::services::fuel as svc;
use crate::state::{DbState, get_conn};

/// Resetear el nivel de tanque a mano, cuando el usuario quiera — no borra
/// tanqueos ni viajes anteriores, solo ancla el conteo desde ahora.
#[tauri::command]
pub async fn vehicle_reset_fuel_level(
    state: State<'_, DbState>,
    input: FuelLevelResetInput,
) -> AppResult<FuelAdjustment> {
    let conn = get_conn(&state).await?;
    svc::reset_level(&conn, &input.vehicle_id, input.level_ml, &input.occurred_on, input.note.as_deref()).await
}

#[tauri::command]
pub async fn fillup_create_v2(state: State<'_, DbState>, input: FillupInputV2) -> AppResult<FillupResult> {
    let conn = get_conn(&state).await?;
    let (fillup, warning) = svc::create_fillup(
        &conn,
        &input.vehicle_id,
        &input.occurred_on,
        input.total_cop,
        input.price_cop_per_gallon,
        input.entry_id.as_deref(),
        input.note.as_deref(),
    )
    .await?;
    Ok(FillupResult { fillup, warning })
}

#[tauri::command]
pub async fn fillup_create_with_expense_v2(
    state: State<'_, DbState>,
    input: FillupWithExpenseInput,
) -> AppResult<FillupWithExpenseResult> {
    let conn = get_conn(&state).await?;
    let (entry, fillup, warning) = svc::create_fillup_with_expense(
        &conn,
        &input.vehicle_id,
        &input.occurred_on,
        input.total_cop,
        input.price_cop_per_gallon,
        &input.category_id,
        input.note.as_deref(),
    )
    .await?;
    Ok(FillupWithExpenseResult { entry, fillup, warning })
}

#[tauri::command]
pub async fn fillups_list_v2(state: State<'_, DbState>, vehicle_id: Option<String>) -> AppResult<Vec<FillupV2>> {
    let conn = get_conn(&state).await?;
    svc::list_fillups(&conn, vehicle_id.as_deref()).await
}

#[tauri::command]
pub async fn vehicle_fuel_status_v2(state: State<'_, DbState>, vehicle_id: String) -> AppResult<TankLevel> {
    let conn = get_conn(&state).await?;
    svc::tank_level(&conn, &vehicle_id).await
}

#[tauri::command]
pub async fn trip_register_v2(state: State<'_, DbState>, input: TripInputV2) -> AppResult<Trip> {
    let conn = get_conn(&state).await?;
    svc::register_trip(&conn, &input.vehicle_id, &input.occurred_on, input.distance_m, input.entry_id.as_deref()).await
}
