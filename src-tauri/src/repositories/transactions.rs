use chrono::{Local, Datelike, NaiveDate};
use crate::error::{AppError, AppResult};
use crate::models::{
    CategoryProgressRaw, CurrentBalance, MonthComparison, CategoryComparison,
    PeriodSummary, Transaction, TransactionFilter, TransactionInput, TransactionPage,
};
use crate::utils::{days_in_month, period_to_dates};

pub fn row_to_transaction(row: &libsql::Row) -> Result<Transaction, libsql::Error> {
    Ok(Transaction {
        id: row.get(0)?,
        date: row.get(1)?,
        kind: row.get(2)?,
        category: row.get(3)?,
        amount: row.get(4)?,
        note: row.get(5)?,
        is_extraordinary: row.get::<i64>(6)? != 0,
        goal_id: row.get(7)?,
        created_at: row.get(8)?,
        is_debt: row.get::<i64>(9).unwrap_or(0) != 0,
    })
}

pub async fn insert(
    conn: &libsql::Connection,
    input: &TransactionInput,
) -> AppResult<Transaction> {
    let mut rows = conn.query(
        "INSERT INTO transactions \
         (date, type, category, amount, note, is_extraordinary, goal_id, is_debt) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING id, date, type, category, amount, note, \
                   is_extraordinary, goal_id, created_at, is_debt",
        libsql::params![
            input.date.clone(),
            input.kind.clone(),
            input.category.clone(),
            input.amount,
            input.note.clone(),
            input.is_extraordinary as i64,
            input.goal_id,
            input.is_debt as i64
        ],
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let row = rows.next().await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::DatabaseError("RETURNING vacío tras INSERT".into()))?;

    row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(
    conn: &libsql::Connection,
    filter: &TransactionFilter,
) -> AppResult<TransactionPage> {
    let mut where_sql = " WHERE 1=1".to_string();
    let mut base_params: Vec<libsql::Value> = Vec::new();

    if let Some(period) = &filter.period {
        let (start, end) = period_to_dates(period);
        where_sql.push_str(" AND date >= ? AND date <= ?");
        base_params.push(start.into());
        base_params.push(end.into());
    }
    if let Some(kind) = &filter.kind {
        where_sql.push_str(" AND type = ?");
        base_params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category {
        where_sql.push_str(" AND category = ?");
        base_params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        where_sql.push_str(" AND is_extraordinary = 1");
    }
    if filter.only_debt == Some(true) {
        where_sql.push_str(" AND is_debt = 1");
    }
    if let Some(note) = &filter.search_note {
        where_sql.push_str(" AND note LIKE ?");
        base_params.push(format!("%{note}%").into());
    }

    let total_count: i64 = {
        let count_sql = format!("SELECT COUNT(*) FROM transactions{where_sql}");
        let mut rows = conn.query(&count_sql, base_params.clone()).await?;
        let row = rows.next().await?
            .ok_or_else(|| AppError::DatabaseError("count query failed".into()))?;
        row.get(0)?
    };

    let (filtered_income, filtered_expenses): (i64, i64) = {
        let sum_sql = format!(
            "SELECT \
             COALESCE(SUM(CASE WHEN type='ingreso' THEN amount ELSE 0 END), 0), \
             COALESCE(SUM(CASE WHEN type='gasto'   THEN amount ELSE 0 END), 0) \
             FROM transactions{where_sql}"
        );
        let mut rows = conn.query(&sum_sql, base_params.clone()).await?;
        let row = rows.next().await?
            .ok_or_else(|| AppError::DatabaseError("filtered sum query failed".into()))?;
        (row.get(0)?, row.get(1)?)
    };

    let page      = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(200).max(1);
    let offset    = (page - 1) * page_size;

    let select_sql = format!(
        "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
         FROM transactions{where_sql} ORDER BY date DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut params = base_params;
    params.push(page_size.into());
    params.push(offset.into());

    let mut rows = conn.query(&select_sql, params).await?;
    let mut transactions = Vec::new();
    while let Some(row) = rows.next().await? {
        transactions.push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }

    Ok(TransactionPage { transactions, total_count, filtered_income, filtered_expenses })
}

pub async fn get_balance(conn: &libsql::Connection) -> AppResult<CurrentBalance> {
    let mut rows = conn.query(
        "SELECT
            COALESCE(SUM(CASE WHEN type='ingreso' THEN amount ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN type='gasto'   THEN amount ELSE 0 END), 0)
         FROM transactions",
        (),
    ).await?;

    let row = rows.next().await?
        .ok_or_else(|| AppError::DatabaseError("balance sin resultados".into()))?;

    let total_income: i64  = row.get(0)?;
    let total_expenses: i64 = row.get(1)?;

    Ok(CurrentBalance { total_income, total_expenses, balance: total_income - total_expenses })
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    input: &TransactionInput,
) -> AppResult<Transaction> {
    let mut rows = conn.query(
        "UPDATE transactions \
         SET date=?, type=?, category=?, amount=?, note=?, is_extraordinary=?, goal_id=?, is_debt=? \
         WHERE id=? \
         RETURNING id, date, type, category, amount, note, \
                   is_extraordinary, goal_id, created_at, is_debt",
        libsql::params![
            input.date.clone(),
            input.kind.clone(),
            input.category.clone(),
            input.amount,
            input.note.clone(),
            input.is_extraordinary as i64,
            input.goal_id,
            input.is_debt as i64,
            id
        ],
    ).await?;

    let row = rows.next().await?
        .ok_or_else(|| AppError::NotFound(format!("transacción {id} no existe")))?;

    row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM transactions WHERE id = ?", libsql::params![id]).await?;
    Ok(())
}

pub async fn get_period_summary(
    conn: &libsql::Connection,
    start: &str,
    end: &str,
) -> AppResult<PeriodSummary> {
    let mut rows = conn.query(
        "SELECT
            COALESCE(SUM(CASE WHEN type='ingreso' THEN amount ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN type='gasto'   THEN amount ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN type='ingreso' AND is_extraordinary=1 THEN amount ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN type='gasto'   AND is_extraordinary=1 THEN amount ELSE 0 END), 0),
            COUNT(*)
         FROM transactions WHERE date >= ? AND date <= ?",
        libsql::params![start.to_string(), end.to_string()],
    ).await?;

    let row = rows.next().await?
        .ok_or_else(|| AppError::DatabaseError("summary sin resultados".into()))?;

    let total_income: i64         = row.get(0)?;
    let total_expenses: i64       = row.get(1)?;
    let extraordinary_income: i64  = row.get(2)?;
    let extraordinary_expenses: i64 = row.get(3)?;
    let transactions_count: i64   = row.get(4)?;

    Ok(PeriodSummary {
        total_income,
        total_expenses,
        balance: total_income - total_expenses,
        extraordinary_income,
        extraordinary_expenses,
        transactions_count,
    })
}

pub async fn list_category_budget_rows(
    conn: &libsql::Connection,
    start: &str,
    end: &str,
) -> AppResult<Vec<CategoryProgressRaw>> {
    let mut rows = conn.query(
        "SELECT
            b.category,
            b.monthly_amount,
            COALESCE((
                SELECT SUM(amount) FROM transactions
                WHERE category = b.category AND date >= ? AND date <= ?
                  AND is_extraordinary = 0
            ), 0) AS current_amount,
            b.type,
            b.is_fixed
         FROM budgets b
         ORDER BY b.category",
        libsql::params![start.to_string(), end.to_string()],
    ).await?;

    let mut result = Vec::new();
    while let Some(row) = rows.next().await? {
        result.push(CategoryProgressRaw {
            category: row.get(0)?,
            monthly_amount: row.get(1)?,
            current_amount: row.get(2)?,
            kind: row.get(3)?,
            is_fixed: row.get::<i64>(4).unwrap_or(0) != 0,
        });
    }
    Ok(result)
}

pub async fn get_month_comparison(conn: &libsql::Connection) -> AppResult<MonthComparison> {
    let today = Local::now().date_naive();

    let curr_first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let curr_last  = NaiveDate::from_ymd_opt(
        today.year(), today.month(), days_in_month(today.year(), today.month()),
    ).unwrap();

    let (prev_year, prev_month) = if today.month() == 1 {
        (today.year() - 1, 12u32)
    } else {
        (today.year(), today.month() - 1)
    };
    let prev_first = NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap();
    let prev_last  = NaiveDate::from_ymd_opt(
        prev_year, prev_month, days_in_month(prev_year, prev_month),
    ).unwrap();

    let cs = curr_first.format("%Y-%m-%d").to_string();
    let ce = curr_last.format("%Y-%m-%d").to_string();
    let ps = prev_first.format("%Y-%m-%d").to_string();
    let pe = prev_last.format("%Y-%m-%d").to_string();

    let mut rows = conn.query(
        "SELECT
            category,
            COALESCE(SUM(CASE WHEN date >= ? AND date <= ? THEN amount ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN date >= ? AND date <= ? THEN amount ELSE 0 END), 0)
         FROM transactions
         WHERE type = 'gasto'
           AND ((date >= ? AND date <= ?) OR (date >= ? AND date <= ?))
         GROUP BY category
         ORDER BY 2 DESC",
        libsql::params![
            cs.clone(), ce.clone(),
            ps.clone(), pe.clone(),
            cs.clone(), ce.clone(),
            ps.clone(), pe.clone()
        ],
    ).await?;

    let mut by_category: Vec<CategoryComparison> = Vec::new();
    let mut current_month_total: i64  = 0;
    let mut previous_month_total: i64 = 0;

    while let Some(row) = rows.next().await? {
        let category: String = row.get(0)?;
        let current: i64     = row.get(1)?;
        let previous: i64    = row.get(2)?;
        let delta_pct = if previous > 0 {
            (current - previous) as f64 / previous as f64 * 100.0
        } else {
            0.0
        };
        current_month_total  += current;
        previous_month_total += previous;
        by_category.push(CategoryComparison { category, current, previous, delta_pct });
    }

    let delta_amount = current_month_total - previous_month_total;
    let delta_percentage = if previous_month_total > 0 {
        delta_amount as f64 / previous_month_total as f64 * 100.0
    } else {
        0.0
    };

    Ok(MonthComparison { current_month_total, previous_month_total, delta_amount, delta_percentage, by_category })
}

pub async fn list_by_goal(
    conn: &libsql::Connection,
    goal_id: i64,
) -> AppResult<Vec<Transaction>> {
    let mut rows = conn.query(
        "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
         FROM transactions WHERE goal_id = ? ORDER BY date DESC, id DESC",
        libsql::params![goal_id],
    ).await?;

    let mut result = Vec::new();
    while let Some(row) = rows.next().await? {
        result.push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(result)
}

pub async fn sum_by_goal(conn: &libsql::Connection, goal_id: i64) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE goal_id = ?",
        libsql::params![goal_id],
    ).await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

pub async fn sum_by_goal_recent(conn: &libsql::Connection, goal_id: i64) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions \
         WHERE goal_id = ? AND date >= date('now', '-3 months')",
        libsql::params![goal_id],
    ).await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

pub async fn spent_in_category(
    conn: &libsql::Connection,
    category: &str,
    month_start: &str,
    today: &str,
) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions \
         WHERE category = ? AND type = 'gasto' AND date >= ? AND date <= ?",
        libsql::params![category.to_string(), month_start.to_string(), today.to_string()],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0))
}

