use chrono::{Local, Datelike, NaiveDate};
use crate::error::{AppError, AppResult};
use crate::models::{
    CategoryProgress, CsvExport, CurrentBalance, ImportResult, MonthComparison,
    Period, PeriodSummary, Transaction, TransactionFilter, TransactionInput, TransactionPage,
};
use crate::repositories::{budgets as budgets_repo, goals as goals_repo, transactions as tx_repo};
use crate::utils::{csv_escape, format_cop_simple, is_valid_date, parse_csv_line, period_to_dates, scale_monthly, send_notification};

pub async fn create(
    conn: &libsql::Connection,
    app: &tauri::AppHandle,
    input: TransactionInput,
) -> AppResult<Transaction> {
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    if !matches!(input.kind.as_str(), "ingreso" | "gasto") {
        return Err(AppError::ValidationError("tipo debe ser 'ingreso' o 'gasto'".into()));
    }

    let gas_km_val = input.gas_km.unwrap_or(0.0);
    let auto_gas   = gas_km_val > 0.0;

    let vehicle_id = if auto_gas {
        match input.vehicle_id {
            Some(id) => id,
            None => return Err(AppError::ValidationError(
                "selecciona un vehículo para calcular el gasto de gasolina".into(),
            )),
        }
    } else { 0 };

    if auto_gas {
        conn.execute("BEGIN", ()).await?;
    }

    let tx = {
        match tx_repo::insert(conn, &input).await {
            Ok(t) => t,
            Err(e) => {
                if auto_gas { let _ = conn.execute("ROLLBACK", ()).await; }
                return Err(e);
            }
        }
    };

    if auto_gas {
        if let Err(e) = crate::services::gas::insert_auto(conn, &input.date, &input.category, gas_km_val, vehicle_id).await {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(e);
        }
        if let Err(e) = conn.execute("COMMIT", ()).await {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(AppError::DatabaseError(e.to_string()));
        }
    }

    // Auto-crear objetivo de deuda
    if input.is_debt && input.kind == "gasto" {
        let debt_name = match &input.note {
            Some(n) if !n.trim().is_empty() => format!("Deuda: {}", n.trim()),
            _ => format!("Deuda: {}", input.category),
        };
        let _ = goals_repo::insert_debt_goal(conn, &debt_name, input.amount).await;
    }

    // Trigger: presupuesto excedido
    if input.kind == "gasto" {
        let today = Local::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .unwrap().format("%Y-%m-%d").to_string();
        let today_str = today.format("%Y-%m-%d").to_string();

        let spent = tx_repo::spent_in_category(conn, &input.category, &month_start, &today_str)
            .await.unwrap_or(0);
        let limit = budgets_repo::get_monthly_limit(conn, &input.category)
            .await.unwrap_or(0);

        if limit > 0 && spent > limit {
            send_notification(
                app,
                "⚠ Presupuesto excedido",
                &format!(
                    "Llevas {} en {} — límite mensual: {}",
                    format_cop_simple(spent),
                    input.category,
                    format_cop_simple(limit),
                ),
            );
        }
    }

    // Trigger: objetivo completado
    if let Some(gid) = input.goal_id
        && let Ok(Some((goal_name, target))) = goals_repo::find_active_by_id(conn, gid).await
            && target > 0 {
                let current = tx_repo::sum_for_goal_completion(conn, gid).await.unwrap_or(0);
                if current >= target {
                    let _ = goals_repo::mark_completed(conn, gid).await;
                    send_notification(
                        app,
                        "🎯 Objetivo completado",
                        &format!("¡Alcanzaste tu meta: {}!", goal_name),
                    );
                }
            }

    Ok(tx)
}

pub async fn list(
    conn: &libsql::Connection,
    filter: TransactionFilter,
) -> AppResult<TransactionPage> {
    tx_repo::list(conn, &filter).await
}

pub async fn get_balance(conn: &libsql::Connection) -> AppResult<CurrentBalance> {
    tx_repo::get_balance(conn).await
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    input: TransactionInput,
) -> AppResult<Transaction> {
    if input.amount <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    if !matches!(input.kind.as_str(), "ingreso" | "gasto") {
        return Err(AppError::ValidationError("tipo debe ser 'ingreso' o 'gasto'".into()));
    }
    tx_repo::update(conn, id, &input).await
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    tx_repo::delete(conn, id).await
}

pub async fn get_period_summary(
    conn: &libsql::Connection,
    period: Period,
) -> AppResult<PeriodSummary> {
    let (start, end) = period_to_dates(&period);
    tx_repo::get_period_summary(conn, &start, &end).await
}

