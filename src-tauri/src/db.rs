use libsql::Builder;
use crate::error::{AppError, AppResult};

// Schema sin PRAGMAs — idempotente, se aplica en cada arranque
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS budgets (
    category        TEXT    PRIMARY KEY,
    monthly_amount  INTEGER NOT NULL CHECK (monthly_amount >= 0),
    route_id        INTEGER REFERENCES custom_routes(id) ON DELETE SET NULL,
    type            TEXT    NOT NULL DEFAULT 'gasto' CHECK (type IN ('ingreso', 'gasto')),
    is_fixed        INTEGER NOT NULL DEFAULT 0 CHECK (is_fixed IN (0, 1))
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
                                CHECK (is_debt IN (0, 1)),
    gas_km              REAL    DEFAULT NULL,
    trip_vehicle_id     INTEGER DEFAULT NULL
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
    km_round_trip   REAL    NOT NULL DEFAULT 0 CHECK (km_round_trip >= 0),
    description     TEXT
);

CREATE TABLE IF NOT EXISTS vehicles (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    km_per_gallon   REAL    NOT NULL CHECK (km_per_gallon > 0)
);

CREATE TABLE IF NOT EXISTS loans (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    person_name     TEXT    NOT NULL,
    amount          INTEGER NOT NULL CHECK (amount > 0),
    date            TEXT    NOT NULL,
    note            TEXT,
    status          TEXT    NOT NULL DEFAULT 'pendiente'
                            CHECK (status IN ('pendiente', 'pagado')),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS loan_payments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    loan_id         INTEGER NOT NULL,
    amount          INTEGER NOT NULL CHECK (amount > 0),
    date            TEXT    NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_loans_status  ON loans(status);
CREATE INDEX IF NOT EXISTS idx_loans_person  ON loans(person_name);
CREATE INDEX IF NOT EXISTS idx_lp_loan_id    ON loan_payments(loan_id);

CREATE TABLE IF NOT EXISTS fuel_fillups (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    date                TEXT    NOT NULL,
    vehicle_id          INTEGER NOT NULL,
    gallons             REAL    NOT NULL CHECK (gallons > 0),
    price_per_gallon    INTEGER NOT NULL CHECK (price_per_gallon > 0),
    total_cost          INTEGER NOT NULL CHECK (total_cost > 0),
    note                TEXT,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_fillups_date    ON fuel_fillups(date);
CREATE INDEX IF NOT EXISTS idx_fillups_vehicle ON fuel_fillups(vehicle_id);

";

pub async fn open_database() -> AppResult<libsql::Database> {
    let mut path = dirs::data_local_dir()
        .ok_or_else(|| AppError::DatabaseError("no se pudo determinar directorio local".into()))?;
    #[cfg(debug_assertions)]
    path.push("finanzas-dev");
    #[cfg(not(debug_assertions))]
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

async fn column_exists(conn: &libsql::Connection, table: &str, column: &str) -> AppResult<bool> {
    let mut rows = conn
        .query(&format!("PRAGMA table_info({table})"), ())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    while let Some(row) = rows.next().await.map_err(|e| AppError::DatabaseError(e.to_string()))? {
        let name: String = row.get(1).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn apply_schema(conn: &libsql::Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA)
        .await
        .map(|_| ())
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Migraciones de esquema — solo añaden columnas, nunca insertan ni modifican datos.
    if !column_exists(conn, "budgets", "type").await? {
        conn.execute(
            "ALTER TABLE budgets ADD COLUMN type TEXT NOT NULL DEFAULT 'gasto' \
             CHECK (type IN ('ingreso', 'gasto'))",
            (),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    if !column_exists(conn, "budgets", "is_fixed").await? {
        conn.execute(
            "ALTER TABLE budgets ADD COLUMN is_fixed INTEGER NOT NULL DEFAULT 0 \
             CHECK (is_fixed IN (0, 1))",
            (),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    // Migración: versiones antiguas crearon `transactions` con FK explícitas a
    // budgets(category) y goals(id). libsql compila SQLite con
    // SQLITE_DEFAULT_FOREIGN_KEYS=1, así que esas FKs se aplican y bloquean
    // cualquier INSERT. Detectamos la tabla antigua y la reconstruimos sin FKs.
    let needs_rebuild: bool = {
        let mut rows = conn
            .query("PRAGMA foreign_key_list(transactions)", ())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        rows.next()
            .await
            .map(|r| r.is_some())
            .unwrap_or(false)
    };

    if needs_rebuild {
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             BEGIN;
             CREATE TABLE transactions_new (
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
                                             CHECK (is_debt IN (0, 1)),
                 gas_km              REAL    DEFAULT NULL,
                 trip_vehicle_id     INTEGER DEFAULT NULL
             );
             INSERT INTO transactions_new
                 SELECT id, date, type, category, amount, note,
                        is_extraordinary, goal_id, created_at, is_debt
                 FROM transactions;
             DROP TABLE transactions;
             ALTER TABLE transactions_new RENAME TO transactions;
             CREATE INDEX IF NOT EXISTS idx_tx_date          ON transactions(date);
             CREATE INDEX IF NOT EXISTS idx_tx_category      ON transactions(category);
             CREATE INDEX IF NOT EXISTS idx_tx_date_category ON transactions(date, category);
             COMMIT;
             PRAGMA foreign_keys = ON;",
        )
        .await
        .map_err(|e| AppError::DatabaseError(format!("migración FK transactions: {e}")))?;
    }

    // Migración: versiones antiguas tenían use_vehicle y fixed_cost en custom_routes.
    // Esas columnas ya no existen en el modelo; se recrea la tabla sin ellas.
    let needs_routes_rebuild: bool = {
        let mut rows = conn
            .query(
                "SELECT 1 FROM pragma_table_info('custom_routes') WHERE name = 'use_vehicle'",
                (),
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        rows.next()
            .await
            .map(|r| r.is_some())
            .unwrap_or(false)
    };

    if needs_routes_rebuild {
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             BEGIN;
             CREATE TABLE custom_routes_new (
                 id              INTEGER PRIMARY KEY AUTOINCREMENT,
                 name            TEXT    NOT NULL,
                 km_round_trip   REAL    NOT NULL DEFAULT 0 CHECK (km_round_trip >= 0),
                 description     TEXT
             );
             INSERT INTO custom_routes_new SELECT id, name, km_round_trip, description
                 FROM custom_routes;
             DROP TABLE custom_routes;
             ALTER TABLE custom_routes_new RENAME TO custom_routes;
             COMMIT;
             PRAGMA foreign_keys = ON;",
        )
        .await
        .map_err(|e| AppError::DatabaseError(format!("migración custom_routes: {e}")))?;
    }

    if !column_exists(conn, "transactions", "gas_km").await? {
        conn.execute("ALTER TABLE transactions ADD COLUMN gas_km REAL DEFAULT NULL", ())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    if !column_exists(conn, "transactions", "trip_vehicle_id").await? {
        conn.execute("ALTER TABLE transactions ADD COLUMN trip_vehicle_id INTEGER DEFAULT NULL", ())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    if !column_exists(conn, "goals", "installments").await? {
        conn.execute("ALTER TABLE goals ADD COLUMN installments INTEGER DEFAULT NULL", ())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    if !column_exists(conn, "vehicles", "tank_liters").await? {
        conn.execute("ALTER TABLE vehicles ADD COLUMN tank_liters REAL DEFAULT NULL", ())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    if !column_exists(conn, "fuel_fillups", "transaction_id").await? {
        conn.execute("ALTER TABLE fuel_fillups ADD COLUMN transaction_id INTEGER DEFAULT NULL", ())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}
