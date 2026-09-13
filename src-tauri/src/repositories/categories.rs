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

/// Borrado real, siempre — a petición explícita: si una categoría tenía
/// movimientos o un presupuesto asociado, esos movimientos quedan
/// "huérfanos" (`entries.category_id` sigue apuntando a un id que ya no
/// existe; el frontend resuelve el nombre a "Sin categoría" cuando no
/// encuentra la categoría). El presupuesto/override de esa categoría sí se
/// borra de una vez, no tendría sentido dejarlo suelto sin categoría dueña.
///
/// `entries.category_id` tiene una FK hacia `categories(id)` sin
/// `ON DELETE`, así que sin desactivar la verificación de FK para esta
/// conexión el borrado fallaría apenas hubiera un solo movimiento
/// asociado — es exactamente lo que se quiere evitar acá.
pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", ()).await?;

    let result: AppResult<i64> = async {
        conn.execute("DELETE FROM budget_overrides WHERE category_id = ?", libsql::params![id.to_string()]).await?;
        conn.execute("DELETE FROM budgets WHERE category_id = ?", libsql::params![id.to_string()]).await?;
        let affected = conn
            .execute("DELETE FROM categories WHERE id = ? AND is_system = 0", libsql::params![id.to_string()])
            .await?;
        Ok(affected as i64)
    }.await;

    conn.execute("PRAGMA foreign_keys = ON", ()).await?;

    match result? {
        0 => Err(AppError::NotFound(format!("categoría {id} no existe"))),
        _ => Ok(()),
    }
}
