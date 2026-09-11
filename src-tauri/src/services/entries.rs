use crate::error::{AppError, AppResult};
use crate::models::{
    AccountBalances, CategoryProgressV2, CsvExport, Entry, EntryFilter, EntryInput, EntryPage,
    MonthComparisonV2, PeriodSummaryV2, PeriodTotals, PeriodV2,
};
use crate::repositories;
use crate::utils::{csv_escape, days_in_month, resolve_period_range};
use chrono::{Datelike, Local, NaiveDate};
use libsql::Connection;
use std::collections::HashMap;
use ulid::Ulid;

/// Igual que el `CHECK` de la tabla `entries` (sección 4 de schema-v2.md),
/// repetido aquí en Rust para dar un `ValidationError` legible en vez de un
/// error crudo de SQLite. La base de datos sigue siendo la garantía final.
fn validate(input: &EntryInput) -> AppResult<()> {
    if input.amount_cop <= 0 {
        return Err(AppError::ValidationError("amount_cop debe ser mayor que cero".into()));
    }
    match input.kind.as_str() {
        "income" => {
            if input.account_from.is_some() || input.account_to.is_none() || input.category_id.is_none() {
                return Err(AppError::ValidationError(
                    "income requiere account_to y category_id, y no debe tener account_from".into(),
                ));
            }
        }
        "expense" => {
            if input.account_from.is_none() || input.account_to.is_some() || input.category_id.is_none() {
                return Err(AppError::ValidationError(
                    "expense requiere account_from y category_id, y no debe tener account_to".into(),
                ));
            }
        }
        "transfer" => {
            if input.account_from.is_none() || input.account_to.is_none() || input.category_id.is_some() {
                return Err(AppError::ValidationError(
                    "transfer requiere account_from y account_to, y no debe tener category_id".into(),
                ));
            }
            if input.account_from == input.account_to {
                return Err(AppError::ValidationError("transfer no puede tener la misma cuenta origen y destino".into()));
            }
        }
        other => return Err(AppError::ValidationError(format!("kind desconocido: {other}"))),
    }
    Ok(())
}

pub async fn create(conn: &Connection, input: EntryInput) -> AppResult<Entry> {
    validate(&input)?;
    let id = Ulid::new().to_string();
    repositories::entries::insert(conn, &id, &input).await
}

pub async fn account_balances(conn: &Connection) -> AppResult<AccountBalances> {
    repositories::accounts::balances(conn).await
}

pub async fn category_spend(conn: &Connection, category_id: &str, start: &str, end: &str) -> AppResult<i64> {
    repositories::entries::category_spend(conn, category_id, start, end).await
}

pub async fn period_totals(conn: &Connection, start: &str, end: &str) -> AppResult<PeriodTotals> {
    repositories::entries::period_totals(conn, start, end).await
}

/// Ver test 6 de la sección 11: `Σ(gastos por categoría) + extraordinarios`
/// debe ser exactamente igual a `total_expense` del período — sin residuo.
pub async fn expense_by_category_matches_total(conn: &Connection, start: &str, end: &str) -> AppResult<bool> {
    let totals = period_totals(conn, start, end).await?;
    let categories = repositories::entries::distinct_expense_categories_in_period(conn, start, end).await?;
    let mut sum = 0i64;
    for cat in categories {
        sum += category_spend(conn, &cat, start, end).await?;
    }
    Ok(sum + totals.extraordinary_expense == totals.total_expense)
}

pub async fn get_by_id(conn: &Connection, id: &str) -> AppResult<Entry> {
    repositories::entries::get_by_id(conn, id).await
}

pub async fn update(
    conn: &Connection,
    id: &str,
    occurred_on: &str,
    amount_cop: i64,
    note: Option<&str>,
    is_extraordinary: bool,
) -> AppResult<Entry> {
    if amount_cop <= 0 {
        return Err(AppError::ValidationError("amount_cop debe ser mayor que cero".into()));
    }
    repositories::entries::update(conn, id, occurred_on, amount_cop, note, is_extraordinary).await
}

pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repositories::entries::soft_delete(conn, id).await
}

pub async fn delete_bulk(conn: &Connection, ids: Vec<String>) -> AppResult<i64> {
    repositories::entries::soft_delete_bulk(conn, &ids).await
}

pub async fn list(conn: &Connection, filter: EntryFilter) -> AppResult<EntryPage> {
    repositories::entries::list(conn, &filter).await
}

pub async fn period_summary(conn: &Connection, period: &PeriodV2) -> AppResult<PeriodSummaryV2> {
    let range = resolve_period_range(period);
    let totals = period_totals(conn, &range.start, &range.end).await?;
    let entries_count = repositories::entries::count_in_range(conn, &range.start, &range.end).await?;
    Ok(PeriodSummaryV2 {
        range,
        total_income: totals.total_income,
        total_expense: totals.total_expense,
        balance: totals.total_income - totals.total_expense,
        extraordinary_income: totals.extraordinary_income,
        extraordinary_expense: totals.extraordinary_expense,
        entries_count,
    })
}

pub async fn category_progress(
    conn: &Connection,
    start: &str,
    end: &str,
    year_month: Option<&str>,
) -> AppResult<Vec<CategoryProgressV2>> {
    repositories::entries::list_category_progress(conn, start, end, year_month).await
}

pub async fn export_csv(conn: &Connection, filter: EntryFilter) -> AppResult<CsvExport> {
    let rows = repositories::entries::list_for_export(conn, &filter).await?;
    let categories = repositories::categories::list(conn, None).await?;
    let cat_names: HashMap<String, String> = categories.into_iter().map(|c| (c.id, c.name)).collect();

    let mut csv = String::from("ID,Fecha,Tipo,Categoría,Monto (COP),Nota,Extraordinario,Creado en\n");
    for e in &rows {
        let category_name = e.category_id.as_ref().and_then(|id| cat_names.get(id)).map(|s| s.as_str()).unwrap_or("");
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            e.id,
            e.occurred_on,
            e.kind,
            csv_escape(category_name),
            e.amount_cop,
            csv_escape(e.note.as_deref().unwrap_or("")),
            if e.is_extraordinary { "Sí" } else { "No" },
            e.created_at,
        ));
    }
    let today = Local::now().format("%Y-%m-%d").to_string();
    Ok(CsvExport { content: csv, suggested_filename: format!("movimientos_{today}.csv") })
}

pub async fn month_comparison(conn: &Connection) -> AppResult<MonthComparisonV2> {
    let today = Local::now().date_naive();
    let curr_first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let curr_last = NaiveDate::from_ymd_opt(today.year(), today.month(), days_in_month(today.year(), today.month())).unwrap();
    let (prev_year, prev_month) = if today.month() == 1 { (today.year() - 1, 12u32) } else { (today.year(), today.month() - 1) };
    let prev_first = NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap();
    let prev_last = NaiveDate::from_ymd_opt(prev_year, prev_month, days_in_month(prev_year, prev_month)).unwrap();

    let cs = curr_first.format("%Y-%m-%d").to_string();
    let ce = curr_last.format("%Y-%m-%d").to_string();
    let ps = prev_first.format("%Y-%m-%d").to_string();
    let pe = prev_last.format("%Y-%m-%d").to_string();

    let by_category = repositories::entries::month_comparison(conn, &cs, &ce, &ps, &pe).await?;
    let current_month_total: i64 = by_category.iter().map(|c| c.current).sum();
    let previous_month_total: i64 = by_category.iter().map(|c| c.previous).sum();
    let delta_amount = current_month_total - previous_month_total;
    let delta_percentage = if previous_month_total > 0 { delta_amount as f64 / previous_month_total as f64 * 100.0 } else { 0.0 };

    Ok(MonthComparisonV2 { current_month_total, previous_month_total, delta_amount, delta_percentage, by_category })
}
