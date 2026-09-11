//! Paso 6 de docs/schema-v2.md §12: un solo camino de abono para los tres
//! tipos de meta (`me_deben` / `debo` / `quiero_juntar`). El frontend deja
//! de decidir contabilidad — solo manda `meta_id` (`"loan:{id}"` o
//! `"goal:{id}"`), monto y fecha; este servicio decide el `transfer`
//! correcto según el tipo.

use crate::error::{AppError, AppResult};
use crate::models::{Entry, EntryInput, MetaPaymentInput};
use crate::repositories;
use crate::services;
use libsql::Connection;

pub async fn add_payment_input(conn: &Connection, input: MetaPaymentInput) -> AppResult<Entry> {
    add_payment(conn, &input.meta_id, input.amount_cop, &input.occurred_on, input.note.as_deref()).await
}

pub async fn add_payment(
    conn: &Connection,
    meta_id: &str,
    amount_cop: i64,
    occurred_on: &str,
    note: Option<&str>,
) -> AppResult<Entry> {
    let cash = repositories::accounts::id_by_code(conn, "cash").await?;

    if let Some(loan_id) = meta_id.strip_prefix("loan:") {
        // "Me deben" — cobrar un préstamo: transfer receivable -> cash.
        let receivable = repositories::accounts::id_by_code(conn, "receivable").await?;
        return services::entries::create(
            conn,
            EntryInput {
                occurred_on: occurred_on.to_string(),
                kind: "transfer".to_string(),
                amount_cop,
                account_from: Some(receivable),
                account_to: Some(cash),
                category_id: None,
                goal_id: None,
                loan_id: Some(loan_id.to_string()),
                note: note.map(String::from),
                is_extraordinary: false,
            },
        )
        .await;
    }

    if let Some(goal_id) = meta_id.strip_prefix("goal:") {
        let kind = repositories::goals_v2::kind_by_id(conn, goal_id).await?;
        // "Debo" (kind=debt) — abonar una deuda: transfer cash -> payable.
        // "Quiero juntar" (kind=saving) — aportar a un ahorro: transfer cash -> savings.
        let to_code = if kind == "debt" { "payable" } else { "savings" };
        let to = repositories::accounts::id_by_code(conn, to_code).await?;
        return services::entries::create(
            conn,
            EntryInput {
                occurred_on: occurred_on.to_string(),
                kind: "transfer".to_string(),
                amount_cop,
                account_from: Some(cash),
                account_to: Some(to),
                category_id: None,
                goal_id: Some(goal_id.to_string()),
                loan_id: None,
                note: note.map(String::from),
                is_extraordinary: false,
            },
        )
        .await;
    }

    Err(AppError::ValidationError(format!("meta_id inválido, esperaba \"loan:<id>\" o \"goal:<id>\": {meta_id}")))
}
