use std::sync::Mutex;
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};

pub struct DbState(pub Mutex<Option<libsql::Database>>);

#[derive(Serialize, Deserialize, Debug)]
pub struct Budget {
    pub category: String,
    pub monthly_amount: i64,
}

#[tauri::command]
pub async fn list_budgets(state: State<'_, DbState>) -> AppResult<Vec<Budget>> {
    let db = {
        let guard = state.0.lock().map_err(|_| AppError::DatabaseError("mutex poisoned".into()))?;
        guard.as_ref()
            .ok_or_else(|| AppError::DatabaseError("base de datos no inicializada".into()))?
            .connect()
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
    };

    let mut rows = db
        .query("SELECT category, monthly_amount FROM budgets ORDER BY category", ())
        .await?;

    let mut budgets = Vec::new();
    while let Some(row) = rows.next().await? {
        budgets.push(Budget {
            category: row.get(0)?,
            monthly_amount: row.get(1)?,
        });
    }
    Ok(budgets)
}

#[tauri::command]
pub async fn has_turso_credentials() -> bool {
    crate::credentials::has_credentials()
}

#[tauri::command]
pub async fn set_turso_credentials(url: String, token: String) -> AppResult<()> {
    let path = crate::credentials::credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!("turso_url = \"{url}\"\nturso_auth_token = \"{token}\"\n");
    std::fs::write(&path, content)?;
    Ok(())
}
