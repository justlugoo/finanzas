use tauri::State;
use crate::error::AppResult;
use crate::models::{CustomRoute, CustomRouteInput};
use crate::services::routes as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn get_custom_routes(state: State<'_, DbState>) -> AppResult<Vec<CustomRoute>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}

#[tauri::command]
pub async fn save_custom_route(
    state: State<'_, DbState>,
    route: CustomRouteInput,
) -> AppResult<CustomRoute> {
    let conn = get_conn(&state).await?;
    svc::save(&conn, route).await
}

#[tauri::command]
pub async fn delete_custom_route(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, id).await
}
