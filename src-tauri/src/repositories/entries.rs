use crate::error::{AppError, AppResult};
use crate::models::{
    CategoryComparisonV2, CategoryProgressV2, Entry, EntryFilter, EntryInput, EntryPage, PeriodTotals,
};
use libsql::Connection;

pub fn row_to_entry(row: &libsql::Row) -> Result<Entry, libsql::Error> {
    Ok(Entry {
        id: row.get(0)?,
        occurred_on: row.get(1)?,
        kind: row.get(2)?,
        amount_cop: row.get(3)?,
        account_from: row.get(4)?,
        account_to: row.get(5)?,
        category_id: row.get(6)?,
        goal_id: row.get(7)?,
        loan_id: row.get(8)?,
        note: row.get(9)?,
        is_extraordinary: row.get::<i64>(10)? != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const SELECT_COLUMNS: &str = "id, occurred_on, kind, amount_cop, account_from, account_to, \
     category_id, goal_id, loan_id, note, is_extraordinary, created_at, updated_at";

pub async fn insert(conn: &Connection, id: &str, input: &EntryInput) -> AppResult<Entry> {
    let sql = format!(
        "INSERT INTO entries \
         (id, occurred_on, kind, amount_cop, account_from, account_to, category_id, goal_id, loan_id, note, is_extraordinary) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING {SELECT_COLUMNS}"
    );
    let mut rows = conn
        .query(
            &sql,
            libsql::params![
                id.to_string(),
                input.occurred_on.clone(),
                input.kind.clone(),
                input.amount_cop,
                input.account_from.clone(),
                input.account_to.clone(),
                input.category_id.clone(),
                input.goal_id.clone(),
                input.loan_id.clone(),
                input.note.clone(),
                input.is_extraordinary as i64
            ],
        )
        .await
        .map_err(|e| AppError::DatabaseError(format!("insert entry: {e}")))?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("RETURNING vacío tras INSERT en entries".into()))?;
    row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list_by_period(conn: &Connection, start: &str, end: &str) -> AppResult<Vec<Entry>> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM entries \
         WHERE occurred_on >= ? AND occurred_on <= ? AND deleted_at IS NULL \
         ORDER BY occurred_on DESC, id DESC"
    );
    let mut rows = conn.query(&sql, libsql::params![start.to_string(), end.to_string()]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    let affected = conn
        .execute(
            "UPDATE entries SET deleted_at = datetime('now') WHERE id = ? AND deleted_at IS NULL",
            libsql::params![id.to_string()],
        )
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("entry {id} no existe")));
    }
    Ok(())
}

pub async fn soft_delete_bulk(conn: &Connection, ids: &[String]) -> AppResult<i64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "UPDATE entries SET deleted_at = datetime('now') WHERE id IN ({placeholders}) AND deleted_at IS NULL"
    );
    let params: Vec<libsql::Value> = ids.iter().map(|id| libsql::Value::Text(id.clone())).collect();
    let affected = conn.execute(&sql, params).await?;
    Ok(affected as i64)
}

pub async fn get_by_id(conn: &Connection, id: &str) -> AppResult<Entry> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM entries WHERE id = ? AND deleted_at IS NULL");
    let mut rows = conn.query(&sql, libsql::params![id.to_string()]).await?;
    let row = rows.next().await?.ok_or_else(|| AppError::NotFound(format!("entry {id} no existe")))?;
    row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Solo permite actualizar los campos que tienen sentido editar desde la UI
