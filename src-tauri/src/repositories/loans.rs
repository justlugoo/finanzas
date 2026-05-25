use crate::error::{AppError, AppResult};
use crate::models::{Loan, LoanInput, LoanPayment, LoanPaymentInput, LoanWithBalance};

pub async fn insert(conn: &libsql::Connection, input: &LoanInput) -> AppResult<Loan> {
    conn.execute(
        "INSERT INTO loans (person_name, amount, date, note) VALUES (?, ?, ?, ?)",
        libsql::params![
            input.person_name.clone(),
            input.amount,
            input.date.clone(),
            input.note.clone()
        ],
    ).await?;
    let id = conn.last_insert_rowid();
    find_loan(conn, id).await?
        .ok_or_else(|| AppError::NotFound("préstamo recién creado no encontrado".into()))
}

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<LoanWithBalance>> {
    let mut rows = conn.query(
        "SELECT id, person_name, amount, date, note, status, created_at \
         FROM loans ORDER BY date DESC",
        (),
    ).await?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().await? {
        let loan = row_to_loan(&row)?;
        let payments = get_payments(conn, loan.id).await?;
        result.push(build_balance(loan, payments));
    }
    Ok(result)
}

pub async fn get(conn: &libsql::Connection, id: i64) -> AppResult<LoanWithBalance> {
    let loan = find_loan(conn, id).await?
        .ok_or_else(|| AppError::NotFound(format!("préstamo {id} no existe")))?;
    let payments = get_payments(conn, id).await?;
    Ok(build_balance(loan, payments))
}

pub async fn add_payment(
    conn: &libsql::Connection,
    input: &LoanPaymentInput,
) -> AppResult<LoanWithBalance> {
    conn.execute(
        "INSERT INTO loan_payments (loan_id, amount, date) VALUES (?, ?, ?)",
        libsql::params![input.loan_id, input.amount, input.date.clone()],
    ).await?;

    // Suma total de abonos tras la inserción
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(amount), 0) FROM loan_payments WHERE loan_id = ?",
        libsql::params![input.loan_id],
    ).await?;
    let total_paid: i64 = rows.next().await?
        .map(|r| r.get(0).unwrap_or(0))
        .unwrap_or(0);

    // Monto original del préstamo
    let mut rows = conn.query(
        "SELECT amount FROM loans WHERE id = ?",
        libsql::params![input.loan_id],
    ).await?;
    let loan_amount: i64 = rows.next().await?
        .ok_or_else(|| AppError::NotFound(format!("préstamo {} no existe", input.loan_id)))?
        .get(0)?;

    // Transición de estado: pagado si la suma cubre el monto original
    let new_status = if total_paid >= loan_amount { "pagado" } else { "pendiente" };
    conn.execute(
        "UPDATE loans SET status = ? WHERE id = ?",
        libsql::params![new_status.to_string(), input.loan_id],
    ).await?;

    get(conn, input.loan_id).await
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<u64> {
    conn.execute(
        "DELETE FROM loan_payments WHERE loan_id = ?",
        libsql::params![id],
    ).await?;
    let affected = conn.execute(
        "DELETE FROM loans WHERE id = ?",
        libsql::params![id],
    ).await?;
    Ok(affected)
}

pub async fn total_pending(conn: &libsql::Connection) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(l.amount - COALESCE(p.total_paid, 0)), 0) \
         FROM loans l \
         LEFT JOIN ( \
             SELECT loan_id, SUM(amount) AS total_paid \
             FROM loan_payments GROUP BY loan_id \
         ) p ON p.loan_id = l.id \
         WHERE l.status = 'pendiente'",
        (),
    ).await?;
    Ok(rows.next().await?.map(|r| r.get(0).unwrap_or(0)).unwrap_or(0))
}

async fn find_loan(conn: &libsql::Connection, id: i64) -> AppResult<Option<Loan>> {
    let mut rows = conn.query(
        "SELECT id, person_name, amount, date, note, status, created_at \
         FROM loans WHERE id = ?",
        libsql::params![id],
    ).await?;
    rows.next().await?.map(|r| row_to_loan(&r)).transpose()
}

async fn get_payments(conn: &libsql::Connection, loan_id: i64) -> AppResult<Vec<LoanPayment>> {
    let mut rows = conn.query(
        "SELECT id, loan_id, amount, date, created_at \
         FROM loan_payments WHERE loan_id = ? ORDER BY date DESC",
        libsql::params![loan_id],
    ).await?;
    let mut payments = Vec::new();
    while let Some(row) = rows.next().await? {
        payments.push(row_to_payment(&row)?);
    }
    Ok(payments)
}

fn build_balance(loan: Loan, payments: Vec<LoanPayment>) -> LoanWithBalance {
    let paid: i64 = payments.iter().map(|p| p.amount).sum();
    let pending = (loan.amount - paid).max(0);
    LoanWithBalance { loan, paid, pending, payments }
}

fn row_to_loan(row: &libsql::Row) -> AppResult<Loan> {
    Ok(Loan {
        id:          row.get(0)?,
        person_name: row.get(1)?,
        amount:      row.get(2)?,
        date:        row.get(3)?,
        note:        row.get(4)?,
        status:      row.get(5)?,
        created_at:  row.get(6)?,
    })
}

fn row_to_payment(row: &libsql::Row) -> AppResult<LoanPayment> {
    Ok(LoanPayment {
        id:         row.get(0)?,
        loan_id:    row.get(1)?,
        amount:     row.get(2)?,
        date:       row.get(3)?,
        created_at: row.get(4)?,
    })
}
