use chrono::{Local, Datelike, NaiveDate};
use crate::error::{AppError, AppResult};
use crate::models::{Goal, GoalDetail, GoalInput, GoalWithProgress};
use crate::repositories::{goals as repo, transactions as tx_repo};

pub async fn build_progress(
    conn: &libsql::Connection,
    goal: Goal,
) -> AppResult<GoalWithProgress> {
    let current_amount = tx_repo::sum_by_goal(conn, goal.id).await?;
    let sum_3m         = tx_repo::sum_by_goal_recent(conn, goal.id).await?;

    let percentage = if goal.target_amount > 0 {
        (current_amount as f64 / goal.target_amount as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let avg_monthly = sum_3m as f64 / 3.0;
    let today = Local::now().date_naive();

    let monthly_required: Option<f64> = if current_amount < goal.target_amount {
        goal.target_date.as_deref().and_then(|td| {
            NaiveDate::parse_from_str(td, "%Y-%m-%d").ok().and_then(|target_date| {
                let months = (target_date.year() - today.year()) * 12
                    + (target_date.month() as i32 - today.month() as i32);
                if months > 0 {
                    Some((goal.target_amount - current_amount) as f64 / months as f64)
                } else {
                    None
                }
            })
        })
    } else {
        None
    };

    let projected_completion_date: Option<String> = if current_amount >= goal.target_amount {
        None
    } else if avg_monthly > 0.0 {
        let remaining    = (goal.target_amount - current_amount) as f64;
        let months_needed = (remaining / avg_monthly).ceil() as i32;
        let raw_month    = today.month() as i32 + months_needed;
        let years_add    = (raw_month - 1) / 12;
        let final_month  = ((raw_month - 1) % 12 + 1) as u32;
        let final_year   = today.year() + years_add;
        NaiveDate::from_ymd_opt(final_year, final_month, 1)
            .map(|d| d.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    let on_track = match (&goal.target_date, &projected_completion_date) {
        (Some(td), Some(pcd)) => pcd <= td,
        _ => true,
    };

    Ok(GoalWithProgress { goal, current_amount, percentage, monthly_required, projected_completion_date, on_track })
}

pub async fn list(
    conn: &libsql::Connection,
    status: Option<String>,
) -> AppResult<Vec<GoalWithProgress>> {
    let goals = repo::list(conn, status.as_deref()).await?;
    let mut result = Vec::new();
    for goal in goals {
        result.push(build_progress(conn, goal).await?);
    }
    Ok(result)
}

pub async fn create(
    conn: &libsql::Connection,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_amount <= 0 {
        return Err(AppError::ValidationError("el monto objetivo debe ser mayor que 0".into()));
    }
    let id = repo::insert(conn, input.name.trim(), input.target_amount, input.target_date.as_deref()).await?;
    let goal = repo::find_by_id(conn, id).await?
        .ok_or_else(|| AppError::NotFound("objetivo recién creado no encontrado".into()))?;
    build_progress(conn, goal).await
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    input: GoalInput,
) -> AppResult<GoalWithProgress> {
    if input.name.trim().is_empty() {
        return Err(AppError::ValidationError("el nombre no puede estar vacío".into()));
    }
    if input.target_amount <= 0 {
        return Err(AppError::ValidationError("el monto objetivo debe ser mayor que 0".into()));
    }
    // Auto-derive status from actual contribution sum; only honour an explicit
    // valid value passed from callers that still manage status manually.
    let current_amount = tx_repo::sum_by_goal(conn, id).await?;
    let status = match input.status.as_deref() {
        Some(s) if matches!(s, "activo" | "completado" | "pausado") => s.to_string(),
        _ => if current_amount >= input.target_amount {
            "completado".to_string()
        } else {
            "activo".to_string()
        },
    };

    let affected = repo::update(conn, id, input.name.trim(), input.target_amount, input.target_date.as_deref(), &status).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("objetivo {id} no existe")));
    }

    let goal = repo::find_by_id(conn, id).await?
        .ok_or_else(|| AppError::NotFound(format!("objetivo {id} no existe")))?;
    build_progress(conn, goal).await
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    let affected = repo::delete(conn, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("objetivo {id} no existe")));
    }
    Ok(())
}

pub async fn get_detail(conn: &libsql::Connection, id: i64) -> AppResult<GoalDetail> {
    let goal = repo::find_by_id(conn, id).await?
        .ok_or_else(|| AppError::NotFound(format!("objetivo {id} no existe")))?;
    let goal_with_progress = build_progress(conn, goal).await?;
    let contributions = tx_repo::list_by_goal(conn, id).await?;
    Ok(GoalDetail { goal: goal_with_progress, contributions })
}
