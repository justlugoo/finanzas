use chrono::Local;
use crate::error::{AppError, AppResult};

pub async fn backup_database() -> AppResult<String> {
    let db_dir = if cfg!(debug_assertions) { "finanzas-dev" } else { "finanzas" };
    let src = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(db_dir)
        .join("local.db");
    if !src.exists() {
        return Err(AppError::NotFound("archivo de base de datos local no encontrado".into()));
    }
    let today    = Local::now().format("%Y-%m-%d").to_string();
    let dest_dir = dirs::document_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Documents"));
    let dest = dest_dir.join(format!("finanzas_backup_{today}.db"));
    std::fs::copy(&src, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

pub async fn factory_reset(conn: &libsql::Connection) -> AppResult<()> {
    conn.execute("DELETE FROM transactions",  libsql::params![]).await?;
    conn.execute("DELETE FROM goals",         libsql::params![]).await?;
    conn.execute("DELETE FROM gas_prices",    libsql::params![]).await?;
    conn.execute("DELETE FROM budgets",       libsql::params![]).await?;
    conn.execute("DELETE FROM custom_routes", libsql::params![]).await?;
    conn.execute("DELETE FROM vehicles",      libsql::params![]).await?;
    conn.execute("DELETE FROM sqlite_sequence", libsql::params![]).await?;
    Ok(())
}

pub async fn get_autostart(app: &tauri::AppHandle) -> bool {
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        if let Some(path) = dirs::home_dir().map(|h| h.join(".config/autostart/Finanzas.desktop")) {
            return path.exists();
        }
    }
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

pub async fn set_autostart(_app: &tauri::AppHandle, enabled: bool) -> AppResult<()> {
    #[cfg(target_os = "linux")]
    {
        let desktop = dirs::home_dir()
            .map(|h| h.join(".config/autostart/Finanzas.desktop"))
            .ok_or_else(|| AppError::IoError("No se pudo determinar el directorio home".into()))?;

        if !enabled {
            let _ = std::fs::remove_file(&desktop);
            return Ok(());
        }

        let exe = std::env::current_exe().map_err(|e| AppError::IoError(e.to_string()))?;
        let release_bin = exe
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("release/finanzas"))
            .unwrap_or_else(|| exe.clone());

        if !release_bin.exists() {
            return Err(AppError::DatabaseError(
                "No se encontró el binario release. Ejecuta `cargo build --release` una vez para generarlo.".to_string(),
            ));
        }

        let content = format!(
            "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Finanzas\nComment=Finanzas startup script\nExec={} --autostart\nStartupNotify=false\nTerminal=false\n",
            release_bin.display()
        );
        std::fs::write(&desktop, &content).map_err(|e| AppError::IoError(e.to_string()))?;
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let al = _app.autolaunch();
        if enabled {
            al.enable().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        } else {
            al.disable().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }
}
