use crate::error::AppResult;
use crate::models::MetaV2;
use crate::repositories;
use crate::services;
use libsql::Connection;

pub async fn list(conn: &Connection) -> AppResult<Vec<MetaV2>> {
    let mut result = Vec::new();

    for lb in services::loans_v2::list(conn).await? {
        let abonos = repositories::loans_v2::payments(conn, &lb.loan.id).await?;
        result.push(MetaV2 {
            id: format!("loan:{}", lb.loan.id),
            tipo: "me_deben".to_string(),
            nombre: lb.loan.person_name,
            total: lb.loan.principal_cop,
            abonado: lb.paid,
            pendiente: lb.pending,
            estado: if lb.status == "pagado" { "completado" } else { "pendiente" }.to_string(),
            fecha: Some(lb.loan.lent_on),
            nota: lb.loan.note,
            cuotas: None,
            abonos,
        });
    }

    for goal in repositories::goals_v2::list(conn, None).await? {
        let progress = services::goals_v2::build_progress(conn, goal).await?;
        let abonos = repositories::goals_v2::abonos(conn, &progress.goal.id, &progress.goal.kind).await?;
        let tipo = if progress.goal.kind == "debt" { "debo" } else { "quiero_juntar" };
        result.push(MetaV2 {
            id: format!("goal:{}", progress.goal.id),
            tipo: tipo.to_string(),
            nombre: progress.goal.name,
            total: progress.goal.target_cop,
            abonado: progress.current_amount,
            pendiente: progress.pending,
            estado: if progress.pending == 0 { "completado" } else { "pendiente" }.to_string(),
            fecha: progress.goal.target_date,
            nota: None,
            cuotas: progress.goal.installments,
            abonos,
        });
    }

    Ok(result)
}
