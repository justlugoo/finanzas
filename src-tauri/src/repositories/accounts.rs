use crate::error::{AppError, AppResult};
use crate::models::{Account, AccountBalances};
use libsql::Connection;

pub fn row_to_account(row: &libsql::Row) -> Result<Account, libsql::Error> {
    Ok(Account {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        kind: row.get(3)?,
        is_system: row.get::<i64>(4)? != 0,
    })
}

pub async fn list(conn: &Connection) -> AppResult<Vec<Account>> {
    let mut rows = conn
        .query("SELECT id, code, name, kind, is_system FROM accounts ORDER BY code", ())
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_account(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn id_by_code(conn: &Connection, code: &str) -> AppResult<String> {
    let mut rows = conn
        .query("SELECT id FROM accounts WHERE code = ?", libsql::params![code.to_string()])
        .await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::NotFound(format!("cuenta '{code}' no existe")))?;
    Ok(row.get(0)?)
}

/// `saldo(cuenta) = Σ(amount_cop donde account_to = cuenta) − Σ(amount_cop donde account_from = cuenta)`,
/// sobre entries activas (`deleted_at IS NULL`). Ver sección 3 de schema-v2.md.
async fn balance_by_code(conn: &Connection, code: &str) -> AppResult<i64> {
    let mut rows = conn
        .query(
            "SELECT
                COALESCE((SELECT SUM(e.amount_cop) FROM entries e JOIN accounts a ON a.id = e.account_to
                          WHERE a.code = ?1 AND e.deleted_at IS NULL), 0)
                -
                COALESCE((SELECT SUM(e.amount_cop) FROM entries e JOIN accounts a ON a.id = e.account_from
                          WHERE a.code = ?1 AND e.deleted_at IS NULL), 0)",
            libsql::params![code.to_string()],
        )
        .await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("balance sin resultados".into()))?;
    Ok(row.get(0)?)
}

/// Saldos de las 5 cuentas de sistema y los totales derivados
/// (`disponible`, `patrimonio`). `payable` se devuelve invertido (positivo
/// = monto de la deuda), como manda la sección 3 del documento. `apps`
/// (plata de Didi/Uber sin retirar) suma al patrimonio pero no al disponible.
pub async fn balances(conn: &Connection) -> AppResult<AccountBalances> {
    let cash = balance_by_code(conn, "cash").await?;
    let apps = balance_by_code(conn, "apps").await?;
    let savings = balance_by_code(conn, "savings").await?;
    let receivable = balance_by_code(conn, "receivable").await?;
    let payable_raw = balance_by_code(conn, "payable").await?;

    Ok(AccountBalances {
        cash,
        apps,
        savings,
        receivable,
        payable: -payable_raw,
        disponible: cash,
        patrimonio: cash + apps + savings + receivable + payable_raw,
    })
}
