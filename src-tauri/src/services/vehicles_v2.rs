use crate::error::{AppError, AppResult};
use crate::models::{VehicleInputV2, VehicleV2};
use crate::repositories;
use crate::utils::{gallons_to_ml, km_per_gallon_to_m_per_l};
use libsql::Connection;

fn validate(input: &VehicleInputV2) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.km_per_gallon <= 0.0 {
        return Err(AppError::ValidationError("el rendimiento debe ser mayor que 0".into()));
    }
    if input.tank_gallons <= 0.0 {
        return Err(AppError::ValidationError("la capacidad del tanque debe ser mayor que 0".into()));
    }
    Ok(())
}

pub async fn list(conn: &Connection) -> AppResult<Vec<VehicleV2>> {
    repositories::vehicles_v2::list(conn).await
}

pub async fn create(conn: &Connection, input: VehicleInputV2) -> AppResult<VehicleV2> {
    validate(&input)?;
    let efficiency = km_per_gallon_to_m_per_l(input.km_per_gallon);
    let tank_capacity_ml = Some(gallons_to_ml(input.tank_gallons));
    repositories::vehicles_v2::insert(conn, input.name.trim(), efficiency, tank_capacity_ml).await
}

pub async fn update(conn: &Connection, id: &str, input: VehicleInputV2) -> AppResult<VehicleV2> {
    validate(&input)?;
    let efficiency = km_per_gallon_to_m_per_l(input.km_per_gallon);
    let tank_capacity_ml = Some(gallons_to_ml(input.tank_gallons));
    repositories::vehicles_v2::update(conn, id, input.name.trim(), efficiency, tank_capacity_ml).await
}

pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::vehicles_v2::soft_delete(conn, id).await
}
