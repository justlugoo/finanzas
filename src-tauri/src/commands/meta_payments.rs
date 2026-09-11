//! Comando único de abono para las tres clases de meta (paso 6 de
//! docs/schema-v2.md §12). **No registrado todavía** en `lib.rs` — opera
//! sobre las tablas del esquema v2 (`entries`, `goals`, `accounts`), que no
//! existen aún en la base de datos real hasta que se corra la migración
//! 001. Registrarlo antes de esa migración haría que la app lo llame contra
//! tablas inexistentes.

use tauri::State;
use crate::error::AppResult;
use crate::models::{Entry, MetaPaymentInput};
use crate::services::meta_payments as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn meta_add_payment(state: State<'_, DbState>, input: MetaPaymentInput) -> AppResult<Entry> {
    let conn = get_conn(&state).await?;
    svc::add_payment_input(&conn, input).await
}
