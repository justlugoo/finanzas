use crate::error::{AppError, AppResult};
use crate::models::{LoanInput, LoanPaymentInput, LoanUpdateInput, LoanWithBalance};
use crate::repositories::loans as repo;

pub async fn create(conn: &libsql::Connection, input: LoanInput) -> AppResult<LoanWithBalance> {
    if input.person_name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre del deudor no puede estar vacío".into()));
    }
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    if input.date.trim().is_empty() {
        return Err(AppError::ValidationError("la fecha no puede estar vacía".into()));
    }
    let input = LoanInput { person_name: input.person_name.trim().to_string(), ..input };
    let loan = repo::insert(conn, &input).await?;
    let amount = loan.amount;
    Ok(LoanWithBalance { loan, paid: 0, pending: amount, payments: vec![] })
}

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<LoanWithBalance>> {
    repo::list(conn).await
}

pub async fn get(conn: &libsql::Connection, id: i64) -> AppResult<LoanWithBalance> {
    repo::get(conn, id).await
}

pub async fn add_payment(
    conn: &libsql::Connection,
    input: LoanPaymentInput,
) -> AppResult<LoanWithBalance> {
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto del abono debe ser mayor que 0".into()));
    }
    if input.date.trim().is_empty() {
        return Err(AppError::ValidationError("la fecha no puede estar vacía".into()));
    }
    let current = repo::get(conn, input.loan_id).await?;
    if current.paid + input.amount > current.loan.amount {
        return Err(AppError::ValidationError(format!(
            "el abono ({}) supera el saldo pendiente ({}) del préstamo",
            input.amount, current.pending
        )));
    }
    repo::add_payment(conn, &input).await
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    input: LoanUpdateInput,
) -> AppResult<LoanWithBalance> {
    if input.person_name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre del deudor no puede estar vacío".into()));
    }
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    let current = repo::get(conn, id).await?;
    if input.amount < current.paid {
        return Err(AppError::ValidationError(format!(
            "el nuevo monto ({}) no puede ser menor que lo ya abonado ({})",
            input.amount, current.paid
        )));
    }
    repo::update(conn, id, input.person_name.trim(), input.amount).await
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    let affected = repo::delete(conn, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("préstamo {id} no existe")));
    }
    Ok(())
}

pub async fn total_pending(conn: &libsql::Connection) -> AppResult<i64> {
    repo::total_pending(conn).await
}
