-- =====================================================
-- Finanzas — schema_finanzas.sql
-- SQLite local vía libsql (sin sync cloud)
-- Idempotente: se aplica en cada arranque de la app.
-- =====================================================

-- PRAGMAs aplicados en cada conexión desde Rust (no persistentes en el archivo):
--   PRAGMA journal_mode = WAL;
--   PRAGMA synchronous  = NORMAL;
--   PRAGMA cache_size   = -65536;    -- 64 MB caché de páginas
--   PRAGMA mmap_size    = 268435456; -- 256 MB mmap
--   PRAGMA temp_store   = MEMORY;


-- =====================================================
-- TABLA: custom_routes
-- Rutas frecuentes con km ida y vuelta.
-- Se define antes que budgets porque budgets la referencia.
-- =====================================================
CREATE TABLE IF NOT EXISTS custom_routes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    km_round_trip   REAL    NOT NULL CHECK (km_round_trip > 0),
    description     TEXT
);


-- =====================================================
-- TABLA: vehicles
-- Vehículos del usuario con rendimiento en km/galón.
-- =====================================================
CREATE TABLE IF NOT EXISTS vehicles (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    km_per_gallon   REAL    NOT NULL CHECK (km_per_gallon > 0)
);


-- =====================================================
-- TABLA: budgets
-- Categorías de ingreso y gasto con meta mensual.
--   monthly_amount = 0 → sin estimación.
--   type: distingue si la categoría es ingreso o gasto.
--   is_fixed: solo relevante para ingresos (fijo vs eventual).
--   route_id: ruta asociada para cálculo automático de gasolina.
-- =====================================================
CREATE TABLE IF NOT EXISTS budgets (
    category        TEXT    PRIMARY KEY,
    monthly_amount  INTEGER NOT NULL CHECK (monthly_amount >= 0),
    route_id        INTEGER REFERENCES custom_routes(id) ON DELETE SET NULL,
    type            TEXT    NOT NULL DEFAULT 'gasto'
                            CHECK (type IN ('ingreso', 'gasto')),
    is_fixed        INTEGER NOT NULL DEFAULT 0
                            CHECK (is_fixed IN (0, 1))
);


-- =====================================================
-- TABLA: goals
-- Objetivos de ahorro y deudas.
-- current_amount NO se almacena: se calcula con
--   SELECT SUM(amount) FROM transactions WHERE goal_id = ?
-- is_debt_goal = 1 → el objetivo representa una deuda a saldar.
-- =====================================================
CREATE TABLE IF NOT EXISTS goals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    target_amount   INTEGER NOT NULL CHECK (target_amount > 0),
    target_date     TEXT,
    status          TEXT    NOT NULL DEFAULT 'activo'
                            CHECK (status IN ('activo', 'completado', 'pausado')),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    is_debt_goal    INTEGER NOT NULL DEFAULT 0
                            CHECK (is_debt_goal IN (0, 1))
);


-- =====================================================
-- TABLA: transactions
-- Cada movimiento de dinero (ingreso o gasto).
--   is_extraordinary = 1 → excluye del progreso de presupuesto.
--   is_debt = 1 → gasto financiado a futuro.
--   goal_id → aporte a un objetivo de ahorro o deuda.
-- =====================================================
CREATE TABLE IF NOT EXISTS transactions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    date                TEXT    NOT NULL,
    type                TEXT    NOT NULL CHECK (type IN ('ingreso', 'gasto')),
    category            TEXT    NOT NULL,
    amount              INTEGER NOT NULL CHECK (amount > 0),
    note                TEXT,
    is_extraordinary    INTEGER NOT NULL DEFAULT 0
                                CHECK (is_extraordinary IN (0, 1)),
    goal_id             INTEGER REFERENCES goals(id) ON DELETE SET NULL,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    is_debt             INTEGER NOT NULL DEFAULT 0
                                CHECK (is_debt IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_tx_date          ON transactions(date);
CREATE INDEX IF NOT EXISTS idx_tx_category      ON transactions(category);
CREATE INDEX IF NOT EXISTS idx_tx_date_category ON transactions(date, category);


-- =====================================================
-- TABLA: gas_prices
-- Histórico de precios de gasolina. Un registro por fecha (UPSERT).
-- price_per_gallon en COP. Rango válido: 1.000–100.000.
-- =====================================================
CREATE TABLE IF NOT EXISTS gas_prices (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    date                TEXT    NOT NULL UNIQUE,
    price_per_gallon    INTEGER NOT NULL
                                CHECK (price_per_gallon BETWEEN 1000 AND 100000),
    source              TEXT    NOT NULL
                                CHECK (source IN ('manual', 'scraping'))
);

CREATE INDEX IF NOT EXISTS idx_gas_date ON gas_prices(date);


-- =====================================================
-- TABLA: config
-- Parámetros clave-valor internos de la app.
-- No almacena datos de usuario ni credenciales.
-- =====================================================
CREATE TABLE IF NOT EXISTS config (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
