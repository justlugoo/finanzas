use crate::error::{AppError, AppResult};
use crate::models::LoanV2;
use libsql::Connection;
use ulid::Ulid;

const COLUMNS: &str = "id, person_name, principal_cop, lent_on, note, created_at, updated_at, deleted_at";

pub fn row_to_loan(row: &libsql::Row) -> Result<LoanV2, libsql::Error> {
    Ok(LoanV2 {
        id: row.get(0)?,
        person_name: row.get(1)?,
        principal_cop: row.get(2)?,
        lent_on: row.get(3)?,
        note: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        deleted_at: row.get(7)?,
    })
}

pub async fn insert(conn: &Connection, person_name: &str, principal_cop: i64, lent_on: &str, note: Option<&str>) -> AppResult<LoanV2> {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO loans (id, person_name, principal_cop, lent_on, note) VALUES (?, ?, ?, ?, ?)",
        libsql::params![id.clone(), person_name.to_string(), principal_cop, lent_on.to_string(), note.map(|s| s.to_string())],
    )
    .await?;
    get(conn, &id).await
}

pub async fn get(conn: &Connection, id: &str) -> AppResult<LoanV2> {
    let sql = format!("SELECT {COLUMNS} FROM loans WHERE id = ? AND deleted_at IS NULL");
    let mut rows = conn.query(&sql, libsql::params![id.to_string()]).await?;
    let row = rows.next().await?.ok_or_else(|| AppError::NotFound(format!("préstamo {id} no existe")))?;
    row_to_loan(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(conn: &Connection) -> AppResult<Vec<LoanV2>> {
    let sql = format!("SELECT {COLUMNS} FROM loans WHERE deleted_at IS NULL ORDER BY lent_on DESC");
    let mut rows = conn.query(&sql, ()).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_loan(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn update(conn: &Connection, id: &str, person_name: &str, principal_cop: i64) -> AppResult<LoanV2> {
    let affected = conn
        .execute(
            "UPDATE loans SET person_name = ?, principal_cop = ?, updated_at = datetime('now') WHERE id = ? AND deleted_at IS NULL",
            libsql::params![person_name.to_string(), principal_cop, id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("préstamo {id} no existe")));
    }
    get(conn, id).await
}

pub async fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute(
            "UPDATE loans SET deleted_at = datetime('now') WHERE id = ? AND deleted_at IS NULL",
            libsql::params![id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("préstamo {id} no existe")));
    }
    Ok(())
}

/// El `entry` de tipo `transfer cash→receivable` que representa "prestar la
/// plata" en sí — el que se crea una sola vez al dar de alta el préstamo
/// (sección 4 de schema-v2.md: "Prestar plata | transfer | cash | receivable").
/// Se distingue de los abonos (`receivable→cash`) por la dirección.
pub async fn lending_entry_id(conn: &Connection, loan_id: &str) -> AppResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT e.id FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             JOIN accounts a_to   ON a_to.id   = e.account_to
             WHERE e.kind = 'transfer' AND e.loan_id = ? AND a_from.code = 'cash' AND a_to.code = 'receivable'
               AND e.deleted_at IS NULL
             ORDER BY e.created_at ASC LIMIT 1",
            libsql::params![loan_id.to_string()],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

/// Todas las entries (préstamo + abonos) asociadas a un `loan_id` — para
/// borrarlas junto con el préstamo y no dejar movimientos huérfanos.
pub async fn soft_delete_entries(conn: &Connection, loan_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE entries SET deleted_at = datetime('now') WHERE loan_id = ? AND deleted_at IS NULL",
        libsql::params![loan_id.to_string()],
    )
    .await?;
    Ok(())
}

/// `pagado = Σ(transfers receivable→cash del loan)` — sección 6 de
/// schema-v2.md. `loans.status` no existe en v2: siempre se deriva.
pub async fn paid(conn: &Connection, loan_id: &str) -> AppResult<i64> {
    let mut rows = conn
        .query(
            "SELECT COALESCE(SUM(e.amount_cop), 0) FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             JOIN accounts a_to   ON a_to.id   = e.account_to
             WHERE e.kind = 'transfer' AND e.loan_id = ? AND a_from.code = 'receivable' AND a_to.code = 'cash'
               AND e.deleted_at IS NULL",
            libsql::params![loan_id.to_string()],
        )
        .await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

pub async fn payments(conn: &Connection, loan_id: &str) -> AppResult<Vec<crate::models::MetaAbonoV2>> {
    let mut rows = conn
        .query(
            "SELECT e.id, e.occurred_on, e.amount_cop FROM entries e
             JOIN accounts a_from ON a_from.id = e.account_from
             JOIN accounts a_to   ON a_to.id   = e.account_to
             WHERE e.kind = 'transfer' AND e.loan_id = ? AND a_from.code = 'receivable' AND a_to.code = 'cash'
               AND e.deleted_at IS NULL
             ORDER BY e.occurred_on DESC",
            libsql::params![loan_id.to_string()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(crate::models::MetaAbonoV2 { id: row.get(0)?, date: row.get(1)?, amount: row.get(2)? });
    }
    Ok(out)
}

pub async fn total_pending(conn: &Connection) -> AppResult<i64> {
    let loans = list(conn).await?;
    let mut total = 0i64;
    for loan in loans {
        let p = paid(conn, &loan.id).await?;
        total += (loan.principal_cop - p).max(0);
    }
    Ok(total)
}
