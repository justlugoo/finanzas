use tauri::State;
use crate::state::{DbState, get_conn};
use crate::error::AppResult;
use crate::models::Meta;
use crate::services::metas as svc;

#[tauri::command]
pub async fn metas_list(state: State<'_, DbState>) -> AppResult<Vec<Meta>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}
