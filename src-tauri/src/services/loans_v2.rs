use crate::error::{AppError, AppResult};
use crate::models::{EntryInput, LoanInputV2, LoanV2, LoanWithBalanceV2};
use crate::repositories;
use libsql::Connection;

async fn with_balance(conn: &Connection, loan: LoanV2) -> AppResult<LoanWithBalanceV2> {
    let paid = repositories::loans_v2::paid(conn, &loan.id).await?;
    let pending = (loan.principal_cop - paid).max(0);
    let status = if pending == 0 { "pagado" } else { "pendiente" }.to_string();
    Ok(LoanWithBalanceV2 { loan, paid, pending, status })
}

/// Crea el préstamo **y** el movimiento real de plata (`transfer
/// cash->receivable`) en la misma transacción — sección 4 de
/// schema-v2.md: "Prestar plata" es un `transfer`, no una fila aislada en
/// `loans`. Sin esto el disponible nunca bajaba al prestar (bug reportado
/// por el usuario, 2026-09-10).
pub async fn create(conn: &Connection, input: LoanInputV2) -> AppResult<LoanWithBalanceV2> {
    if input.person_name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre del deudor no puede estar vacío".into()));
    }
    if input.principal_cop <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }

    conn.execute_batch("BEGIN;")
        .await
        .map_err(|e| AppError::DatabaseError(format!("loan create (begin): {e}")))?;

    let result: AppResult<LoanV2> = async {
        let loan = repositories::loans_v2::insert(
            conn, input.person_name.trim(), input.principal_cop, &input.lent_on, input.note.as_deref(),
        )
        .await?;

        let cash = repositories::accounts::id_by_code(conn, "cash").await?;
        let receivable = repositories::accounts::id_by_code(conn, "receivable").await?;
        crate::services::entries::create(
            conn,
            EntryInput {
                occurred_on: input.lent_on.clone(),
                kind: "transfer".to_string(),
                amount_cop: input.principal_cop,
                account_from: Some(cash),
                account_to: Some(receivable),
                category_id: None,
                goal_id: None,
                loan_id: Some(loan.id.clone()),
                note: input.note.clone(),
                is_extraordinary: false,
            },
        )
        .await?;

        Ok(loan)
    }
    .await;

    match result {
        Ok(loan) => {
            conn.execute_batch("COMMIT;")
                .await
                .map_err(|e| AppError::DatabaseError(format!("loan create (commit): {e}")))?;
            with_balance(conn, loan).await
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;").await;
            Err(e)
        }
    }
}

pub async fn list(conn: &Connection) -> AppResult<Vec<LoanWithBalanceV2>> {
    let loans = repositories::loans_v2::list(conn).await?;
    let mut out = Vec::new();
    for l in loans {
        out.push(with_balance(conn, l).await?);
    }
    Ok(out)
}

pub async fn get(conn: &Connection, id: &str) -> AppResult<LoanWithBalanceV2> {
    let loan = repositories::loans_v2::get(conn, id).await?;
    with_balance(conn, loan).await
}

/// Cambiar el monto del préstamo también ajusta el `transfer` original que
/// lo representa — si no, el disponible/patrimonio quedarían calculados
/// sobre un monto que ya no coincide con lo que muestra la meta.
pub async fn update(conn: &Connection, id: &str, person_name: &str, principal_cop: i64) -> AppResult<LoanWithBalanceV2> {
    if person_name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre del deudor no puede estar vacío".into()));
    }
    if principal_cop <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    let current = get(conn, id).await?;
    if principal_cop < current.paid {
        return Err(AppError::ValidationError(format!(
            "el nuevo monto ({principal_cop}) no puede ser menor que lo ya cobrado ({})",
            current.paid
        )));
    }

    let loan = repositories::loans_v2::update(conn, id, person_name.trim(), principal_cop).await?;

    if let Some(entry_id) = repositories::loans_v2::lending_entry_id(conn, id).await? {
        let entry = repositories::entries::get_by_id(conn, &entry_id).await?;
        repositories::entries::update(conn, &entry_id, &entry.occurred_on, principal_cop, entry.note.as_deref(), entry.is_extraordinary, entry.category_id.as_deref()).await?;
    }

    with_balance(conn, loan).await
}

/// Borrar el préstamo también borra (lógicamente) el movimiento de plata
/// que representó y todos sus abonos — si no, quedarían transferencias
/// huérfanas afectando el patrimonio de un préstamo que ya no existe.
pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::loans_v2::soft_delete_entries(conn, id).await?;
    repositories::loans_v2::soft_delete(conn, id).await
}

pub async fn total_pending(conn: &Connection) -> AppResult<i64> {
    repositories::loans_v2::total_pending(conn).await
}
