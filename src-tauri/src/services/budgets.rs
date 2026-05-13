use crate::error::{AppError, AppResult};
use crate::models::Budget;
use crate::repositories::budgets as repo;

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<Budget>> {
    repo::list(conn).await
}

pub async fn create(
    conn: &libsql::Connection,
    category: String,
    monthly_amount: i64,
    kind: String,
    is_fixed: Option<bool>,
) -> AppResult<Budget> {
    let category = category.trim().to_string();
    if category.is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if monthly_amount < 0 {
        return Err(AppError::ValidationError("el monto no puede ser negativo".into()));
    }
    if !matches!(kind.as_str(), "ingreso" | "gasto") {
        return Err(AppError::ValidationError("tipo debe ser 'ingreso' o 'gasto'".into()));
    }
    let effective_fixed = if kind == "ingreso" { is_fixed.unwrap_or(false) } else { false };
    repo::insert(conn, &category, monthly_amount, &kind, effective_fixed).await?;
    Ok(Budget { category, monthly_amount, route_id: None, r#type: kind, is_fixed: effective_fixed })
}

pub async fn update_amount(
    conn: &libsql::Connection,
    category: String,
    monthly_amount: i64,
) -> AppResult<Budget> {
    if monthly_amount < 0 {
        return Err(AppError::ValidationError("el monto no puede ser negativo".into()));
    }
    let affected = repo::update_amount(conn, &category, monthly_amount).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría '{category}' no existe en presupuestos")));
    }
    repo::find_by_category(conn, &category).await?
        .ok_or(AppError::NotFound(category))
}

pub async fn update_route(
    conn: &libsql::Connection,
    category: String,
    route_id: Option<i64>,
) -> AppResult<()> {
    let affected = repo::update_route(conn, &category, route_id).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría '{category}' no existe en presupuestos")));
    }
    Ok(())
}

pub async fn update_fixed(
    conn: &libsql::Connection,
    category: String,
    is_fixed: bool,
) -> AppResult<Budget> {
    let affected = repo::update_fixed(conn, &category, is_fixed).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría de ingreso '{category}' no existe")));
    }
    repo::find_by_category(conn, &category).await?
        .ok_or(AppError::NotFound(category))
}

pub async fn delete(conn: &libsql::Connection, category: String) -> AppResult<()> {
    let affected = repo::delete(conn, &category).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría '{category}' no existe")));
    }
    Ok(())
}
