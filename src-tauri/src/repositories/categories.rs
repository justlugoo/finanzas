use crate::error::{AppError, AppResult};
use crate::models::Category;
use libsql::Connection;
use ulid::Ulid;

const COLUMNS: &str = "id, name, kind, is_fixed, route_id, is_system, code, archived_at, created_at, updated_at";

pub fn row_to_category(row: &libsql::Row) -> Result<Category, libsql::Error> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        is_fixed: row.get::<i64>(3)? != 0,
        route_id: row.get(4)?,
        is_system: row.get::<i64>(5)? != 0,
        code: row.get(6)?,
        archived_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub async fn list(conn: &Connection, kind: Option<&str>) -> AppResult<Vec<Category>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM categories WHERE is_system = 0 {} ORDER BY name",
        if kind.is_some() { "AND kind = ?" } else { "" }
    );
    let mut rows = match kind {
        Some(k) => conn.query(&sql, libsql::params![k.to_string()]).await?,
        None => conn.query(&sql, ()).await?,
    };
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_category(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn get(conn: &Connection, id: &str) -> AppResult<Category> {
    let sql = format!("SELECT {COLUMNS} FROM categories WHERE id = ?");
    let mut rows = conn.query(&sql, libsql::params![id.to_string()]).await?;
    let row = rows.next().await?.ok_or_else(|| AppError::NotFound(format!("categoría {id} no existe")))?;
    row_to_category(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn insert(
    conn: &Connection,
    name: &str,
    kind: &str,
    is_fixed: bool,
    route_id: Option<&str>,
) -> AppResult<Category> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO categories (id, name, kind, is_fixed, route_id) VALUES (?, ?, ?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), kind.to_string(), is_fixed as i64, route_id.map(|s| s.to_string())],
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::ValidationError(format!("ya existe una categoría '{name}' de tipo {kind}"))
        } else {
            AppError::DatabaseError(e.to_string())
        }
    })?;
    get(conn, &id).await
}

pub async fn update(
    conn: &Connection,
    id: &str,
    is_fixed: bool,
    route_id: Option<&str>,
) -> AppResult<Category> {
    let affected = conn
        .execute(
            "UPDATE categories SET is_fixed = ?, route_id = ?, updated_at = datetime('now') WHERE id = ? AND is_system = 0",
            libsql::params![is_fixed as i64, route_id.map(|s| s.to_string()), id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría {id} no existe")));
    }
    get(conn, id).await
}

pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute("DELETE FROM categories WHERE id = ? AND is_system = 0", libsql::params![id.to_string()])
        .await
        .map_err(|e| {
            AppError::ValidationError(format!(
                "no se puede borrar la categoría {id}: todavía tiene movimientos o presupuesto asociado ({e})"
            ))
        })?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría {id} no existe")));
    }
    Ok(())
}
