use libsql::Builder;
use std::path::PathBuf;
use crate::credentials::Credentials;
use crate::error::{AppError, AppResult};

pub fn local_db_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("finanzas");
    std::fs::create_dir_all(&dir).ok();
    dir.join("local.db")
}

pub async fn open_database(credentials: &Credentials) -> AppResult<libsql::Database> {
    let path = local_db_path();
    Builder::new_remote_replica(
        path.to_string_lossy().to_string(),
        credentials.turso_url.clone(),
        credentials.turso_auth_token.clone(),
    )
    .build()
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))
}
