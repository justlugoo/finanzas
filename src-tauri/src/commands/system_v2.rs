//! Reinicio de fábrica v2. **No registrado en `invoke_handler!` todavía.**
//! `backup_database` y el autoarranque no cambian con el esquema — se
//! reusan tal cual desde `commands::system`.

use tauri::State;
use crate::error::AppResult;
use crate::services::system_v2 as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn factory_reset_v2(state: State<'_, DbState>) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::factory_reset(&conn).await
}
