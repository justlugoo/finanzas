use crate::error::AppResult;
use crate::models::Goal;

pub async fn list(
    conn: &libsql::Connection,
    status: Option<&str>,
) -> AppResult<Vec<Goal>> {
    let mut rows = if let Some(s) = status {
        conn.query(
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
             FROM goals WHERE status = ? ORDER BY name",
            libsql::params![s.to_string()],
        ).await?
    } else {
        conn.query(
            "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
             FROM goals ORDER BY name",
            (),
        ).await?
    };

    let mut goals = Vec::new();
    while let Some(row) = rows.next().await? {
        goals.push(row_to_goal(&row)?);
    }
    Ok(goals)
}

pub async fn find_by_id(
    conn: &libsql::Connection,
    id: i64,
) -> AppResult<Option<Goal>> {
    let mut rows = conn.query(
        "SELECT id, name, target_amount, target_date, status, created_at, is_debt_goal \
         FROM goals WHERE id = ?",
        libsql::params![id],
    ).await?;

    match rows.next().await? { Some(row) => {
        Ok(Some(row_to_goal(&row)?))
    } _ => {
        Ok(None)
    }}
}

pub async fn find_active_by_id(
    conn: &libsql::Connection,
    id: i64,
) -> AppResult<Option<(String, i64)>> {
    let mut rows = conn.query(
        "SELECT name, target_amount FROM goals WHERE id = ? AND status != 'completado'",
        libsql::params![id],
    ).await?;
    Ok(rows.next().await?.map(|r| {
        let name: String = r.get(0).unwrap_or_default();
        let target: i64  = r.get(1).unwrap_or(0);
        (name, target)
    }))
}

pub async fn insert(
    conn: &libsql::Connection,
    name: &str,
    target_amount: i64,
    target_date: Option<&str>,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO goals (name, target_amount, target_date) VALUES (?, ?, ?)",
        libsql::params![name.to_string(), target_amount, target_date.map(|s| s.to_string())],
    ).await?;
    Ok(conn.last_insert_rowid())
}

pub async fn find_debt_goal_by_name(
    conn: &libsql::Connection,
    name: &str,
) -> AppResult<Option<i64>> {
    let mut rows = conn.query(
        "SELECT id FROM goals WHERE name = ? AND is_debt_goal = 1 LIMIT 1",
        libsql::params![name.to_string()],
    ).await?;
    Ok(rows.next().await?.map(|r| r.get(0).unwrap_or(0)))
}

pub async fn insert_debt_goal(
    conn: &libsql::Connection,
    name: &str,
    amount: i64,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO goals (name, target_amount, is_debt_goal) VALUES (?, ?, 1)",
        libsql::params![name.to_string(), amount],
    ).await?;
    Ok(conn.last_insert_rowid())
}

pub async fn update(
    conn: &libsql::Connection,
    id: i64,
    name: &str,
    target_amount: i64,
    target_date: Option<&str>,
    status: &str,
) -> AppResult<u64> {
    let affected = conn.execute(
        "UPDATE goals SET name = ?, target_amount = ?, target_date = ?, status = ? WHERE id = ?",
        libsql::params![
            name.to_string(),
            target_amount,
            target_date.map(|s| s.to_string()),
            status.to_string(),
            id
        ],
    ).await?;
    Ok(affected)
}

pub async fn mark_completed(conn: &libsql::Connection, id: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE goals SET status = 'completado' WHERE id = ?",
        libsql::params![id],
    ).await?;
    Ok(())
}

pub async fn delete(conn: &libsql::Connection, id: i64) -> AppResult<u64> {
    conn.execute(
        "UPDATE transactions SET goal_id = NULL WHERE goal_id = ?",
        libsql::params![id],
    ).await?;
    let affected = conn.execute(
        "DELETE FROM goals WHERE id = ?",
        libsql::params![id],
    ).await?;
    Ok(affected)
}

fn row_to_goal(row: &libsql::Row) -> AppResult<Goal> {
    Ok(Goal {
        id: row.get(0)?,
        name: row.get(1)?,
        target_amount: row.get(2)?,
        target_date: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        is_debt_goal: row.get::<i64>(6).unwrap_or(0) != 0,
    })
}
