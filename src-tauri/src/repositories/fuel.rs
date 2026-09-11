use crate::error::AppResult;
use crate::models::{FillupV2, FuelAdjustment, Trip};
use libsql::Connection;
use ulid::Ulid;

pub async fn insert_fillup(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    volume_ml: i64,
    price_cop_per_gallon: i64,
    total_cop: i64,
    entry_id: Option<&str>,
    note: Option<&str>,
) -> AppResult<FillupV2> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO fillups \
         (id, occurred_on, vehicle_id, volume_ml, price_cop_per_gallon, total_cop, entry_id, note) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            id.clone(),
            occurred_on.to_string(),
            vehicle_id.to_string(),
            volume_ml,
            price_cop_per_gallon,
            total_cop,
            entry_id.map(|s| s.to_string()),
            note.map(|s| s.to_string())
        ],
    )
    .await?;
    Ok(FillupV2 {
        id,
        occurred_on: occurred_on.to_string(),
        vehicle_id: vehicle_id.to_string(),
        volume_ml,
        price_cop_per_gallon,
        total_cop,
        entry_id: entry_id.map(|s| s.to_string()),
        note: note.map(|s| s.to_string()),
    })
}

pub async fn insert_trip(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    distance_m: i64,
    consumed_ml: i64,
    entry_id: Option<&str>,
) -> AppResult<Trip> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO trips (id, occurred_on, vehicle_id, distance_m, consumed_ml, entry_id) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params![
            id.clone(),
            occurred_on.to_string(),
            vehicle_id.to_string(),
            distance_m,
            consumed_ml,
            entry_id.map(|s| s.to_string())
        ],
    )
    .await?;
    Ok(Trip {
        id,
        occurred_on: occurred_on.to_string(),
        vehicle_id: vehicle_id.to_string(),
        distance_m,
        consumed_ml,
        entry_id: entry_id.map(|s| s.to_string()),
    })
}

pub async fn insert_adjustment(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    level_ml: i64,
    note: Option<&str>,
) -> AppResult<FuelAdjustment> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO fuel_adjustments (id, occurred_on, vehicle_id, level_ml, note) VALUES (?, ?, ?, ?, ?)",
        libsql::params![id.clone(), occurred_on.to_string(), vehicle_id.to_string(), level_ml, note.map(|s| s.to_string())],
    )
    .await?;
    Ok(FuelAdjustment {
        id,
        occurred_on: occurred_on.to_string(),
        vehicle_id: vehicle_id.to_string(),
        level_ml,
        note: note.map(|s| s.to_string()),
    })
}

pub async fn list_fillups(conn: &Connection, vehicle_id: Option<&str>) -> AppResult<Vec<FillupV2>> {
    let sql = format!(
        "SELECT id, occurred_on, vehicle_id, volume_ml, price_cop_per_gallon, total_cop, entry_id, note \
         FROM fillups WHERE deleted_at IS NULL {} ORDER BY occurred_on DESC",
        if vehicle_id.is_some() { "AND vehicle_id = ?" } else { "" }
    );
    let mut rows = match vehicle_id {
        Some(v) => conn.query(&sql, libsql::params![v.to_string()]).await?,
        None => conn.query(&sql, ()).await?,
    };
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(FillupV2 {
            id: row.get(0)?,
            occurred_on: row.get(1)?,
            vehicle_id: row.get(2)?,
            volume_ml: row.get(3)?,
            price_cop_per_gallon: row.get(4)?,
            total_cop: row.get(5)?,
            entry_id: row.get(6)?,
            note: row.get(7)?,
        });
    }
    Ok(out)
}

struct Anchor {
    occurred_on: String,
    level_ml: i64,
}

async fn latest_anchor(conn: &Connection, vehicle_id: &str, as_of: &str) -> AppResult<Option<Anchor>> {
    let mut rows = conn
        .query(
            "SELECT occurred_on, level_ml FROM fuel_adjustments \
             WHERE vehicle_id = ? AND occurred_on <= ? ORDER BY occurred_on DESC, id DESC LIMIT 1",
            libsql::params![vehicle_id.to_string(), as_of.to_string()],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(Anchor { occurred_on: row.get(0)?, level_ml: row.get(1)? }),
        None => None,
    })
}

/// Nivel **sin topar** a la capacidad del tanque — sección 7 de
/// schema-v2.md: `base + Σ(fillups.volume_ml) − Σ(trips.consumed_ml)` desde
/// la última ancla (o desde "el principio de los tiempos" si no hay
/// ninguna). El clamp lo aplica la capa de servicio, una sola vez.
pub async fn raw_level_ml(conn: &Connection, vehicle_id: &str, as_of: &str) -> AppResult<i64> {
    let anchor = latest_anchor(conn, vehicle_id, as_of).await?;
    let (base, since) = match anchor {
        Some(a) => (a.level_ml, a.occurred_on),
        None => (0, "0000-01-01".to_string()),
    };
    let mut rows = conn
        .query(
            "SELECT
                COALESCE((SELECT SUM(volume_ml) FROM fillups WHERE vehicle_id = ?1 AND occurred_on >= ?2 AND deleted_at IS NULL), 0)
                -
                COALESCE((SELECT SUM(consumed_ml) FROM trips WHERE vehicle_id = ?1 AND occurred_on >= ?2 AND deleted_at IS NULL), 0)",
            libsql::params![vehicle_id.to_string(), since],
        )
        .await?;
    let row = rows.next().await?.expect("SELECT con COALESCE siempre retorna una fila");
    let delta: i64 = row.get(0)?;
    Ok(base + delta)
}
