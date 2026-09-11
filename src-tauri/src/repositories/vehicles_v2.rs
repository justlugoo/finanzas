use crate::error::{AppError, AppResult};
use crate::models::VehicleV2;
use libsql::Connection;
use ulid::Ulid;

const COLUMNS: &str = "id, name, efficiency_m_per_l, tank_capacity_ml, created_at, updated_at, deleted_at";

pub fn row_to_vehicle(row: &libsql::Row) -> Result<VehicleV2, libsql::Error> {
    Ok(VehicleV2 {
        id: row.get(0)?,
        name: row.get(1)?,
        efficiency_m_per_l: row.get(2)?,
        tank_capacity_ml: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        deleted_at: row.get(6)?,
    })
}

pub async fn insert(
    conn: &Connection,
    name: &str,
    efficiency_m_per_l: i64,
    tank_capacity_ml: Option<i64>,
) -> AppResult<VehicleV2> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO vehicles (id, name, efficiency_m_per_l, tank_capacity_ml) VALUES (?, ?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), efficiency_m_per_l, tank_capacity_ml],
    )
    .await?;
    get(conn, &id).await
}

pub async fn get(conn: &Connection, id: &str) -> AppResult<VehicleV2> {
    let sql = format!("SELECT {COLUMNS} FROM vehicles WHERE id = ?");
    let mut rows = conn.query(&sql, libsql::params![id.to_string()]).await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("vehículo {id} no existe")))?;
    row_to_vehicle(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(conn: &Connection) -> AppResult<Vec<VehicleV2>> {
    let sql = format!("SELECT {COLUMNS} FROM vehicles WHERE deleted_at IS NULL ORDER BY name");
    let mut rows = conn.query(&sql, ()).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_vehicle(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

/// El rendimiento se puede corregir sin afectar el consumo ya congelado en
/// `trips.consumed_ml` (principio 2 de schema-v2.md) — solo cambia hacia
/// adelante: nuevos viajes y el cálculo de autonomía del nivel actual.
pub async fn update_efficiency(conn: &Connection, id: &str, efficiency_m_per_l: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE vehicles SET efficiency_m_per_l = ?, updated_at = datetime('now') WHERE id = ?",
        libsql::params![efficiency_m_per_l, id.to_string()],
    )
    .await?;
    Ok(())
}

pub async fn update(
    conn: &Connection,
    id: &str,
    name: &str,
    efficiency_m_per_l: i64,
    tank_capacity_ml: Option<i64>,
) -> AppResult<VehicleV2> {
    let affected = conn
        .execute(
            "UPDATE vehicles SET name = ?, efficiency_m_per_l = ?, tank_capacity_ml = ?, updated_at = datetime('now') \
             WHERE id = ? AND deleted_at IS NULL",
            libsql::params![name.to_string(), efficiency_m_per_l, tank_capacity_ml, id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("vehículo {id} no existe")));
    }
    get(conn, id).await
}

pub async fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute(
            "UPDATE vehicles SET deleted_at = datetime('now') WHERE id = ? AND deleted_at IS NULL",
            libsql::params![id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("vehículo {id} no existe")));
    }
    Ok(())
}
