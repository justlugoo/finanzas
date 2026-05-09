use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use commands::DbState;

mod error;
mod credentials;
mod db;
pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(DbState(Arc::new(Mutex::new(None))));
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match init_db().await {
                    Ok(database) => {
                        let state = handle.state::<DbState>();
                        let mut guard = state.0.lock().await;
                        *guard = Some(database);
                        println!("[finanzas] DB lista");
                    }
                    Err(e) => {
                        eprintln!("[finanzas] Error al iniciar DB: {e:?}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_budgets,
            commands::create_transaction,
            commands::list_transactions,
            commands::update_transaction,
            commands::delete_transaction,
            commands::get_period_summary,
            commands::get_category_progress,
            commands::has_turso_credentials,
            commands::set_turso_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error running Finanzas");
}

async fn init_db() -> Result<libsql::Database, Box<dyn std::error::Error + Send + Sync>> {
    let creds = credentials::load_credentials()?;
    let database = db::open_database(&creds).await?;
    database.sync().await?;
    // Aplica schema localmente (idempotente). Garantiza que las tablas
    // existan incluso cuando la replica local es nueva y el sync no trae DDL.
    let conn = database.connect()?;
    db::apply_schema(&conn).await?;
    Ok(database)
}