pub async fn sum_for_goal_completion(
    conn: &libsql::Connection,
    goal_id: i64,
) -> AppResult<i64> {
    let mut rows = conn.query(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE goal_id = ?",
        libsql::params![goal_id],
    ).await?;
    Ok(rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0))
}

pub async fn delete_bulk(conn: &libsql::Connection, ids: &[i64]) -> AppResult<i64> {
    if ids.is_empty() { return Ok(0); }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM transactions WHERE id IN ({placeholders})");
    let params: Vec<libsql::Value> = ids.iter().map(|&id| libsql::Value::Integer(id)).collect();
    let affected = conn.execute(&sql, params).await?;
    Ok(affected as i64)
}

pub async fn list_for_export(
    conn: &libsql::Connection,
    filter: &TransactionFilter,
) -> AppResult<Vec<Transaction>> {
    let mut sql = "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, created_at, is_debt \
                   FROM transactions WHERE 1=1".to_string();
    let mut params: Vec<libsql::Value> = Vec::new();

    if let Some(period) = &filter.period {
        let (start, end) = period_to_dates(period);
        sql.push_str(" AND date >= ? AND date <= ?");
        params.push(start.into());
        params.push(end.into());
    }
    if let Some(kind) = &filter.kind {
        sql.push_str(" AND type = ?");
        params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category {
        sql.push_str(" AND category = ?");
        params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        sql.push_str(" AND is_extraordinary = 1");
    }
    if let Some(note) = &filter.search_note {
        sql.push_str(" AND note LIKE ?");
        params.push(format!("%{note}%").into());
    }
    sql.push_str(" ORDER BY date DESC, id DESC");

    let mut rows = conn.query(&sql, params).await?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().await? {
        result.push(row_to_transaction(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(result)
}

pub async fn list_distinct_categories_by_kind(
    conn: &libsql::Connection,
    kind: &str,
) -> AppResult<Vec<String>> {
    let mut rows = conn.query(
        "SELECT DISTINCT category FROM transactions WHERE type = ? ORDER BY category",
        libsql::params![kind.to_string()],
    ).await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? { cats.push(row.get(0)?); }
    Ok(cats)
}

pub async fn list_all_distinct_categories(conn: &libsql::Connection) -> AppResult<Vec<String>> {
    let mut rows = conn.query(
        "SELECT DISTINCT category FROM transactions ORDER BY category",
        (),
    ).await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? { cats.push(row.get(0)?); }
    Ok(cats)
}
