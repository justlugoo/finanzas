use crate::error::{AppError, AppResult};
use crate::models::Vehicle;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<Vehicle>> {
    let mut rows = conn.query(
        "SELECT id, name, km_per_gallon, tank_liters FROM vehicles ORDER BY name",
        (),
    ).await?;
    let mut vehicles = Vec::new();
    while let Some(row) = rows.next().await? {
        vehicles.push(Vehicle {
            id: row.get(0)?,
            name: row.get(1)?,
            km_per_gallon: row.get(2)?,
            tank_liters: row.get(3)?,
        });
    }
    Ok(vehicles)
}

pub async fn insert(
    conn: &libsql::Connection,
    name: &str,
    km_per_gallon: f64,
    tank_liters: Option<f64>,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO vehicles (name, km_per_gallon, tank_liters) VALUES (?, ?, ?)",
        libsql::params![name.to_string(), km_per_gallon, tank_liters],
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(conn.last_insert_rowid())
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    name: &str,
    km_per_gallon: f64,
    tank_liters: Option<f64>,
) -> AppResult<u64> {
    let affected = conn.execute(
        "UPDATE vehicles SET name = ?, km_per_gallon = ?, tank_liters = ? WHERE id = ?",
        libsql::params![name.to_string(), km_per_gallon, tank_liters, id],
    ).await?;
    Ok(affected)
}

pub async fn get_by_id(conn: &libsql::Connection, id: i64) -> AppResult<Option<Vehicle>> {
    let mut rows = conn.query(
        "SELECT id, name, km_per_gallon, tank_liters FROM vehicles WHERE id = ?",
        libsql::params![id],
    ).await?;
    match rows.next().await? {
        Some(row) => Ok(Some(Vehicle {
            id:            row.get(0)?,
            name:          row.get(1)?,
            km_per_gallon: row.get(2)?,
            tank_liters:   row.get(3)?,
        })),
        None => Ok(None),
    }
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<u64> {
    let affected = conn.execute(
        "DELETE FROM vehicles WHERE id = ?",
        libsql::params![id],
    ).await?;
    Ok(affected)
}
