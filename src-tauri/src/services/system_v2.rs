use crate::error::AppResult;
use libsql::Connection;

/// Reinicio de fábrica para el esquema v2: borra absolutamente todo dato
/// del usuario. Solo se preservan `accounts` (5 filas de sistema fijas,
/// no datos) y `categories` con `is_system = 1` (categorías base de la
/// app, no creadas por el usuario).
pub async fn factory_reset(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM entries", ()).await?;
    conn.execute("DELETE FROM fillups", ()).await?;
    conn.execute("DELETE FROM trips", ()).await?;
    conn.execute("DELETE FROM fuel_adjustments", ()).await?;
    conn.execute("DELETE FROM gas_prices", ()).await?;
    conn.execute("DELETE FROM goals", ()).await?;
    conn.execute("DELETE FROM loans", ()).await?;
    conn.execute("DELETE FROM vehicles", ()).await?;
    conn.execute("DELETE FROM budget_overrides", ()).await?;
    conn.execute("DELETE FROM budgets", ()).await?;
    conn.execute("DELETE FROM categories WHERE is_system = 0", ()).await?;
    conn.execute("DELETE FROM routes", ()).await?;
    Ok(())
}