/// (fecha, monto, nota, extraordinario) — cambiar `kind`/cuentas/categoría
/// de una entry ya creada equivaldría a otro movimiento distinto; para eso
/// se borra y se crea de nuevo.
pub async fn update(
    conn: &Connection,
    id: &str,
    occurred_on: &str,
    amount_cop: i64,
    note: Option<&str>,
    is_extraordinary: bool,
) -> AppResult<Entry> {
    let sql = format!(
        "UPDATE entries SET occurred_on = ?, amount_cop = ?, note = ?, is_extraordinary = ?, updated_at = datetime('now') \
         WHERE id = ? AND deleted_at IS NULL \
         RETURNING {SELECT_COLUMNS}"
    );
    let mut rows = conn
        .query(
            &sql,
            libsql::params![
                occurred_on.to_string(),
                amount_cop,
                note.map(|s| s.to_string()),
                is_extraordinary as i64,
                id.to_string()
            ],
        )
        .await?;
    let row = rows.next().await?.ok_or_else(|| AppError::NotFound(format!("entry {id} no existe")))?;
    row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn list(conn: &Connection, filter: &EntryFilter) -> AppResult<EntryPage> {
    let mut where_sql = " WHERE deleted_at IS NULL".to_string();
    let mut base_params: Vec<libsql::Value> = Vec::new();

    if let Some(start) = &filter.start {
        where_sql.push_str(" AND occurred_on >= ?");
        base_params.push(start.clone().into());
    }
    if let Some(end) = &filter.end {
        where_sql.push_str(" AND occurred_on <= ?");
        base_params.push(end.clone().into());
    }
    if let Some(kind) = &filter.kind {
        where_sql.push_str(" AND kind = ?");
        base_params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category_id {
        where_sql.push_str(" AND category_id = ?");
        base_params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        where_sql.push_str(" AND is_extraordinary = 1");
    }
    if let Some(note) = &filter.search_note {
        where_sql.push_str(" AND note LIKE ?");
        base_params.push(format!("%{note}%").into());
    }

    let total_count: i64 = {
        let sql = format!("SELECT COUNT(*) FROM entries{where_sql}");
        let mut rows = conn.query(&sql, base_params.clone()).await?;
        rows.next().await?.ok_or_else(|| AppError::DatabaseError("count falló".into()))?.get(0)?
    };

    let (filtered_income, filtered_expense): (i64, i64) = {
        let sql = format!(
            "SELECT \
             COALESCE(SUM(CASE WHEN kind='income'  THEN amount_cop ELSE 0 END), 0), \
             COALESCE(SUM(CASE WHEN kind='expense' THEN amount_cop ELSE 0 END), 0) \
             FROM entries{where_sql}"
        );
        let mut rows = conn.query(&sql, base_params.clone()).await?;
        let row = rows.next().await?.ok_or_else(|| AppError::DatabaseError("sum falló".into()))?;
        (row.get(0)?, row.get(1)?)
    };

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(200).max(1);
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM entries{where_sql} ORDER BY occurred_on DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut params = base_params;
    params.push(page_size.into());
    params.push(offset.into());

    let mut rows = conn.query(&sql, params).await?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next().await? {
        entries.push(row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }

    Ok(EntryPage { entries, total_count, filtered_income, filtered_expense })
}

/// Progreso de presupuesto y alerta comparten esta única función (sección 9
/// de schema-v2.md): excluye extraordinarios, y filtra por `category_id`
/// — que ya fija el `kind` (una categoría es income o expense, nunca
/// ambos), así que no hace falta filtrar por `kind` aparte.
pub async fn category_spend(
    conn: &Connection,
    category_id: &str,
    start: &str,
    end: &str,
) -> AppResult<i64> {
    let mut rows = conn
        .query(
            "SELECT COALESCE(SUM(amount_cop), 0) FROM entries \
             WHERE category_id = ? AND is_extraordinary = 0 \
               AND occurred_on >= ? AND occurred_on <= ? AND deleted_at IS NULL",
            libsql::params![category_id.to_string(), start.to_string(), end.to_string()],
        )
        .await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

/// Totales reales del período — income/expense únicamente; los `transfer`
/// quedan fuera por construcción (no aparecen en ningún CASE). Extraordinarios
/// incluidos y desglosados, como manda la sección 9. Las entries cuya
/// categoría es de sistema (`categories.is_system = 1`, ej. saldo/deuda de
/// apertura) se excluyen siempre — no son movimientos reales del período.
pub async fn period_totals(conn: &Connection, start: &str, end: &str) -> AppResult<PeriodTotals> {
    let mut rows = conn
        .query(
            "SELECT
                COALESCE(SUM(CASE WHEN e.kind='income'  THEN e.amount_cop ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN e.kind='expense' THEN e.amount_cop ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN e.kind='income'  AND e.is_extraordinary=1 THEN e.amount_cop ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN e.kind='expense' AND e.is_extraordinary=1 THEN e.amount_cop ELSE 0 END), 0)
             FROM entries e
             LEFT JOIN categories c ON c.id = e.category_id
             WHERE e.occurred_on >= ? AND e.occurred_on <= ? AND e.deleted_at IS NULL
               AND (c.is_system IS NULL OR c.is_system = 0)",
            libsql::params![start.to_string(), end.to_string()],
        )
        .await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| AppError::DatabaseError("period_totals sin resultados".into()))?;
    Ok(PeriodTotals {
        total_income: row.get(0)?,
        total_expense: row.get(1)?,
        extraordinary_income: row.get(2)?,
        extraordinary_expense: row.get(3)?,
    })
}

/// `Σ entries.income − Σ entries.expense` sin exclusión alguna (ni
/// categorías de sistema, ni extraordinarios) — es el invariante de libro
/// mayor del test 10, distinto de `period_totals` (que sí excluye
/// categorías de sistema porque esos son totales para reportes, no para
/// contabilidad general). Los `transfer` nunca aparecen aquí.
pub async fn raw_income_expense_delta(conn: &Connection, start: &str, end: &str) -> AppResult<i64> {
    let mut rows = conn
        .query(
            "SELECT
                COALESCE(SUM(CASE WHEN kind='income'  THEN amount_cop ELSE 0 END), 0)
                -
                COALESCE(SUM(CASE WHEN kind='expense' THEN amount_cop ELSE 0 END), 0)
             FROM entries WHERE occurred_on >= ? AND occurred_on <= ? AND deleted_at IS NULL",
            libsql::params![start.to_string(), end.to_string()],
        )
        .await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

/// Todas las entries que cumplen el filtro, sin paginar — para exportar CSV.
pub async fn list_for_export(conn: &Connection, filter: &EntryFilter) -> AppResult<Vec<Entry>> {
    let mut where_sql = " WHERE deleted_at IS NULL".to_string();
    let mut params: Vec<libsql::Value> = Vec::new();

    if let Some(start) = &filter.start {
        where_sql.push_str(" AND occurred_on >= ?");
        params.push(start.clone().into());
    }
    if let Some(end) = &filter.end {
        where_sql.push_str(" AND occurred_on <= ?");
        params.push(end.clone().into());
    }
    if let Some(kind) = &filter.kind {
        where_sql.push_str(" AND kind = ?");
        params.push(kind.clone().into());
    }
    if let Some(cat) = &filter.category_id {
        where_sql.push_str(" AND category_id = ?");
        params.push(cat.clone().into());
    }
    if filter.only_extraordinary == Some(true) {
        where_sql.push_str(" AND is_extraordinary = 1");
    }
    if let Some(note) = &filter.search_note {
        where_sql.push_str(" AND note LIKE ?");
        params.push(format!("%{note}%").into());
    }

    let sql = format!("SELECT {SELECT_COLUMNS} FROM entries{where_sql} ORDER BY occurred_on DESC, id DESC");
    let mut rows = conn.query(&sql, params).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row_to_entry(&row).map_err(|e| AppError::DatabaseError(e.to_string()))?);
    }
    Ok(out)
}

pub async fn count_in_range(conn: &Connection, start: &str, end: &str) -> AppResult<i64> {
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM entries WHERE occurred_on >= ? AND occurred_on <= ? AND deleted_at IS NULL",
            libsql::params![start.to_string(), end.to_string()],
        )
        .await?;
    Ok(rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(0))
}

/// IDs de categorías con al menos un gasto no extraordinario en el período
/// — para reconstruir `Σ(gastos por categoría)` sin adivinar el universo
/// de categorías de antemano (test 6). Excluye categorías de sistema, igual
/// que `period_totals`.
pub async fn distinct_expense_categories_in_period(
    conn: &Connection,
    start: &str,
    end: &str,
) -> AppResult<Vec<String>> {
    let mut rows = conn
        .query(
            "SELECT DISTINCT e.category_id FROM entries e
             JOIN categories c ON c.id = e.category_id
             WHERE e.kind = 'expense' AND e.is_extraordinary = 0 AND c.is_system = 0
               AND e.occurred_on >= ? AND e.occurred_on <= ? AND e.deleted_at IS NULL",
            libsql::params![start.to_string(), end.to_string()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row.get(0)?);
    }
    Ok(out)
}

/// Progreso por categoría (todas las que no son de sistema), para un rango
/// y un `year_month` opcional (para resolver `budget_overrides`). Usa
/// `category_spend` para cada fila — la misma función que sirve a la
/// alerta de presupuesto, sección 9 de schema-v2.md.
pub async fn list_category_progress(
    conn: &Connection,
    start: &str,
    end: &str,
    year_month: Option<&str>,
) -> AppResult<Vec<CategoryProgressV2>> {
    let mut rows = conn
        .query(
            "SELECT id, name, kind, is_fixed FROM categories WHERE is_system = 0 AND archived_at IS NULL ORDER BY name",
            (),
        )
        .await?;
    let mut cats = Vec::new();
    while let Some(row) = rows.next().await? {
        cats.push((row.get::<String>(0)?, row.get::<String>(1)?, row.get::<String>(2)?, row.get::<i64>(3)? != 0));
    }

    let mut out = Vec::new();
    for (id, name, kind, is_fixed) in cats {
        let current_amount = category_spend(conn, &id, start, end).await?;
        let monthly_target = crate::repositories::budgets_v2::effective_monthly(conn, &id, year_month).await?;
        let percentage = if monthly_target > 0 { current_amount as f64 / monthly_target as f64 * 100.0 } else { 0.0 };
        out.push(CategoryProgressV2 {
            category_id: id,
            category_name: name,
            kind,
            is_fixed,
            monthly_target,
            current_amount,
            percentage,
            is_over: monthly_target > 0 && current_amount > monthly_target,
        });
    }
    Ok(out)
}

/// Comparativa gasto actual vs. mes anterior, por categoría — igual criterio
/// que v1 (mes calendario completo, no `Period` genérico), excluyendo
/// categorías de sistema.
pub async fn month_comparison(
    conn: &Connection,
    curr_start: &str,
    curr_end: &str,
    prev_start: &str,
    prev_end: &str,
) -> AppResult<Vec<CategoryComparisonV2>> {
    let mut rows = conn
        .query(
            "SELECT
                c.id, c.name,
                COALESCE(SUM(CASE WHEN e.occurred_on >= ?1 AND e.occurred_on <= ?2 THEN e.amount_cop ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN e.occurred_on >= ?3 AND e.occurred_on <= ?4 THEN e.amount_cop ELSE 0 END), 0)
             FROM entries e
             JOIN categories c ON c.id = e.category_id
             WHERE e.kind = 'expense' AND c.is_system = 0 AND e.deleted_at IS NULL
               AND ((e.occurred_on >= ?1 AND e.occurred_on <= ?2) OR (e.occurred_on >= ?3 AND e.occurred_on <= ?4))
             GROUP BY c.id, c.name
             ORDER BY 3 DESC",
            libsql::params![curr_start.to_string(), curr_end.to_string(), prev_start.to_string(), prev_end.to_string()],
        )
        .await?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let current: i64 = row.get(2)?;
        let previous: i64 = row.get(3)?;
        let delta_pct = if previous > 0 { (current - previous) as f64 / previous as f64 * 100.0 } else { 0.0 };
        out.push(CategoryComparisonV2 {
            category_id: row.get(0)?,
            category_name: row.get(1)?,
            current,
            previous,
            delta_pct,
        });
    }
    Ok(out)
}
