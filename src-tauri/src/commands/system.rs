use tauri::State;
use crate::error::AppResult;
use crate::services::system as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> bool {
    svc::get_autostart(&app).await
}

#[tauri::command]
pub async fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> AppResult<()> {
    svc::set_autostart(&app, enabled).await

}

#[tauri::command]
pub async fn backup_database() -> AppResult<String> {
    svc::backup_database().await
}

#[tauri::command]
pub async fn factory_reset(state: State<'_, DbState>) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::factory_reset(&conn).await
}
