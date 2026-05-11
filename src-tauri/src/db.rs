use libsql::Builder;
use std::path::{Path, PathBuf};
use crate::credentials::Credentials;
use crate::error::{AppError, AppResult};

// Schema sin PRAGMAs — idempotente, se aplica en cada arranque
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS budgets (
    category        TEXT    PRIMARY KEY,
    monthly_amount  INTEGER NOT NULL CHECK (monthly_amount >= 0)
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

INSERT OR IGNORE INTO budgets (category, monthly_amount) VALUES
    ('Mesada',          300000),
    ('Carrera',              0),
    ('Carrera mamá',    259800),
    ('Carrera cuñada',  259800),
    ('Eventual',             0),
    ('Otro ingreso',         0),
    ('Gasolina',         29531),
    ('Parqueadero',      64950),
    ('Mantenimiento',    40000),
    ('Social/Salidas',  100000),
    ('Imprevisto',           0),
    ('Otro gasto',           0);

INSERT OR IGNORE INTO config (key, value) VALUES
    ('mesada_mensual',                  '300000'),
    ('consumo_moto_km_galon',           '350'),
    ('umbral_alerta_gasolina_pct',      '5'),
    ('umbral_alerta_meta_pct',          '100'),
    ('scraping_gasolina_activo',        'false'),
    ('km_carrera_mama_redondo',         '8'),
    ('km_carrera_cunada_redondo',       '16'),
    ('km_universidad_redondo',          '11.4');

INSERT INTO gas_prices (date, price_per_gallon, source)
SELECT '2026-05-08', 15881, 'manual'
WHERE NOT EXISTS (SELECT 1 FROM gas_prices);

UPDATE config SET value = '350' WHERE key = 'consumo_moto_km_galon' AND value = '415';
";

pub fn local_db_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("finanzas");
    std::fs::create_dir_all(&dir).ok();
    dir.join("local.db")
}

pub fn cleanup_local_replica(path: &Path) {
    let metadata_path = PathBuf::from(format!("{}-info", path.display()));
    let cleanup_files = [
        path.to_path_buf(),
        metadata_path,
        PathBuf::from(format!("{}-shm", path.display())),
        PathBuf::from(format!("{}-wal", path.display())),
    ];

    for file in cleanup_files.iter() {
        if file.exists() {
            let _ = std::fs::remove_file(file);
        }
    }
}

pub async fn open_database(credentials: &Credentials) -> AppResult<libsql::Database> {
    let path = local_db_path();
    let metadata_path = PathBuf::from(format!("{}-info", path.display()));

    if path.exists() ^ metadata_path.exists() {
        cleanup_local_replica(&path);
    }

    Builder::new_remote_replica(
        path.to_string_lossy().to_string(),
        credentials.turso_url.clone(),
        credentials.turso_auth_token.clone(),
    )
    .build()
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Pragmas de rendimiento — se aplican a cada conexión nueva.
/// synchronous=NORMAL: omite fsync por write (WAL es seguro así).
/// cache_size=-65536: 64 MB de caché de páginas en RAM (default=2 MB).
/// mmap_size=268435456: 256 MB de mmap; leer es casi acceso a memoria.
/// temp_store=MEMORY: tablas temporales (sorts/joins) en RAM, no disco.
pub async fn apply_pragmas(conn: &libsql::Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA synchronous  = NORMAL;
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

    // Migración: añade is_debt si la tabla existía antes de esta versión
    let _ = conn.execute(
        "ALTER TABLE transactions ADD COLUMN is_debt INTEGER NOT NULL DEFAULT 0",
        (),
    ).await;

    // Migración: añade is_debt_goal a goals si la tabla existía antes
    let _ = conn.execute(
        "ALTER TABLE goals ADD COLUMN is_debt_goal INTEGER NOT NULL DEFAULT 0",
        (),
    ).await;

    Ok(())
}
