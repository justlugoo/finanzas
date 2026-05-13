use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use state::DbState;

mod error;
mod db;
mod state;
mod utils;
mod models;
mod repositories;
mod services;
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

            let launched_at_startup = std::env::args().any(|a| a == "--autostart");

            #[cfg(debug_assertions)]
            if launched_at_startup {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().disable();
                eprintln!("[finanzas] autostart registrado con binario debug — entrada eliminada. Compila en release y reactiva el autoarranque.");
                std::process::exit(0);
            }

            if launched_at_startup
                && let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }

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

            let tray_ok2   = Arc::clone(&tray_ok);
            let app_handle = app.handle().clone();
            if let Some(main_win) = app.get_webview_window("main") {
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event
                        && tray_ok2.load(Ordering::Relaxed) {
                            api.prevent_close();
                            if let Some(win) = app_handle.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::transactions::create_transaction,
            commands::transactions::list_transactions,
            commands::transactions::get_current_balance,
            commands::transactions::update_transaction,
            commands::transactions::delete_transaction,
            commands::transactions::get_period_summary,
            commands::transactions::get_category_progress,
            commands::transactions::get_month_comparison,
            commands::transactions::list_categories,
            commands::transactions::export_transactions_csv,
            commands::transactions::import_transactions_csv,
            commands::transactions::delete_transactions_bulk,
            commands::budgets::list_budgets,
            commands::budgets::create_budget,
            commands::budgets::update_budget,
            commands::budgets::update_budget_route,
            commands::budgets::update_budget_fixed,
            commands::budgets::delete_budget,
            commands::goals::list_goals,
            commands::goals::create_goal,
            commands::goals::update_goal,
            commands::goals::delete_goal,
            commands::goals::get_goal_detail,
            commands::gas::get_current_gas_price,
            commands::gas::list_gas_prices,
            commands::gas::register_gas_price_manual,
            commands::gas::get_weekly_gas_comparison,
            commands::gas::get_route_costs,
            commands::vehicles::list_vehicles,
            commands::vehicles::create_vehicle,
            commands::vehicles::update_vehicle,
            commands::vehicles::delete_vehicle,
            commands::routes::get_custom_routes,
            commands::routes::save_custom_route,
            commands::routes::delete_custom_route,
            commands::system::get_autostart_enabled,
            commands::system::set_autostart_enabled,
            commands::system::backup_database,
            commands::system::factory_reset,
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
