//! Comandos v2 de rutas personalizadas. **No registrados en
//! `invoke_handler!` todavía.**

use tauri::State;
use crate::error::AppResult;
use crate::models::{RouteInputV2, RouteV2};
use crate::services::routes_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn route_list_v2(state: State<'_, DbState>) -> AppResult<Vec<RouteV2>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn route_save_v2(state: State<'_, DbState>, input: RouteInputV2) -> AppResult<RouteV2> {
    let conn = get_conn(&state).await?;
    svc::save(&conn, input).await
}

#[tauri::command]
pub async fn route_delete_v2(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}
