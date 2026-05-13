use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::State;
use crate::error::{AppError, AppResult};

pub struct DbState {
    pub db:   Arc<RwLock<Option<libsql::Database>>>,
    pub conn: Arc<tokio::sync::Mutex<Option<libsql::Connection>>>,
}

pub struct ConnGuard(pub tokio::sync::OwnedMutexGuard<Option<libsql::Connection>>);

impl std::ops::Deref for ConnGuard {
    type Target = libsql::Connection;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("conn inicializado en get_conn")
    }
}

pub async fn get_conn(state: &State<'_, DbState>) -> AppResult<ConnGuard> {
    for _ in 0..20u8 {
        if state.db.read().await.is_some() { break; }
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }
    let mut guard = state.conn.clone().lock_owned().await;
    if guard.is_none() {
        let db_guard = state.db.read().await;
        let db = db_guard.as_ref()
            .ok_or_else(|| AppError::DatabaseError("base de datos no inicializada".into()))?;
        let conn = db.connect().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        crate::db::apply_pragmas(&conn).await?;
        *guard = Some(conn);
    }
    Ok(ConnGuard(guard))
}
