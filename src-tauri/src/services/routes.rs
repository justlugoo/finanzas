use crate::error::{AppError, AppResult};
use crate::models::{CustomRoute, CustomRouteInput};
use crate::repositories::routes as repo;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<CustomRoute>> {
    repo::list(conn).await
}

pub async fn save(conn: &libsql::Connection, route: CustomRouteInput) -> AppResult<CustomRoute> {
    if route.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if route.km_round_trip <= 0.0 {
        return Err(AppError::ValidationError("los km deben ser mayores que 0".into()));
    }
    let name = route.name.trim().to_string();
    let id = repo::insert(conn, &name, route.km_round_trip, route.description.as_deref()).await?;
    Ok(CustomRoute { id, name, km_round_trip: route.km_round_trip, description: route.description })
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    repo::delete(conn, id).await
}
