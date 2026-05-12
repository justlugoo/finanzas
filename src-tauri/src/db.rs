use libsql::Builder;
use crate::error::{AppError, AppResult};

// Schema sin PRAGMAs — idempotente, se aplica en cada arranque
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS budgets (
    category        TEXT    PRIMARY KEY,
    monthly_amount  INTEGER NOT NULL CHECK (monthly_amount >= 0),
    route_id        INTEGER REFERENCES custom_routes(id) ON DELETE SET NULL,
    type            TEXT    NOT NULL DEFAULT 'gasto' CHECK (type IN ('ingreso', 'gasto'))
);

CREATE TABLE IF NOT EXISTS goals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    target_amount   INTEGER NOT NULL CHECK (target_amount > 0),
    target_date     TEXT,
    status          TEXT    NOT NULL DEFAULT 'activo'
                            CHECK (status IN ('activo', 'completado', 'pausado')),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    is_debt_goal    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS transactions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    date                TEXT    NOT NULL,
    type                TEXT    NOT NULL CHECK (type IN ('ingreso', 'gasto')),
    category            TEXT    NOT NULL,
    amount              INTEGER NOT NULL CHECK (amount > 0),
    note                TEXT,
    is_extraordinary    INTEGER NOT NULL DEFAULT 0
                                CHECK (is_extraordinary IN (0, 1)),
    goal_id             INTEGER,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    is_debt             INTEGER NOT NULL DEFAULT 0
                                CHECK (is_debt IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_tx_date          ON transactions(date);
CREATE INDEX IF NOT EXISTS idx_tx_category      ON transactions(category);
CREATE INDEX IF NOT EXISTS idx_tx_date_category ON transactions(date, category);

CREATE TABLE IF NOT EXISTS gas_prices (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    date                TEXT    NOT NULL UNIQUE,
    price_per_gallon    INTEGER NOT NULL CHECK (price_per_gallon BETWEEN 1000 AND 100000),
    source              TEXT    NOT NULL CHECK (source IN ('manual', 'scraping'))
);

CREATE TABLE IF NOT EXISTS config (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_routes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    km_round_trip   REAL    NOT NULL CHECK (km_round_trip > 0),
    description     TEXT
);


INSERT OR IGNORE INTO config (key, value) VALUES
    ('consumo_moto_km_galon',      '350'),
    ('umbral_alerta_gasolina_pct', '5'),
    ('umbral_alerta_meta_pct',     '100'),
    ('scraping_gasolina_activo',   'false');
";

pub async fn open_database() -> AppResult<libsql::Database> {
    let mut path = dirs::data_local_dir()
        .ok_or_else(|| AppError::DatabaseError("no se pudo determinar directorio local".into()))?;
    path.push("finanzas");
    std::fs::create_dir_all(&path)?;
    path.push("local.db");

    match Builder::new_local(&path).build().await {
        Ok(db) => Ok(db),
        Err(e) if e.to_string().contains("wal_index") => {
            // libsql falla al arrancar si encuentra un .db-shm huérfano de una
            // sesión anterior que no hizo checkpoint (apagado abrupto del sistema).
            // Si el .db-wal está vacío no hay transacciones pendientes → es seguro
            // eliminar ambos archivos WAL y reabrir.
            let wal = path.with_extension("db-wal");
            let shm = path.with_extension("db-shm");
            let wal_empty = wal.metadata().map(|m| m.len() == 0).unwrap_or(true);
            if wal_empty {
                let _ = std::fs::remove_file(&shm);
                let _ = std::fs::remove_file(&wal);
                Builder::new_local(&path).build().await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))
            } else {
                Err(AppError::DatabaseError(e.to_string()))
            }
        }
        Err(e) => Err(AppError::DatabaseError(e.to_string())),
    }
}

/// Pragmas de rendimiento — se aplican a cada conexión nueva.
/// synchronous=NORMAL: omite fsync por write (WAL es seguro así).
/// cache_size=-65536: 64 MB de caché de páginas en RAM (default=2 MB).
/// mmap_size=268435456: 256 MB de mmap; leer es casi acceso a memoria.
/// temp_store=MEMORY: tablas temporales (sorts/joins) en RAM, no disco.
pub async fn apply_pragmas(conn: &libsql::Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous  = NORMAL;
         PRAGMA cache_size   = -65536;
         PRAGMA mmap_size    = 268435456;
         PRAGMA temp_store   = MEMORY;",
    )
    .await
    .map(|_| ())
    .map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn apply_schema(conn: &libsql::Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA)
        .await
        .map(|_| ())
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Migración: añade columna type si la tabla existía antes.
    let _ = conn.execute(
        "ALTER TABLE budgets ADD COLUMN type TEXT NOT NULL DEFAULT 'gasto' \
         CHECK (type IN ('ingreso', 'gasto'))",
        (),
    ).await;
    let _ = conn.execute_batch(
        "UPDATE budgets SET type = 'ingreso' \
         WHERE category IN ('Mesada', 'Viaje', 'Eventual', 'Otro ingreso');",
    ).await;

    // Seed inicial — solo si la tabla está completamente vacía (instalación nueva).
    let mut n = conn.query("SELECT COUNT(*) FROM budgets", ()).await?;
    if n.next().await?.and_then(|r| r.get::<i64>(0).ok()).unwrap_or(1) == 0 {
        conn.execute_batch(
            "INSERT INTO budgets (category, monthly_amount, type) VALUES
                 ('Mesada',         300000, 'ingreso'),
                 ('Viaje',               0, 'ingreso'),
                 ('Eventual',            0, 'ingreso'),
                 ('Otro ingreso',        0, 'ingreso'),
                 ('Gasolina',        29531, 'gasto'),
                 ('Parqueadero',     64950, 'gasto'),
                 ('Mantenimiento',   40000, 'gasto'),
                 ('Social/Salidas', 100000, 'gasto'),
                 ('Imprevisto',          0, 'gasto'),
                 ('Otro gasto',          0, 'gasto');",
        ).await?;
    }

    Ok(())
}
