//! Comandos v2 de categorías. **No registrados en `invoke_handler!` todavía**
//! — ver nota en `commands/meta_payments.rs`.

use tauri::State;
use crate::error::AppResult;
use crate::models::{Category, CategoryInput};
use crate::services::categories as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn category_list(state: State<'_, DbState>, kind: Option<String>) -> AppResult<Vec<Category>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, kind.as_deref()).await
}

#[tauri::command]
pub async fn category_create(state: State<'_, DbState>, input: CategoryInput) -> AppResult<Category> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn category_update(
    state: State<'_, DbState>,
    id: String,
    is_fixed: bool,
    route_id: Option<String>,
) -> AppResult<Category> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, &id, is_fixed, route_id).await
}

#[tauri::command]
pub async fn category_delete(state: State<'_, DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, &id).await
}
