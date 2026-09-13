use crate::error::{AppError, AppResult};
use crate::models::{Category, CategoryInput};
use crate::repositories;
use libsql::Connection;

pub async fn list(conn: &Connection, kind: Option<&str>, include_archived: bool) -> AppResult<Vec<Category>> {
    repositories::categories::list(conn, kind, include_archived).await
}

pub async fn create(conn: &Connection, input: CategoryInput) -> AppResult<Category> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if !matches!(input.kind.as_str(), "income" | "expense") {
        return Err(AppError::ValidationError("kind debe ser 'income' o 'expense'".into()));
    }
    repositories::categories::insert(conn, &name, &input.kind, input.is_fixed, input.route_id.as_deref()).await
}

pub async fn update(
    conn: &Connection,
    id: &str,
    name: &str,
    is_fixed: bool,
    route_id: Option<String>,
) -> AppResult<Category> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    repositories::categories::update(conn, id, name, is_fixed, route_id.as_deref()).await
}

/// "Eliminar" desde la UI siempre borra la categoría de verdad, tenga o no
/// movimientos/presupuesto asociados — ver `repositories::categories::delete`
/// para el porqué (movimientos viejos quedan con una categoría "huérfana",
/// a propósito).
pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::categories::delete(conn, id).await
}
