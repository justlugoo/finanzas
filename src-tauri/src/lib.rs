use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use commands::DbState;

mod error;
mod db;
pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            app.manage(DbState {
                db:   Arc::new(RwLock::new(None)),
                conn: Arc::new(tokio::sync::Mutex::new(None)),
            });

            // Si el arg --autostart está presente → arrancó con el sistema.
            // Ocultamos la ventana; el usuario la abre desde el tray.
            let launched_at_startup = std::env::args().any(|a| a == "--autostart");

            // Los binarios debug no embeben el frontend: dependen del servidor Vite
            // (localhost:1420) que no existe al arrancar el sistema.
            // Si detectamos ese caso, desregistramos el autostart y salimos antes
            // de que el WebView muestre la pantalla en blanco de "Connection refused".
            #[cfg(debug_assertions)]
            if launched_at_startup {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().disable();
                eprintln!("[finanzas] autostart registrado con binario debug — entrada eliminada. Compila en release y reactiva el autoarranque.");
                std::process::exit(0);
            }

            if launched_at_startup {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }

            // Re-register autostart so the .desktop entry includes --autostart.
            // Skipped in debug builds: the plugin's is_enabled() detects the
            // existing .desktop file and enable() would overwrite it with the
            // debug binary path, breaking the release registration.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let mgr = app.autolaunch();
                if mgr.is_enabled().unwrap_or(false) && !launched_at_startup {
                    let _ = mgr.enable();
                }
            }

            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match init_db().await {
                    Ok(database) => {
                        let state = handle.state::<DbState>();
                        let mut guard = state.db.write().await;
                        *guard = Some(database);
                        println!("[finanzas] DB lista");
                    }
                    Err(e) => {
                        eprintln!("[finanzas] Error al iniciar DB: {e:?}");
                    }
                }
            });

            // ── System tray ───────────────────────────────────────────────
            // En Linux sin AppIndicator (GNOME puro, Fedora sin extensión),
            // libappindicator-sys *panea* al cargar la librería dinámica.
            // Capturamos el pánico para que la app arranque igual sin tray.
            let tray_ok = Arc::new(AtomicBool::new(false));

            let tray_result: Result<(), String> =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
                    let open_item =
                        MenuItem::with_id(app, "open", "Abrir Finanzas", true, None::<&str>)
                            .map_err(|e| e.to_string())?;
                    let sep = PredefinedMenuItem::separator(app)
                        .map_err(|e| e.to_string())?;
                    let quit_item =
                        MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)
                            .map_err(|e| e.to_string())?;
                    let menu = Menu::with_items(app, &[&open_item, &sep, &quit_item])
                        .map_err(|e| e.to_string())?;

                    let mut builder = TrayIconBuilder::new()
                        .menu(&menu)
                        .on_menu_event(|app, event| match event.id.as_ref() {
                            "open" => {
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                            "quit" => app.exit(0),
                            _ => {}
                        })
                        .on_tray_icon_event(|tray, event| {
                            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                            if let TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } = event
                            {
                                let app = tray.app_handle();
                                if let Some(win) = app.get_webview_window("main") {
                                    if win.is_visible().unwrap_or(false) {
                                        let _ = win.hide();
                                    } else {
                                        let _ = win.show();
                                        let _ = win.set_focus();
                                    }
                                }
                            }
                        });

                    if let Some(icon) = app.default_window_icon() {
                        builder = builder.icon(icon.clone());
                    }

                    builder.build(app).map_err(|e| e.to_string())?;
                    Ok(())
                }))
                .unwrap_or_else(|_| Err("AppIndicator no disponible".to_string()));

            match tray_result {
                Ok(()) => {
                    tray_ok.store(true, Ordering::Relaxed);
                    println!("[finanzas] tray activo");
                }
                Err(e) => {
                    eprintln!("[finanzas] tray no disponible ({e}) — cerrando la ventana cierra la app");
                }
            }

            // ── Intercept window close: hide si tray activo, cerrar si no ─
            let tray_ok2   = Arc::clone(&tray_ok);
            let app_handle = app.handle().clone();
            if let Some(main_win) = app.get_webview_window("main") {
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if tray_ok2.load(Ordering::Relaxed) {
                            api.prevent_close();
                            if let Some(win) = app_handle.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                    }
                });
            }

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
            commands::get_month_comparison,
            commands::list_categories,
            commands::list_active_goals,
            commands::export_transactions_csv,
            commands::import_transactions_csv,
            commands::list_goals,
            commands::create_goal,
            commands::update_goal,
            commands::delete_goal,
            commands::get_goal_detail,
            commands::get_current_gas_price,
            commands::list_gas_prices,
            commands::register_gas_price_manual,
            commands::get_weekly_gas_comparison,
            commands::calculate_trip_cost,
            commands::get_config_value,
            commands::get_route_costs,
            commands::update_budget,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::backup_database,
            commands::get_current_balance,
            commands::factory_reset,
            commands::delete_transactions_bulk,
        ])
        .run(tauri::generate_context!())
        .expect("error running Finanzas");
}

async fn init_db() -> Result<libsql::Database, Box<dyn std::error::Error + Send + Sync>> {
    let database = db::open_database().await?;
    let conn = database.connect()?;
    db::apply_pragmas(&conn).await?;
    db::apply_schema(&conn).await?;
    Ok(database)
}
