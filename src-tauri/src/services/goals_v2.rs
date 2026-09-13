use crate::error::{AppError, AppResult};
use crate::models::{DebtGoalInput, DebtGoalResult, EntryInput, GoalDetailV2, GoalInputV2, GoalV2, GoalWithProgressV2};
use crate::repositories;
use libsql::Connection;

fn validate(input: &GoalInputV2) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_cop <= 0 {
        return Err(AppError::ValidationError("el monto objetivo debe ser mayor que 0".into()));
    }
    if !matches!(input.kind.as_str(), "saving" | "debt") {
        return Err(AppError::ValidationError("type debe ser 'saving' o 'debt'".into()));
    }
    Ok(())
}

pub async fn build_progress(conn: &Connection, goal: GoalV2) -> AppResult<GoalWithProgressV2> {
    let current_amount = repositories::goals_v2::current_amount(conn, &goal.id, &goal.kind).await?;
    let pending = (goal.target_cop - current_amount).max(0);
    let percentage = if goal.target_cop > 0 { (current_amount as f64 / goal.target_cop as f64 * 100.0).min(100.0) } else { 0.0 };
    Ok(GoalWithProgressV2 { goal, current_amount, pending, percentage })
}

pub async fn list(conn: &Connection, kind: Option<&str>) -> AppResult<Vec<GoalWithProgressV2>> {
    let goals = repositories::goals_v2::list(conn, kind).await?;
    let mut out = Vec::new();
    for g in goals {
        out.push(build_progress(conn, g).await?);
    }
    Ok(out)
}

pub async fn create(conn: &Connection, input: GoalInputV2) -> AppResult<GoalWithProgressV2> {
    validate(&input)?;
    let goal = repositories::goals_v2::insert(
        conn, input.name.trim(), input.target_cop, input.target_date.as_deref(), &input.kind, input.installments,
    )
    .await?;
    build_progress(conn, goal).await
}

/// Si es una deuda, cambiar el monto también ajusta el `expense` original
/// que la representa (igual motivo que `loans_v2::update`: si no, el
/// patrimonio quedaría calculado sobre un monto que ya no coincide).
pub async fn update(conn: &Connection, id: &str, input: GoalInputV2) -> AppResult<GoalWithProgressV2> {
    validate(&input)?;
    let goal = repositories::goals_v2::update(
        conn, id, input.name.trim(), input.target_cop, input.target_date.as_deref(), input.installments,
    )
    .await?;

    if goal.kind == "debt"
        && let Some(entry_id) = repositories::goals_v2::debt_opening_entry_id(conn, id).await? {
            let entry = repositories::entries::get_by_id(conn, &entry_id).await?;
            repositories::entries::update(conn, &entry_id, &entry.occurred_on, input.target_cop, entry.note.as_deref(), entry.is_extraordinary, entry.category_id.as_deref()).await?;
        }

    build_progress(conn, goal).await
}

/// Borrar la meta también borra (lógicamente) sus movimientos — igual
/// criterio que préstamos: nada de transferencias/gastos huérfanos.
pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::goals_v2::soft_delete_entries(conn, id).await?;
    repositories::goals_v2::soft_delete(conn, id).await
}

/// Crea una deuda de una vez: el `goal` (kind=debt) y el `expense` real
/// desde `payable` en la misma transacción — "Comprar a crédito" es un
/// gasto real desde el día uno (sección 4 de schema-v2.md), no algo que
/// se registra solo si el disponible no alcanza. Centralizado en Metas.
pub async fn create_debt(conn: &Connection, input: DebtGoalInput) -> AppResult<DebtGoalResult> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_cop <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }

    conn.execute_batch("BEGIN;")
        .await
        .map_err(|e| AppError::DatabaseError(format!("create_debt (begin): {e}")))?;

    let result: AppResult<(GoalV2, crate::models::Entry)> = async {
        let goal = repositories::goals_v2::insert(
            conn, input.name.trim(), input.target_cop, None, "debt", input.installments,
        )
        .await?;

        let payable = repositories::accounts::id_by_code(conn, "payable").await?;
        let entry = crate::services::entries::create(
            conn,
            EntryInput {
                occurred_on: input.occurred_on.clone(),
                kind: "expense".to_string(),
                amount_cop: input.target_cop,
                account_from: Some(payable),
                account_to: None,
                category_id: Some(input.category_id.clone()),
                goal_id: Some(goal.id.clone()),
                loan_id: None,
                note: input.note.clone(),
                is_extraordinary: false,
            },
        )
        .await?;

        Ok((goal, entry))
    }
    .await;

    match result {
        Ok((goal, entry)) => {
            conn.execute_batch("COMMIT;")
                .await
                .map_err(|e| AppError::DatabaseError(format!("create_debt (commit): {e}")))?;
            let goal_with_progress = build_progress(conn, goal).await?;
            Ok(DebtGoalResult { goal: goal_with_progress, entry })
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;").await;
            Err(e)
        }
    }
}

pub async fn get_detail(conn: &Connection, id: &str) -> AppResult<GoalDetailV2> {
    let goal = repositories::goals_v2::get(conn, id).await?;
    let goal_with_progress = build_progress(conn, goal).await?;
    let contributions = repositories::goals_v2::contributions(conn, id).await?;
    Ok(GoalDetailV2 { goal: goal_with_progress, contributions })
}
