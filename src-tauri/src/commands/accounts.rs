//! Comando v2 de cuentas — el frontend necesita resolver `code` ("cash",
//! "payable", "savings"...) a `id` para poder armar cualquier `entry`.
//! **No registrado en `invoke_handler!` todavía.**

use tauri::State;
use crate::error::AppResult;
use crate::models::Account;
use crate::repositories;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn account_list(state: State<'_, DbState>) -> AppResult<Vec<Account>> {
    let conn = get_conn(&state).await?;
    repositories::accounts::list(&conn).await
}
