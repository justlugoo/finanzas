use crate::error::AppResult;
use crate::models::{GasPrice, WeeklyGasPoint, RoutesCost};

pub async fn find_latest(conn: &libsql::Connection) -> AppResult<Option<GasPrice>> {
    let mut rows = conn.query(
        "SELECT id, date, price_per_gallon, source FROM gas_prices ORDER BY date DESC LIMIT 1",
        (),
    ).await?;

    match rows.next().await? { Some(row) => {
        Ok(Some(GasPrice {
            id: row.get(0)?,
            date: row.get(1)?,
            price_per_gallon: row.get(2)?,
            source: row.get(3)?,
        }))
    } _ => {
        Ok(None)
    }}
}

pub async fn find_latest_price(conn: &libsql::Connection) -> AppResult<Option<i64>> {
    let mut rows = conn.query(
        "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
        (),
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get(0).ok()))
}

pub async fn list(conn: &libsql::Connection, limit: i64) -> AppResult<Vec<GasPrice>> {
    let mut rows = conn.query(
        "SELECT id, date, price_per_gallon, source FROM gas_prices ORDER BY date DESC LIMIT ?",
        libsql::params![limit],
    ).await?;

    let mut prices = Vec::new();
    while let Some(row) = rows.next().await? {
        prices.push(GasPrice {
            id: row.get(0)?,
            date: row.get(1)?,
            price_per_gallon: row.get(2)?,
            source: row.get(3)?,
        });
    }
    Ok(prices)
}

pub async fn upsert(conn: &libsql::Connection, date: &str, price: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO gas_prices (date, price_per_gallon, source) VALUES (?, ?, 'manual') \
         ON CONFLICT(date) DO UPDATE SET price_per_gallon = excluded.price_per_gallon, source = 'manual'",
        libsql::params![date.to_string(), price],
    ).await?;
    Ok(())
}

pub async fn find_by_date(conn: &libsql::Connection, date: &str) -> AppResult<Option<GasPrice>> {
    let mut rows = conn.query(
        "SELECT id, date, price_per_gallon, source FROM gas_prices WHERE date = ?",
        libsql::params![date.to_string()],
    ).await?;

    match rows.next().await? { Some(row) => {
        Ok(Some(GasPrice {
            id: row.get(0)?,
            date: row.get(1)?,
            price_per_gallon: row.get(2)?,
            source: row.get(3)?,
        }))
    } _ => {
        Ok(None)
    }}
}

pub async fn get_vehicle_km_per_gallon(
    conn: &libsql::Connection,
    vehicle_id: i64,
) -> AppResult<Option<f64>> {
    let mut rows = conn.query(
        "SELECT km_per_gallon FROM vehicles WHERE id = ?",
        libsql::params![vehicle_id],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get::<f64>(0).ok()))
}

pub async fn insert_gas_transaction(
    conn: &libsql::Connection,
    date: &str,
    gas_cost: i64,
    gas_note: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transactions (date, type, category, amount, note, is_extraordinary, goal_id) \
         VALUES (?, 'gasto', 'Gasolina', ?, ?, 0, NULL)",
        libsql::params![date.to_string(), gas_cost, gas_note.to_string()],
    ).await?;
    Ok(())
}

pub async fn get_weekly_comparison(conn: &libsql::Connection) -> AppResult<Vec<WeeklyGasPoint>> {
    let mut rows = conn.query(
        "SELECT
            date(date, '-' || ((strftime('%w', date) + 6) % 7) || ' days') AS week_start,
            AVG(price_per_gallon) AS avg_price,
            COUNT(*) AS entry_count
         FROM gas_prices
         GROUP BY week_start
         ORDER BY week_start DESC
         LIMIT 8",
        (),
    ).await?;

    let mut points = Vec::new();
    while let Some(row) = rows.next().await? {
        points.push(WeeklyGasPoint {
            week_start: row.get(0)?,
            avg_price: row.get(1)?,
            entry_count: row.get(2)?,
        });
    }
    Ok(points)
}

pub async fn get_route_costs(conn: &libsql::Connection) -> AppResult<RoutesCost> {
    let mut rows = conn.query(
        "SELECT price_per_gallon FROM gas_prices ORDER BY date DESC LIMIT 1",
        (),
    ).await?;
    let precio_galon: i64 = rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0);
    Ok(RoutesCost { precio_galon })
}
