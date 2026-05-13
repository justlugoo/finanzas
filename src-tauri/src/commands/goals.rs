use tauri::State;
use crate::error::AppResult;
use crate::models::{GoalDetail, GoalInput, GoalWithProgress};
use crate::services::goals as svc;
use crate::state::{DbState, get_conn};

#[tauri::command]
pub async fn list_goals(
    state: State<'_, DbState>,
    status: Option<String>,
) -> AppResult<Vec<GoalWithProgress>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn, status).await
}

#[tauri::command]
pub async fn create_goal(
    state: State<'_, DbState>,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    let conn = get_conn(&state).await?;
    svc::create(&conn, input).await
}

#[tauri::command]
pub async fn update_goal(
    state: State<'_, DbState>,
    id: i64,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    let conn = get_conn(&state).await?;
    svc::update(&conn, id, input).await
}

#[tauri::command]
pub async fn delete_goal(state: State<'_, DbState>, id: i64) -> AppResult<()> {
    let conn = get_conn(&state).await?;
    svc::delete(&conn, id).await
}

#[tauri::command]
pub async fn get_goal_detail(state: State<'_, DbState>, id: i64) -> AppResult<GoalDetail> {
    let conn = get_conn(&state).await?;
    svc::get_detail(&conn, id).await
}
