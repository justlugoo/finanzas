use crate::error::AppResult;
use crate::models::RouteV2;
use libsql::Connection;
use ulid::Ulid;

pub fn row_to_route(row: &libsql::Row) -> Result<RouteV2, libsql::Error> {
    Ok(RouteV2 { id: row.get(0)?, name: row.get(1)?, distance_m: row.get(2)?, description: row.get(3)? })
}

pub async fn list(conn: &Connection) -> AppResult<Vec<RouteV2>> {
    let mut rows = conn.query("SELECT id, name, distance_m, description FROM routes ORDER BY name", ()).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_route(&row).map_err(|e| crate::error::AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn insert(conn: &Connection, name: &str, distance_m: i64, description: Option<&str>) -> AppResult<RouteV2> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO routes (id, name, distance_m, description) VALUES (?, ?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), distance_m, description.map(|s| s.to_string())],
    )
    .await?;
    Ok(RouteV2 { id, name: name.to_string(), distance_m, description: description.map(|s| s.to_string()) })
}

pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM routes WHERE id = ?", libsql::params![id.to_string()]).await?;
    Ok(())
}