pub async fn get_category_progress(
    conn: &libsql::Connection,
    period: Period,
) -> AppResult<Vec<CategoryProgress>> {
    let (start, end) = period_to_dates(&period);
    let rows = tx_repo::list_category_budget_rows(conn, &start, &end).await?;

    let progress = rows.into_iter().map(|raw| {
        let monthly_target = scale_monthly(raw.monthly_amount, &period);
        let percentage = if monthly_target > 0 {
            (raw.current_amount as f64 / monthly_target as f64) * 100.0
        } else {
            0.0
        };
        CategoryProgress {
            is_over: monthly_target > 0 && raw.current_amount > monthly_target,
            category: raw.category,
            monthly_target,
            current_amount: raw.current_amount,
            percentage,
            kind: raw.kind,
            is_fixed: raw.is_fixed,
        }
    }).collect();

    Ok(progress)
}

pub async fn get_month_comparison(conn: &libsql::Connection) -> AppResult<MonthComparison> {
    tx_repo::get_month_comparison(conn).await
}

pub async fn list_categories(
    conn: &libsql::Connection,
    kind: Option<String>,
) -> AppResult<Vec<String>> {
    let mut cats: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    match &kind {
        None => {
            for c in budgets_repo::list_all_categories(conn).await? { cats.insert(c); }
            for c in tx_repo::list_all_distinct_categories(conn).await? { cats.insert(c); }
        }
        Some(k) => {
            for c in budgets_repo::list_categories_by_kind(conn, k).await? { cats.insert(c); }
            for c in tx_repo::list_distinct_categories_by_kind(conn, k).await? { cats.insert(c); }
        }
    }

    Ok(cats.into_iter().collect())
}

pub async fn export_csv(
    conn: &libsql::Connection,
    filter: TransactionFilter,
) -> AppResult<CsvExport> {
    let rows = tx_repo::list_for_export(conn, &filter).await?;
    let mut csv = String::from(
        "ID,Fecha,Tipo,Categoría,Monto (COP),Nota,Extraordinario,ID Objetivo,Creado en\n",
    );
    for tx in &rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            tx.id,
            tx.date,
            tx.kind,
            csv_escape(&tx.category),
            tx.amount,
            csv_escape(tx.note.as_deref().unwrap_or("")),
            if tx.is_extraordinary { "Sí" } else { "No" },
            tx.goal_id.map(|id| id.to_string()).unwrap_or_default(),
            tx.created_at,
        ));
    }
    let today = Local::now().format("%Y-%m-%d").to_string();
    Ok(CsvExport { content: csv, suggested_filename: format!("transacciones_{today}.csv") })
}

pub async fn import_csv(
    conn: &libsql::Connection,
    csv_content: String,
) -> AppResult<ImportResult> {
    let mut valid_cats: std::collections::HashSet<String> = std::collections::HashSet::new();
    for c in budgets_repo::list_all_categories(conn).await? { valid_cats.insert(c); }

    let mut imported = 0i64;
    let mut skipped  = 0i64;
    let mut errors: Vec<String> = Vec::new();

    let mut lines = csv_content.lines();
    lines.next(); // saltar encabezado

    for (i, line) in lines.enumerate() {
        let row_num = i + 2;
        if line.trim().is_empty() { continue; }

        let fields = parse_csv_line(line);
        let offset = if fields[0].trim().parse::<i64>().is_ok() { 1usize } else { 0 };

        if fields.len() < offset + 4 {
            skipped += 1;
            errors.push(format!("Fila {row_num}: columnas insuficientes"));
            continue;
        }

        let date = fields[offset].trim().to_string();
        if !is_valid_date(&date) {
            skipped += 1;
            errors.push(format!("Fila {row_num}: fecha inválida '{date}'"));
            continue;
        }

        let kind = fields[offset + 1].trim().to_lowercase();
        if kind != "ingreso" && kind != "gasto" {
            skipped += 1;
            errors.push(format!("Fila {row_num}: tipo inválido '{kind}'"));
            continue;
        }

        let category = fields[offset + 2].trim().to_string();
        if !valid_cats.contains(&category) {
            skipped += 1;
            errors.push(format!("Fila {row_num}: categoría '{category}' no existe en presupuestos"));
            continue;
        }

        let amount: i64 = match fields[offset + 3].trim().parse() {
            Ok(a) if a > 0 => a,
            _ => {
                skipped += 1;
                errors.push(format!("Fila {row_num}: monto inválido '{}'", fields[offset + 3].trim()));
                continue;
            }
        };

        let note: Option<String> = fields.get(offset + 4)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let is_extraordinary: bool = fields.get(offset + 5)
            .map(|s| matches!(s.trim(), "Sí" | "Si" | "1" | "true"))
            .unwrap_or(false);

        match conn.execute(
            "INSERT INTO transactions (date, type, category, amount, note, is_extraordinary, goal_id) \
             VALUES (?, ?, ?, ?, ?, ?, NULL)",
            libsql::params![date, kind, category, amount, note, is_extraordinary as i64],
        ).await {
            Ok(_)  => imported += 1,
            Err(e) => { skipped += 1; errors.push(format!("Fila {row_num}: {e}")); }
        }
    }

    Ok(ImportResult { imported, skipped, errors })
}

pub async fn delete_bulk(conn: &libsql::Connection, ids: Vec<i64>) -> AppResult<i64> {
    tx_repo::delete_bulk(conn, &ids).await
}
