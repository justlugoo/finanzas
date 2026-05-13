use crate::error::AppResult;
use crate::models::CustomRoute;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<CustomRoute>> {
    let mut rows = conn.query(
        "SELECT id, name, km_round_trip, description, use_vehicle, fixed_cost
         FROM custom_routes ORDER BY name",
        libsql::params![],
    ).await?;
    let mut routes = Vec::new();
    while let Some(row) = rows.next().await? {
        routes.push(CustomRoute {
            id:            row.get(0)?,
            name:          row.get(1)?,
            km_round_trip: row.get(2)?,
            description:   row.get(3)?,
            use_vehicle:   row.get::<i64>(4)? != 0,
            fixed_cost:    row.get(5)?,
        });
    }
    Ok(routes)
}

pub async fn insert(
    conn: &libsql::Connection,
    name: &str,
    km_round_trip: f64,
    description: Option<&str>,
    use_vehicle: bool,
    fixed_cost: Option<i64>,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO custom_routes (name, km_round_trip, description, use_vehicle, fixed_cost)
         VALUES (?, ?, ?, ?, ?)",
        libsql::params![
            name.to_string(),
            km_round_trip,
            description.map(|s| s.to_string()),
            use_vehicle as i64,
            fixed_cost,
        ],
    ).await?;
    Ok(conn.last_insert_rowid())
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM custom_routes WHERE id = ?",
        libsql::params![id],
    ).await?;
    Ok(())
}
