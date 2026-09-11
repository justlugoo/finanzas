use crate::error::{AppError, AppResult};
use crate::models::GoalV2;
use libsql::Connection;
use ulid::Ulid;

const COLUMNS: &str = "id, name, target_cop, target_date, kind, installments, created_at, updated_at, deleted_at";

pub fn row_to_goal(row: &libsql::Row) -> Result<GoalV2, libsql::Error> {
    Ok(GoalV2 {
        id: row.get(0)?,
        name: row.get(1)?,
        target_cop: row.get(2)?,
        target_date: row.get(3)?,
        kind: row.get(4)?,
        installments: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        deleted_at: row.get(8)?,
    })
}

/// `"saving"` o `"debt"` — todo lo que `meta_add_payment` necesita saber
/// para decidir hacia dónde va la transferencia.
pub async fn kind_by_id(conn: &Connection, goal_id: &str) -> AppResult<String> {
    let mut rows = conn
        .query("SELECT kind FROM goals WHERE id = ?", libsql::params![goal_id.to_string()])
        .await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("goal {goal_id} no existe")))?;
    Ok(row.get(0)?)
}

pub async fn insert(
    conn: &Connection,
    name: &str,
    target_cop: i64,
    target_date: Option<&str>,
    kind: &str,
    installments: Option<i64>,
) -> AppResult<GoalV2> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO goals (id, name, target_cop, target_date, kind, installments) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), target_cop, target_date.map(|s| s.to_string()), kind.to_string(), installments],
    )
    .await?;
    get(conn, &id).await
}

pub async fn get(conn: &Connection, id: &str) -> AppResult<GoalV2> {
    let sql = format!("SELECT {COLUMNS} FROM goals WHERE id = ? AND deleted_at IS NULL");
    let mut rows = conn.query(&sql, libsql::params![id.to_string()]).await?;
    let row = rows.next().await?.ok_or_else(|| AppError::NotFound(format!("goal {id} no existe")))?;
    row_to_goal(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(conn: &Connection, kind: Option<&str>) -> AppResult<Vec<GoalV2>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM goals WHERE deleted_at IS NULL {} ORDER BY created_at DESC",
        if kind.is_some() { "AND kind = ?" } else { "" }
    );
    let mut rows = match kind {
        Some(k) => conn.query(&sql, libsql::params![k.to_string()]).await?,
        None => conn.query(&sql, ()).await?,
    };
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_goal(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn update(
    conn: &Connection,
    id: &str,
    name: &str,
    target_cop: i64,
    target_date: Option<&str>,
    installments: Option<i64>,
) -> AppResult<GoalV2> {
    let affected = conn
        .execute(
            "UPDATE goals SET name = ?, target_cop = ?, target_date = ?, installments = ?, updated_at = datetime('now') \
             WHERE id = ? AND deleted_at IS NULL",
            libsql::params![name.to_string(), target_cop, target_date.map(|s| s.to_string()), installments, id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("goal {id} no existe")));
    }
    get(conn, id).await
}

pub async fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute(
            "UPDATE goals SET deleted_at = datetime('now') WHERE id = ? AND deleted_at IS NULL",
            libsql::params![id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("goal {id} no existe")));
    }
    Ok(())
}

/// El `expense` que representa "comprar a crédito" en sí (account_from =
/// payable) — distinto de los abonos posteriores, que son `transfer`. Debe
/// haber como mucho una fila así por goal de tipo debt.
pub async fn debt_opening_entry_id(conn: &Connection, goal_id: &str) -> AppResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT e.id FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             WHERE e.kind = 'expense' AND e.goal_id = ? AND a_from.code = 'payable' AND e.deleted_at IS NULL
             ORDER BY e.created_at ASC LIMIT 1",
            libsql::params![goal_id.to_string()],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

/// Todas las entries (compra a crédito / aportes + abonos) asociadas a un
/// `goal_id` — para borrarlas junto con la meta y no dejar movimientos
/// huérfanos afectando el patrimonio.
pub async fn soft_delete_entries(conn: &Connection, goal_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE entries SET deleted_at = datetime('now') WHERE goal_id = ? AND deleted_at IS NULL",
        libsql::params![goal_id.to_string()],
    )
    .await?;
    Ok(())
}

/// Progreso de una meta — sección 6 de schema-v2.md. La dirección de la
/// transferencia es parte del filtro, no un `SUM` indiscriminado.
pub async fn current_amount(conn: &Connection, goal_id: &str, kind: &str) -> AppResult<i64> {
    let to_code = if kind == "debt" { "payable" } else { "savings" };
    let mut rows = conn
        .query(
            "SELECT COALESCE(SUM(e.amount_cop), 0) FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             JOIN accounts a_to   ON a_to.id   = e.account_to
             WHERE e.kind = 'transfer' AND e.goal_id = ? AND a_from.code = 'cash' AND a_to.code = ?
               AND e.deleted_at IS NULL",
            libsql::params![goal_id.to_string(), to_code],
        )
        .await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

/// Solo los `transfer` que cuentan como abono real (misma dirección que
/// `current_amount`) — para la vista unificada de Metas.
pub async fn abonos(conn: &Connection, goal_id: &str, kind: &str) -> AppResult<Vec<crate::models::MetaAbonoV2>> {
    let to_code = if kind == "debt" { "payable" } else { "savings" };
    let mut rows = conn
        .query(
            "SELECT e.id, e.occurred_on, e.amount_cop FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             JOIN accounts a_to   ON a_to.id   = e.account_to
             WHERE e.kind = 'transfer' AND e.goal_id = ? AND a_from.code = 'cash' AND a_to.code = ?
               AND e.deleted_at IS NULL
             ORDER BY e.occurred_on DESC",
            libsql::params![goal_id.to_string(), to_code],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(crate::models::MetaAbonoV2 { id: row.get(0)?, date: row.get(1)?, amount: row.get(2)? });
    }
    Ok(out)
}

pub async fn contributions(conn: &Connection, goal_id: &str) -> AppResult<Vec<crate::models::Entry>> {
    let sql = "SELECT id, occurred_on, kind, amount_cop, account_from, account_to, category_id, goal_id, loan_id, note, is_extraordinary, created_at, updated_at \
               FROM entries WHERE goal_id = ? AND deleted_at IS NULL ORDER BY occurred_on DESC, id DESC";
    let mut rows = conn.query(sql, libsql::params![goal_id.to_string()]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(super::entries::row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}
