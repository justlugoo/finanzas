-- =====================================================
-- Finanzas — schema.sql
-- SQLite / libSQL (Turso) compatible
-- Idempotente: se ejecuta al iniciar la app cada vez
-- =====================================================

-- WAL mode: persistente en el archivo .db.
-- Permite que la app lea mientras Turso sincroniza en background.
PRAGMA journal_mode = WAL;

-- Foreign keys: NO es persistente. Debe activarse en cada conexión
-- desde el código Rust. Se incluye aquí como documentación.
PRAGMA foreign_keys = ON;


-- =====================================================
-- TABLA: budgets
-- Categorías y su monto mensual esperado.
--   Para ingresos: lo que esperas recibir al mes.
--   Para gastos: el límite que te impones al mes.
-- monthly_amount = 0 significa "sin estimación".
-- =====================================================
CREATE TABLE IF NOT EXISTS budgets (
    category        TEXT    PRIMARY KEY,
    monthly_amount  INTEGER NOT NULL CHECK (monthly_amount >= 0)
);


-- =====================================================
-- TABLA: goals
-- Objetivos de ahorro.
-- current_amount NO se almacena: se calcula con
--   SELECT SUM(amount) FROM transactions WHERE goal_id = ?
-- =====================================================
CREATE TABLE IF NOT EXISTS goals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    target_amount   INTEGER NOT NULL CHECK (target_amount > 0),
    target_date     TEXT,
    status          TEXT    NOT NULL DEFAULT 'activo'
                            CHECK (status IN ('activo', 'completado', 'pausado')),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);


-- =====================================================
-- TABLA: transactions
-- Cada movimiento de dinero (ingreso o gasto).
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
    goal_id             INTEGER,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (category) REFERENCES budgets(category) ON UPDATE CASCADE,
    FOREIGN KEY (goal_id)  REFERENCES goals(id)         ON DELETE SET NULL
);

-- Índices para queries frecuentes
CREATE INDEX IF NOT EXISTS idx_tx_date          ON transactions(date);
CREATE INDEX IF NOT EXISTS idx_tx_category      ON transactions(category);
CREATE INDEX IF NOT EXISTS idx_tx_date_category ON transactions(date, category);
CREATE INDEX IF NOT EXISTS idx_tx_goal_id       ON transactions(goal_id)
    WHERE goal_id IS NOT NULL;


-- =====================================================
-- TABLA: gas_prices
-- Histórico de precios. Un único registro por fecha.
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
-- Parámetros key-value (comportamiento + datos personales).
-- NO almacena credenciales de Turso (esos van en archivo local).
-- =====================================================
CREATE TABLE IF NOT EXISTS config (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);


-- =====================================================
-- SEEDS — datos iniciales
-- INSERT OR IGNORE: idempotente, no sobrescribe si ya existen
-- =====================================================

-- Categorías con monto mensual esperado
INSERT OR IGNORE INTO budgets (category, monthly_amount) VALUES
    ('Mesada',          300000),
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

-- Parámetros de la app y situación personal
INSERT OR IGNORE INTO config (key, value) VALUES
    -- Ingresos fijos esperados
    ('mesada_mensual',                  '300000'),
    ('precio_carrera_mama',             '10000'),
    ('precio_carrera_cunada',           '10000'),

    -- Distancias en km (ida + vuelta)
    ('km_carrera_mama_redondo',         '8'),
    ('km_carrera_cunada_redondo',       '16'),
    ('km_universidad_redondo',          '11.4'),

    -- Frecuencia semanal
    ('dias_carrera_mama_semana',        '6'),
    ('dias_carrera_cunada_semana',      '6'),
    ('dias_universidad_semana',         '3'),

    -- Costos fijos
    ('parqueadero_universidad_dia',     '5000'),
    ('provision_mantenimiento_mensual', '40000'),

    -- Moto Hero Eco T (km por galón promedio)
    ('consumo_moto_km_galon',           '415'),

    -- Comportamiento de la app
    ('tema',                            'oscuro'),
    ('autoarranque',                    'true'),
    ('scraping_gasolina_activo',        'false'),
    ('umbral_alerta_gasolina_pct',      '5'),
    ('umbral_alerta_meta_pct',          '100');

-- Precio inicial de gasolina (semilla solo si la tabla está vacía)
INSERT INTO gas_prices (date, price_per_gallon, source)
SELECT '2026-05-08', 15881, 'manual'
WHERE NOT EXISTS (SELECT 1 FROM gas_prices);
