use crate::error::AppResult;
use crate::models::{Meta, MetaAbono};
use crate::repositories::{goals as goals_repo, transactions as tx_repo};
use crate::services::{goals as goals_svc, loans as loans_svc};

pub async fn list(conn: &libsql::Connection) -> AppResult<Vec<Meta>> {
    let mut result: Vec<Meta> = Vec::new();

    // Loans → tipo "me_deben"
    let loans = loans_svc::list(conn).await?;
    for lb in loans {
        let estado = if lb.loan.status == "pagado" { "completado" } else { "pendiente" };
        let abonos: Vec<MetaAbono> = lb.payments.into_iter().map(|p| MetaAbono {
            id: p.id,
            date: p.date,
            amount: p.amount,
        }).collect();
        result.push(Meta {
            id: format!("loan:{}", lb.loan.id),
            tipo: "me_deben".to_string(),
            nombre: lb.loan.person_name,
            total: lb.loan.amount,
            abonado: lb.paid,
            pendiente: lb.pending,
            estado: estado.to_string(),
            fecha: Some(lb.loan.date),
            nota: lb.loan.note,
            cuotas: None,
            abonos,
        });
    }

    // Goals → tipo "debo" (is_debt_goal=1) or "quiero_juntar" (is_debt_goal=0)
    let goals = goals_repo::list(conn, None).await?;
    for goal in goals {
        let progress = goals_svc::build_progress(conn, goal).await?;
        let all_txs = tx_repo::list_by_goal(conn, progress.goal.id).await?;
        let abonos: Vec<MetaAbono> = all_txs.into_iter()
            .filter(|tx| !(tx.is_debt && tx.kind == "gasto"))
            .map(|tx| MetaAbono { id: tx.id, date: tx.date, amount: tx.amount })
            .collect();

        let tipo = if progress.goal.is_debt_goal { "debo" } else { "quiero_juntar" };
        let estado = if progress.goal.status == "completado" { "completado" } else { "pendiente" };
        let pendiente = (progress.goal.target_amount - progress.current_amount).max(0);

        result.push(Meta {
            id: format!("goal:{}", progress.goal.id),
            tipo: tipo.to_string(),
            nombre: progress.goal.name,
            total: progress.goal.target_amount,
            abonado: progress.current_amount,
            pendiente,
            estado: estado.to_string(),
            fecha: progress.goal.target_date,
            nota: None,
            cuotas: progress.goal.installments,
            abonos,
        });
    }

    Ok(result)
}
