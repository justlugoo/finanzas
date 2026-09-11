use crate::error::{AppError, AppResult};
use crate::models::{RouteInputV2, RouteV2};
use crate::repositories;
use crate::utils::km_to_m;
use libsql::Connection;

pub async fn list(conn: &Connection) -> AppResult<Vec<RouteV2>> {
    repositories::routes_v2::list(conn).await
}

pub async fn save(conn: &Connection, input: RouteInputV2) -> AppResult<RouteV2> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.km_round_trip <= 0.0 {
        return Err(AppError::ValidationError("los km deben ser mayores que 0".into()));
    }
    repositories::routes_v2::insert(conn, &name, km_to_m(input.km_round_trip), input.description.as_deref()).await
}

pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::routes_v2::delete(conn, id).await
}
