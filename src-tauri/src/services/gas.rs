use chrono::Local;
use crate::error::{AppError, AppResult};
use crate::models::{GasPrice, RoutesCost, WeeklyGasPoint};
use crate::repositories::gas as repo;
use crate::utils::{format_cop_simple, send_notification};

pub async fn get_current(conn: &libsql::Connection) -> AppResult<Option<GasPrice>> {
    repo::find_latest(conn).await
}

pub async fn list(conn: &libsql::Connection, limit: Option<i64>) -> AppResult<Vec<GasPrice>> {
    repo::list(conn, limit.unwrap_or(30)).await
}

pub async fn register_manual(
    conn: &libsql::Connection,
    app: &tauri::AppHandle,
    price: i64,
) -> AppResult<GasPrice> {
    if !(1000..=100_000).contains(&price) {
        return Err(AppError::ValidationError(
            "el precio debe estar entre 1.000 y 100.000 COP/galón".into(),
        ));
    }

    let prev_price = repo::find_latest_price(conn).await?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    repo::upsert(conn, &today, price).await?;

    let result = repo::find_by_date(conn, &today).await?
        .ok_or_else(|| AppError::DatabaseError("precio no encontrado tras insertar".into()))?;

    if let Some(anterior) = prev_price
        && anterior > 0 {
            let delta = (price - anterior).abs() as f64 / anterior as f64;
            if delta > 0.05 {
                let direccion = if price > anterior { "subió" } else { "bajó" };
                let delta_pct = (delta * 100.0).round() as i64;
                send_notification(
                    app,
                    "⛽ Precio de gasolina",
                    &format!("El precio {} {}% → {}/gal", direccion, delta_pct, format_cop_simple(price)),
                );
            }
        }

    Ok(result)
}

pub async fn get_weekly_comparison(conn: &libsql::Connection) -> AppResult<Vec<WeeklyGasPoint>> {
    repo::get_weekly_comparison(conn).await
}

pub async fn get_route_costs(conn: &libsql::Connection) -> AppResult<RoutesCost> {
    repo::get_route_costs(conn).await
}
