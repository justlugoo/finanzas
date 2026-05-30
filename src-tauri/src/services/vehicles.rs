use crate::error::{AppError, AppResult};
use crate::models::{Vehicle, VehicleInput};
use crate::repositories::vehicles as repo;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<Vehicle>> {
    repo::list(conn).await
}

pub async fn create(conn: &libsql::Connection, input: VehicleInput) -> AppResult<Vehicle> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.km_per_gallon <= 0.0 {
        return Err(AppError::ValidationError("el rendimiento debe ser mayor que 0".into()));
    }
    if let Some(t) = input.tank_liters {
        if t <= 0.0 {
            return Err(AppError::ValidationError("la capacidad del tanque debe ser mayor que 0".into()));
        }
    }
    let id = repo::insert(conn, &name, input.km_per_gallon, input.tank_liters).await?;
    Ok(Vehicle { id, name, km_per_gallon: input.km_per_gallon, tank_liters: input.tank_liters })
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    input: VehicleInput,
) -> AppResult<Vehicle> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.km_per_gallon <= 0.0 {
        return Err(AppError::ValidationError("el rendimiento debe ser mayor que 0".into()));
    }
    if let Some(t) = input.tank_liters {
        if t <= 0.0 {
            return Err(AppError::ValidationError("la capacidad del tanque debe ser mayor que 0".into()));
        }
    }
    let affected = repo::update(conn, id, &name, input.km_per_gallon, input.tank_liters).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("vehículo {id} no existe")));
    }
    Ok(Vehicle { id, name, km_per_gallon: input.km_per_gallon, tank_liters: input.tank_liters })
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    let affected = repo::delete(conn, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("vehículo {id} no existe")));
    }
    Ok(())
}
