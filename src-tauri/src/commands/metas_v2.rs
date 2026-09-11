//! Comando v2 de la vista unificada de Metas. **No registrado en
//! `invoke_handler!` todavía.**

use tauri::State;
use crate::error::AppResult;
use crate::models::MetaV2;
use crate::services::metas_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn metas_list_v2(state: State<'_, DbState>) -> AppResult<Vec<MetaV2>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}
