use crate::error::AppResult;
use libsql::Connection;

/// Reinicio de fábrica para el esquema v2. A diferencia de v1, no toca
/// `accounts` (son 5 filas de sistema, no datos del usuario) ni `gas_prices`
/// (historial de referencia, sección 7 de schema-v2.md) ni `config`.
pub async fn factory_reset(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM entries", ()).await?;
    conn.execute("DELETE FROM fillups", ()).await?;
    conn.execute("DELETE FROM trips", ()).await?;
    conn.execute("DELETE FROM fuel_adjustments", ()).await?;
    conn.execute("DELETE FROM goals", ()).await?;
    conn.execute("DELETE FROM loans", ()).await?;
    conn.execute("DELETE FROM vehicles", ()).await?;
    conn.execute("DELETE FROM budget_overrides", ()).await?;
    conn.execute("DELETE FROM budgets", ()).await?;
    conn.execute("DELETE FROM categories WHERE is_system = 0", ()).await?;
    conn.execute("DELETE FROM routes", ()).await?;
    Ok(())
}
