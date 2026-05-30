use crate::error::{AppError, AppResult};
use crate::models::FuelFillup;

fn row_to_fillup(row: &libsql::Row) -> Result<FuelFillup, libsql::Error> {
    Ok(FuelFillup {
        id:              row.get(0)?,
        date:            row.get(1)?,
        vehicle_id:      row.get(2)?,
        gallons:         row.get(3)?,
        price_per_gallon: row.get(4)?,
        total_cost:      row.get(5)?,
        note:            row.get(6)?,
        created_at:      row.get(7)?,
        transaction_id:  row.get(8)?,
    })
}

pub async fn insert(
    conn: &libsql::Connection,
    date: &str,
    vehicle_id: i64,
    gallons: f64,
    price_per_gallon: i64,
    total_cost: i64,
    note: Option<&str>,
    transaction_id: Option<i64>,
) -> AppResult<FuelFillup> {
    let mut rows = conn.query(
        "INSERT INTO fuel_fillups \
         (date, vehicle_id, gallons, price_per_gallon, total_cost, note, transaction_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         RETURNING id, date, vehicle_id, gallons, price_per_gallon, total_cost, \
                   note, created_at, transaction_id",
        libsql::params![
            date.to_string(), vehicle_id, gallons, price_per_gallon, total_cost,
            note.map(|s| s.to_string()),
            transaction_id
        ],
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let row = rows.next().await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::DatabaseError("RETURNING vacío tras INSERT fillup".into()))?;

    row_to_fillup(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(
    conn: &libsql::Connection,
    vehicle_id: Option<i64>,
) -> AppResult<Vec<FuelFillup>> {
    let (sql, params): (String, Vec<libsql::Value>) = if let Some(vid) = vehicle_id {
        (
            "SELECT id, date, vehicle_id, gallons, price_per_gallon, total_cost, \
                    note, created_at, transaction_id \
             FROM fuel_fillups WHERE vehicle_id = ? ORDER BY date DESC, id DESC".to_string(),
            vec![libsql::Value::Integer(vid)],
        )
    } else {
        (
            "SELECT id, date, vehicle_id, gallons, price_per_gallon, total_cost, \
                    note, created_at, transaction_id \
             FROM fuel_fillups ORDER BY date DESC, id DESC".to_string(),
            vec![],
        )
    };

    let mut rows = conn.query(&sql, params).await?;
    let mut fillups = Vec::new();
    while let Some(row) = rows.next().await? {
        fillups.push(row_to_fillup(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(fillups)
}

/// Suma total de galones cargados al vehículo via tanqueos.
pub async fn sum_gallons_by_vehicle(conn: &libsql::Connection, vehicle_id: i64) -> AppResult<f64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(gallons), 0.0) FROM fuel_fillups WHERE vehicle_id = ?",
        libsql::params![vehicle_id],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get::<f64>(0).ok()).unwrap_or(0.0))
}

/// Suma de km recorridos por el vehículo en transacciones de viaje.
/// Dividir por km_per_gallon del vehículo da los galones consumidos.
pub async fn sum_trip_km_by_vehicle(conn: &libsql::Connection, vehicle_id: i64) -> AppResult<f64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(gas_km), 0.0) FROM transactions \
         WHERE trip_vehicle_id = ? AND gas_km IS NOT NULL",
        libsql::params![vehicle_id],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get::<f64>(0).ok()).unwrap_or(0.0))
}
