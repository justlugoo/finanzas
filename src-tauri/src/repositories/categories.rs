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

/// `include_archived` decide si entran las categorías archivadas (ver
/// `archive()` más abajo): la selección activa (chips de Registrar, lista
/// de Presupuestos) debe excluirlas, pero resolver el nombre de categoría
/// de movimientos ya existentes (Historial, "Último registro", etc.)
/// necesita seguir viéndolas para no perder el nombre real.
pub async fn list(conn: &Connection, kind: Option<&str>, include_archived: bool) -> AppResult<Vec<Category>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM categories WHERE is_system = 0 {} {} ORDER BY name",
        if include_archived { "" } else { "AND archived_at IS NULL" },
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
    name: &str,
    is_fixed: bool,
    route_id: Option<&str>,
) -> AppResult<Category> {
    let affected = conn
        .execute(
            "UPDATE categories SET name = ?, is_fixed = ?, route_id = ?, updated_at = datetime('now') \
             WHERE id = ? AND is_system = 0",
            libsql::params![name.to_string(), is_fixed as i64, route_id.map(|s| s.to_string()), id.to_string()],
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                AppError::ValidationError(format!("ya existe una categoría '{name}' de ese tipo"))
            } else {
                AppError::DatabaseError(e.to_string())
            }
        })?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría {id} no existe")));
    }
    get(conn, id).await
}

/// El error de FK se traduce a un mensaje humano sin exponer el texto crudo
/// de SQLite ni el id (ULID) de la categoría — el llamador (capa de
/// servicio) ya conoce el nombre real y arma el mensaje final con eso.
pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute("DELETE FROM categories WHERE id = ? AND is_system = 0", libsql::params![id.to_string()])
        .await
        .map_err(|_| AppError::ValidationError("todavía tiene movimientos o un presupuesto asociado".into()))?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría {id} no existe")));
    }
    Ok(())
}

/// Alternativa al borrado real cuando una categoría tiene movimientos
/// asociados: `entries.category_id` es `NOT NULL` para income/expense (el
/// `CHECK` compuesto de la tabla lo exige), así que un movimiento ya
/// registrado nunca puede quedar en "sin categoría" — borrar la fila a la
/// fuerza rompería esa regla o requeriría debilitarla. Archivar dejar la
/// categoría fuera de cualquier selección activa (chips, lista de
/// Presupuestos) sin tocar el histórico: los movimientos viejos conservan
/// su categoría real y su nombre real en Historial.
pub async fn archive(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute(
            "UPDATE categories SET archived_at = datetime('now'), updated_at = datetime('now') \
             WHERE id = ? AND is_system = 0 AND archived_at IS NULL",
            libsql::params![id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("categoría {id} no existe o ya está archivada")));
    }
    Ok(())
}
