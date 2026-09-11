//! Esquema v2 (ver `docs/schema-v2.md`).
//!
//! Este módulo crea el esquema nuevo y traduce los datos de v1. **No se
//! invoca automáticamente al arrancar la app** (a diferencia de
//! `db::apply_schema`): correrlo reemplaza los nombres `transactions`,
//! `budgets`, `goals`, `vehicles`, `loans` y `custom_routes` por las tablas
//! nuevas y renombra las viejas a `_v1`, lo que rompe de inmediato los
//! repositorios v1 (siguen usando esos nombres por texto plano) hasta que
//! el resto del plan (pasos 2–8 de la sección 12) esté conectado.
//!
//! La migración 002 (ancla de saldo de caja) exige un valor real que solo
//! la persona dueña de los datos puede dar — no debe inventarse. Ver
//! `apply_cash_balance_anchor`. El nivel de gasolina **no** es un anclaje
//! de migración: es una función normal de la app (`services::fuel::reset_level`)
//! que cualquier usuario puede disparar cuando quiera, sin borrar tanqueos.
//!
//! El migrador queda aquí como código importable y probado (ver
//! `src-tauri/tests/`), listo para conectarse a un comando Tauri explícito
//! cuando se decida cortar de verdad — con backup antes.

use crate::error::{AppError, AppResult};
use crate::models::{MigrationReport, MigrationRow};
use crate::utils::{csv_escape, gallons_cost_to_ml, km_per_gallon_to_m_per_l, km_to_m, liters_to_ml};
use libsql::Connection;
use std::collections::HashMap;
use ulid::Ulid;

fn new_id() -> String {
    Ulid::new().to_string()
}

fn map_kind(v1_kind: &str) -> &'static str {
    if v1_kind == "ingreso" { "income" } else { "expense" }
}

pub const SCHEMA_V2: &str = "
CREATE TABLE IF NOT EXISTS accounts (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('asset', 'liability')),
    is_system   INTEGER NOT NULL DEFAULT 1 CHECK (is_system IN (0, 1))
);

CREATE TABLE IF NOT EXISTS routes (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    distance_m      INTEGER NOT NULL DEFAULT 0 CHECK (distance_m >= 0),
    description     TEXT
);

CREATE TABLE IF NOT EXISTS categories (
    id           TEXT    PRIMARY KEY,
    name         TEXT    NOT NULL,
    kind         TEXT    NOT NULL CHECK (kind IN ('income', 'expense')),
    is_fixed     INTEGER NOT NULL DEFAULT 0 CHECK (is_fixed IN (0, 1)),
    route_id     TEXT             REFERENCES routes(id) ON DELETE SET NULL,
    is_system    INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    code         TEXT,
    archived_at  TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (name, kind)
);

CREATE TABLE IF NOT EXISTS budgets (
    category_id  TEXT    PRIMARY KEY REFERENCES categories(id) ON DELETE CASCADE,
    monthly_cop  INTEGER NOT NULL CHECK (monthly_cop >= 0),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS budget_overrides (
    category_id  TEXT    NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    year_month   TEXT    NOT NULL,
    amount_cop   INTEGER NOT NULL CHECK (amount_cop >= 0),
    PRIMARY KEY (category_id, year_month)
);

CREATE TABLE IF NOT EXISTS goals (
    id            TEXT    PRIMARY KEY,
    name          TEXT    NOT NULL,
    target_cop    INTEGER NOT NULL CHECK (target_cop > 0),
    target_date   TEXT,
    kind          TEXT    NOT NULL CHECK (kind IN ('saving', 'debt')),
    installments  INTEGER,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at    TEXT
);

CREATE TABLE IF NOT EXISTS loans (
    id            TEXT    PRIMARY KEY,
    person_name   TEXT    NOT NULL,
    principal_cop INTEGER NOT NULL CHECK (principal_cop > 0),
    lent_on       TEXT    NOT NULL,
    note          TEXT,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at    TEXT
);

CREATE TABLE IF NOT EXISTS vehicles (
    id                 TEXT    PRIMARY KEY,
    name               TEXT    NOT NULL,
    efficiency_m_per_l INTEGER NOT NULL CHECK (efficiency_m_per_l > 0),
    tank_capacity_ml   INTEGER          CHECK (tank_capacity_ml > 0),
    created_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at         TEXT
);

CREATE TABLE IF NOT EXISTS entries (
    id               TEXT    PRIMARY KEY,
    occurred_on      TEXT    NOT NULL,
    kind             TEXT    NOT NULL CHECK (kind IN ('income', 'expense', 'transfer')),
    amount_cop       INTEGER NOT NULL CHECK (amount_cop > 0),
    account_from     TEXT             REFERENCES accounts(id),
    account_to       TEXT             REFERENCES accounts(id),
    category_id      TEXT             REFERENCES categories(id),
    goal_id          TEXT             REFERENCES goals(id) ON DELETE SET NULL,
    loan_id          TEXT             REFERENCES loans(id) ON DELETE SET NULL,
    note             TEXT,
    is_extraordinary INTEGER NOT NULL DEFAULT 0 CHECK (is_extraordinary IN (0, 1)),
    created_at       TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at       TEXT,

    CHECK (
        (kind = 'income'   AND account_from IS NULL     AND account_to IS NOT NULL AND category_id IS NOT NULL) OR
        (kind = 'expense'  AND account_from IS NOT NULL AND account_to IS NULL     AND category_id IS NOT NULL) OR
        (kind = 'transfer' AND account_from IS NOT NULL AND account_to IS NOT NULL AND category_id IS NULL
                           AND account_from <> account_to)
    )
);

CREATE INDEX IF NOT EXISTS idx_entries_date       ON entries(occurred_on) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entries_category   ON entries(category_id, occurred_on) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entries_goal       ON entries(goal_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entries_loan       ON entries(loan_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entries_accounts   ON entries(account_from, account_to) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS fillups (
    id                    TEXT    PRIMARY KEY,
    occurred_on           TEXT    NOT NULL,
    vehicle_id            TEXT    NOT NULL REFERENCES vehicles(id),
    volume_ml             INTEGER NOT NULL CHECK (volume_ml > 0),
    price_cop_per_gallon  INTEGER NOT NULL CHECK (price_cop_per_gallon > 0),
    total_cop             INTEGER NOT NULL CHECK (total_cop > 0),
    entry_id              TEXT             REFERENCES entries(id) ON DELETE SET NULL,
    note                  TEXT,
    created_at            TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at            TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at            TEXT
);

CREATE TABLE IF NOT EXISTS trips (
    id           TEXT    PRIMARY KEY,
    occurred_on  TEXT    NOT NULL,
    vehicle_id   TEXT    NOT NULL REFERENCES vehicles(id),
    distance_m   INTEGER NOT NULL CHECK (distance_m > 0),
    consumed_ml  INTEGER NOT NULL CHECK (consumed_ml > 0),
    entry_id     TEXT             REFERENCES entries(id) ON DELETE SET NULL,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at   TEXT
);

CREATE TABLE IF NOT EXISTS fuel_adjustments (
    id           TEXT    PRIMARY KEY,
    occurred_on  TEXT    NOT NULL,
    vehicle_id   TEXT    NOT NULL REFERENCES vehicles(id),
    level_ml     INTEGER NOT NULL CHECK (level_ml >= 0),
    note         TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);
";

async fn ensure_system_accounts(conn: &Connection) -> AppResult<()> {
    const ACCOUNTS: [(&str, &str, &str); 5] = [
        ("cash", "Disponible", "asset"),
        ("apps", "Saldo en apps", "asset"),
        ("savings", "Ahorros", "asset"),
        ("receivable", "Por cobrar", "asset"),
        ("payable", "Por pagar", "liability"),
    ];
    for (code, name, kind) in ACCOUNTS {
        conn.execute(
            "INSERT INTO accounts (id, code, name, kind, is_system) \
             SELECT ?, ?, ?, ?, 1 WHERE NOT EXISTS (SELECT 1 FROM accounts WHERE code = ?)",
            libsql::params![new_id(), code, name, kind, code],
        )
        .await
        .map_err(|e| AppError::DatabaseError(format!("seed accounts: {e}")))?;
    }
    Ok(())
}

/// Crea el esquema v2 en una base de datos que **no** tiene tablas v1 que
/// renombrar (ej. una réplica de pruebas). No toca `user_version` ni
/// ninguna tabla v1 — solo para tests y para arrancar un esquema v2 desde
/// cero.
pub async fn init_fresh_v2_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA_V2)
        .await
        .map_err(|e| AppError::DatabaseError(format!("init_fresh_v2_schema: {e}")))?;
    ensure_system_accounts(conn).await
}

pub async fn user_version(conn: &Connection) -> AppResult<i64> {
    let mut rows = conn
        .query("PRAGMA user_version", ())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    match rows.next().await.map_err(|e| AppError::DatabaseError(e.to_string()))? {
        Some(row) => row.get(0).map_err(|e| AppError::DatabaseError(e.to_string())),
        None => Ok(0),
    }
}

async fn set_user_version(conn: &Connection, v: i64) -> AppResult<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {v}"))
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(())
}

async fn account_id_by_code(conn: &Connection, code: &str) -> AppResult<String> {
    let mut rows = conn
        .query("SELECT id FROM accounts WHERE code = ?", libsql::params![code.to_string()])
        .await?;
    rows.next()
        .await?
        .ok_or_else(|| AppError::DatabaseError(format!("cuenta de sistema '{code}' no existe")))?
        .get(0)
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn insert_entry(
    conn: &Connection,
    occurred_on: &str,
    kind: &str,
    amount_cop: i64,
    account_from: Option<&str>,
    account_to: Option<&str>,
    category_id: Option<&str>,
    goal_id: Option<&str>,
    loan_id: Option<&str>,
    note: Option<&str>,
    is_extraordinary: bool,
    created_at: &str,
) -> AppResult<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO entries \
         (id, occurred_on, kind, amount_cop, account_from, account_to, category_id, \
          goal_id, loan_id, note, is_extraordinary, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            id.clone(),
            occurred_on.to_string(),
            kind.to_string(),
            amount_cop,
            account_from.map(|s| s.to_string()),
            account_to.map(|s| s.to_string()),
            category_id.map(|s| s.to_string()),
            goal_id.map(|s| s.to_string()),
            loan_id.map(|s| s.to_string()),
            note.map(|s| s.to_string()),
            is_extraordinary as i64,
            created_at.to_string(),
            created_at.to_string()
        ],
    )
    .await
    .map_err(|e| AppError::DatabaseError(format!("insert_entry ({kind}): {e}")))?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
async fn ensure_category(
    conn: &Connection,
    categories: &mut HashMap<(String, String), String>,
    name: &str,
    kind: &str,
    is_fixed: bool,
    route_id: Option<&str>,
    is_system: bool,
    code: Option<&str>,
) -> AppResult<String> {
    let key = (name.to_string(), kind.to_string());
    if let Some(id) = categories.get(&key) {
        return Ok(id.clone());
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO categories (id, name, kind, is_fixed, route_id, is_system, code) VALUES (?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            id.clone(),
            name.to_string(),
            kind.to_string(),
            is_fixed as i64,
            route_id.map(|s| s.to_string()),
            is_system as i64,
            code.map(|s| s.to_string())
        ],
    )
    .await
    .map_err(|e| AppError::DatabaseError(format!("ensure_category '{name}': {e}")))?;
    categories.insert(key, id.clone());
    Ok(id)
}

struct OldTx {
    id: i64,
    date: String,
    kind: String,
    category: String,
    amount: i64,
    note: Option<String>,
    is_extraordinary: bool,
    goal_id: Option<i64>,
    created_at: String,
    is_debt: bool,
    gas_km: Option<f64>,
    trip_vehicle_id: Option<i64>,
}

struct OldFillup {
    date: String,
    vehicle_id: i64,
    price_per_gallon: i64,
    total_cost: i64,
    note: Option<String>,
}

struct OldLoan {
    id: i64,
    person_name: String,
    amount: i64,
    date: String,
    note: Option<String>,
    created_at: String,
}

struct OldLoanPayment {
    id: i64,
    loan_id: i64,
    amount: i64,
    date: String,
    created_at: String,
}

/// Migración 001 — esquema v2 y traslado (sección 10 de schema-v2.md).
/// Idempotente vía `PRAGMA user_version`: si ya corrió, no hace nada.
///
/// Requiere que `db::apply_schema` (esquema v1) ya se haya aplicado sobre
/// esta conexión — de lo contrario no hay `transactions`/`budgets`/etc.
/// que renombrar.
pub async fn migrate_001_schema_v2(conn: &Connection) -> AppResult<MigrationReport> {
    if user_version(conn).await? >= 1 {
        return Ok(MigrationReport::default());
    }

    // Fase 1 — DDL puro (rename + esquema nuevo) en una sola transacción: si
    // algo falla a mitad de camino, SQLite revierte todo y las tablas v1
    // quedan con sus nombres originales, listas para reintentar.
    let ddl = format!(
        "BEGIN;
         ALTER TABLE transactions   RENAME TO transactions_v1;
         ALTER TABLE fuel_fillups   RENAME TO fuel_fillups_v1;
         ALTER TABLE loans          RENAME TO loans_v1;
         ALTER TABLE loan_payments  RENAME TO loan_payments_v1;
         ALTER TABLE goals          RENAME TO goals_v1;
         ALTER TABLE budgets        RENAME TO budgets_v1;
         ALTER TABLE vehicles       RENAME TO vehicles_v1;
         ALTER TABLE custom_routes  RENAME TO custom_routes_v1;
         {SCHEMA_V2}
         COMMIT;"
    );
    conn.execute_batch(&ddl)
        .await
        .map_err(|e| AppError::DatabaseError(format!("migración 001 (DDL): {e}")))?;

    // Fase 2 — traslado de datos, en su propia transacción. Si falla, se
    // hace ROLLBACK explícito: el esquema nuevo ya existe (fase 1 quedó
    // confirmada) pero sin datos a medio traducir, y se puede reintentar
    // llamando de nuevo a esta función sin tocar `user_version`.
    conn.execute_batch("BEGIN;")
        .await
        .map_err(|e| AppError::DatabaseError(format!("migración 001 (begin datos): {e}")))?;

    match translate_data(conn).await {
        Ok(report) => {
            set_user_version(conn, 1).await?;
            conn.execute_batch("COMMIT;")
                .await
                .map_err(|e| AppError::DatabaseError(format!("migración 001 (commit): {e}")))?;
            Ok(report)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;").await;
            Err(e)
        }
    }
}

async fn translate_data(conn: &Connection) -> AppResult<MigrationReport> {
    let mut report = MigrationReport::default();

    ensure_system_accounts(conn).await?;

    let cash = account_id_by_code(conn, "cash").await?;
    let savings = account_id_by_code(conn, "savings").await?;
    let receivable = account_id_by_code(conn, "receivable").await?;
    let payable = account_id_by_code(conn, "payable").await?;

    // ---- routes (custom_routes_v1 -> routes) ----
    let mut route_map: HashMap<i64, String> = HashMap::new();
    {
        let mut rows = conn
            .query("SELECT id, name, km_round_trip, description FROM custom_routes_v1", ())
            .await?;
        let mut old_routes = Vec::new();
        while let Some(row) = rows.next().await? {
            old_routes.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<f64>(2)?,
                row.get::<Option<String>>(3)?,
            ));
        }
        for (old_id, name, km_round_trip, description) in old_routes {
            let new_id = new_id();
            conn.execute(
                "INSERT INTO routes (id, name, distance_m, description) VALUES (?, ?, ?, ?)",
                libsql::params![new_id.clone(), name, km_to_m(km_round_trip), description],
            )
            .await?;
            route_map.insert(old_id, new_id);
        }
    }

    // ---- categorías (dedup por (nombre, kind), desde budgets_v1 y transactions_v1) ----
    let mut categories: HashMap<(String, String), String> = HashMap::new();
    {
        let mut rows = conn
            .query(
                "SELECT category, monthly_amount, route_id, type, is_fixed FROM budgets_v1",
                (),
            )
            .await?;
        let mut old_budgets = Vec::new();
        while let Some(row) = rows.next().await? {
            old_budgets.push((
                row.get::<String>(0)?,
                row.get::<i64>(1)?,
                row.get::<Option<i64>>(2)?,
                row.get::<String>(3)?,
                row.get::<i64>(4)? != 0,
            ));
        }
        for (category, monthly_amount, route_id, kind, is_fixed) in old_budgets {
            let mapped_kind = map_kind(&kind);
            let new_route_id = route_id.and_then(|rid| route_map.get(&rid).cloned());
            let category_id = ensure_category(
                conn,
                &mut categories,
                &category,
                mapped_kind,
                is_fixed,
                new_route_id.as_deref(),
                false,
                None,
            )
            .await?;
            conn.execute(
                "INSERT INTO budgets (category_id, monthly_cop) VALUES (?, ?)",
                libsql::params![category_id, monthly_amount],
            )
            .await?;
        }
    }
    {
        let mut rows = conn
            .query("SELECT DISTINCT category, type FROM transactions_v1", ())
            .await?;
        let mut pairs = Vec::new();
        while let Some(row) = rows.next().await? {
            pairs.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        for (category, kind) in pairs {
            ensure_category(conn, &mut categories, &category, map_kind(&kind), false, None, false, None).await?;
        }
    }

    // ---- goals_v1 -> goals ----
    let mut goal_map: HashMap<i64, (String, bool)> = HashMap::new();
    {
        let mut rows = conn
            .query(
                "SELECT id, name, target_amount, target_date, is_debt_goal, installments, created_at \
                 FROM goals_v1",
                (),
            )
            .await?;
        let mut old_goals = Vec::new();
        while let Some(row) = rows.next().await? {
            old_goals.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<i64>(2)?,
                row.get::<Option<String>>(3)?,
                row.get::<i64>(4)? != 0,
                row.get::<Option<i64>>(5)?,
                row.get::<String>(6)?,
            ));
        }
        for (old_id, name, target_amount, target_date, is_debt_goal, installments, created_at) in old_goals {
            let nid = new_id();
            let kind = if is_debt_goal { "debt" } else { "saving" };
            conn.execute(
                "INSERT INTO goals (id, name, target_cop, target_date, kind, installments, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                libsql::params![nid.clone(), name, target_amount, target_date, kind, installments, created_at.clone(), created_at],
            )
            .await?;
            goal_map.insert(old_id, (nid, is_debt_goal));
        }
    }

    // ---- loans_v1 (fila) -> loans + entry transfer cash->receivable ----
    let mut loan_map: HashMap<i64, String> = HashMap::new();
    let mut old_loans: Vec<OldLoan> = Vec::new();
    {
        let mut rows = conn
            .query("SELECT id, person_name, amount, date, note, created_at FROM loans_v1", ())
            .await?;
        while let Some(row) = rows.next().await? {
            old_loans.push(OldLoan {
                id: row.get(0)?,
                person_name: row.get(1)?,
                amount: row.get(2)?,
                date: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            });
        }
    }
    for loan in &old_loans {
        let nid = new_id();
        conn.execute(
            "INSERT INTO loans (id, person_name, principal_cop, lent_on, note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                nid.clone(),
                loan.person_name.clone(),
                loan.amount,
                loan.date.clone(),
                loan.note.clone(),
                loan.created_at.clone(),
                loan.created_at.clone()
            ],
        )
        .await?;
        loan_map.insert(loan.id, nid.clone());

        let entry_id = insert_entry(
            conn,
            &loan.date,
            "transfer",
            loan.amount,
            Some(&cash),
            Some(&receivable),
            None,
            None,
            Some(&nid),
            None,
            false,
            &loan.created_at,
        )
        .await?;
        report.rows.push(MigrationRow {
            source_table: "loans_v1".into(),
            source_id: loan.id.to_string(),
            rule: "loan -> transfer cash->receivable".into(),
            new_entry_id: Some(entry_id),
            needs_review: false,
        });
    }

    // ---- loan_payments_v1 -> entry transfer receivable->cash ----
    {
        let mut rows = conn
            .query("SELECT id, loan_id, amount, date, created_at FROM loan_payments_v1", ())
            .await?;
        let mut payments = Vec::new();
        while let Some(row) = rows.next().await? {
            payments.push(OldLoanPayment {
                id: row.get(0)?,
                loan_id: row.get(1)?,
                amount: row.get(2)?,
                date: row.get(3)?,
                created_at: row.get(4)?,
            });
        }
        for p in payments {
            match loan_map.get(&p.loan_id) {
                Some(new_loan_id) => {
                    let entry_id = insert_entry(
                        conn,
                        &p.date,
                        "transfer",
                        p.amount,
                        Some(&receivable),
                        Some(&cash),
                        None,
                        None,
                        Some(new_loan_id),
                        None,
                        false,
                        &p.created_at,
                    )
                    .await?;
                    report.rows.push(MigrationRow {
                        source_table: "loan_payments_v1".into(),
                        source_id: p.id.to_string(),
                        rule: "abono préstamo -> transfer receivable->cash".into(),
                        new_entry_id: Some(entry_id),
                        needs_review: false,
                    });
                }
                None => report.exceptions.push(format!(
                    "loan_payments_v1.id={} referencia loan_id={} inexistente en loans_v1 (huérfano)",
                    p.id, p.loan_id
                )),
            }
        }
    }

    // ---- vehicles_v1 -> vehicles ----
    let mut vehicle_map: HashMap<i64, String> = HashMap::new();
    let mut vehicle_efficiency: HashMap<String, i64> = HashMap::new();
    {
        let mut rows = conn
            .query("SELECT id, name, km_per_gallon, tank_liters FROM vehicles_v1", ())
            .await?;
        let mut old_vehicles = Vec::new();
        while let Some(row) = rows.next().await? {
            old_vehicles.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<f64>(2)?,
                row.get::<Option<f64>>(3)?,
            ));
        }
        for (old_id, name, km_per_gallon, tank_liters) in old_vehicles {
            let nid = new_id();
            let efficiency = km_per_gallon_to_m_per_l(km_per_gallon);
            let tank_capacity_ml = tank_liters.map(liters_to_ml);
            conn.execute(
                "INSERT INTO vehicles (id, name, efficiency_m_per_l, tank_capacity_ml) VALUES (?, ?, ?, ?)",
                libsql::params![nid.clone(), name, efficiency, tank_capacity_ml],
            )
            .await?;
            vehicle_map.insert(old_id, nid.clone());
            vehicle_efficiency.insert(nid, efficiency);
        }
    }

    // ---- fuel_fillups_v1 indexado por transaction_id ----
    let mut fillup_by_tx: HashMap<i64, OldFillup> = HashMap::new();
    {
        let mut rows = conn
            .query(
                "SELECT date, vehicle_id, price_per_gallon, total_cost, note, transaction_id \
                 FROM fuel_fillups_v1",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            let transaction_id: Option<i64> = row.get(5)?;
            if let Some(tx_id) = transaction_id {
                fillup_by_tx.insert(
                    tx_id,
                    OldFillup {
                        date: row.get(0)?,
                        vehicle_id: row.get(1)?,
                        price_per_gallon: row.get(2)?,
                        total_cost: row.get(3)?,
                        note: row.get(4)?,
                    },
                );
            }
        }
    }

    // ---- transactions_v1 -> entries (+ fillups / trips) ----
    let mut old_txs: Vec<OldTx> = Vec::new();
    {
        let mut rows = conn
            .query(
                "SELECT id, date, type, category, amount, note, is_extraordinary, goal_id, \
                        created_at, is_debt, gas_km, trip_vehicle_id \
                 FROM transactions_v1 ORDER BY id",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            old_txs.push(OldTx {
                id: row.get(0)?,
                date: row.get(1)?,
                kind: row.get(2)?,
                category: row.get(3)?,
                amount: row.get(4)?,
                note: row.get(5)?,
                is_extraordinary: row.get::<i64>(6)? != 0,
                goal_id: row.get(7)?,
                created_at: row.get(8)?,
                is_debt: row.get::<i64>(9)? != 0,
                gas_km: row.get(10)?,
                trip_vehicle_id: row.get(11)?,
            });
        }
    }

    for tx in &old_txs {
        let mapped_kind = map_kind(&tx.kind);
        let category_id = match categories.get(&(tx.category.clone(), mapped_kind.to_string())) {
            Some(id) => id.clone(),
            None => {
                let id = ensure_category(conn, &mut categories, &tx.category, mapped_kind, false, None, false, None).await?;
                report.exceptions.push(format!(
                    "transactions_v1.id={} usaba categoría '{}' no vista antes — creada sobre la marcha",
                    tx.id, tx.category
                ));
                id
            }
        };

        if let Some(fillup) = fillup_by_tx.get(&tx.id) {
            let entry_id = insert_entry(
                conn,
                &tx.date,
                "expense",
                tx.amount,
                Some(&cash),
                None,
                Some(&category_id),
                None,
                None,
                tx.note.as_deref(),
                tx.is_extraordinary,
                &tx.created_at,
            )
            .await?;
            match vehicle_map.get(&fillup.vehicle_id) {
                Some(new_vehicle_id) => {
                    let volume_ml = gallons_cost_to_ml(fillup.total_cost, fillup.price_per_gallon);
                    conn.execute(
                        "INSERT INTO fillups \
                         (id, occurred_on, vehicle_id, volume_ml, price_cop_per_gallon, total_cop, entry_id, note) \
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                        libsql::params![
                            new_id(),
                            fillup.date.clone(),
                            new_vehicle_id.clone(),
                            volume_ml,
                            fillup.price_per_gallon,
                            fillup.total_cost,
                            entry_id.clone(),
                            fillup.note.clone()
                        ],
                    )
                    .await?;
                }
                None => report.exceptions.push(format!(
                    "fuel_fillups_v1 con transaction_id={} referencia vehicle_id={} inexistente",
                    tx.id, fillup.vehicle_id
                )),
            }
            report.rows.push(MigrationRow {
                source_table: "transactions_v1".into(),
                source_id: tx.id.to_string(),
                rule: "tanqueo -> expense cash-> + fillups".into(),
                new_entry_id: Some(entry_id),
                needs_review: false,
            });
            continue;
        }

        let (kind, from, to, entry_category, entry_goal, needs_review, rule): (
            &str,
            Option<&str>,
            Option<&str>,
            Option<&str>,
            Option<String>,
            bool,
            &str,
        ) = if mapped_kind == "income" {
            if tx.category == "Abono" && tx.goal_id.is_some() {
                match goal_map.get(&tx.goal_id.unwrap()) {
                    Some((new_goal_id, is_debt_goal)) if *is_debt_goal => (
                        "transfer", Some(cash.as_str()), Some(payable.as_str()), None,
                        Some(new_goal_id.clone()), false, "abono deuda -> transfer cash->payable",
                    ),
                    Some((new_goal_id, _)) => (
                        "transfer", Some(cash.as_str()), Some(savings.as_str()), None,
                        Some(new_goal_id.clone()), false, "abono ahorro -> transfer cash->savings",
                    ),
                    None => (
                        "income", None, Some(cash.as_str()), Some(category_id.as_str()),
                        None, true, "ingreso 'Abono' con goal_id huérfano -> income (revisar)",
                    ),
                }
            } else {
                ("income", None, Some(cash.as_str()), Some(category_id.as_str()), None, false, "ingreso -> income")
            }
        } else if tx.is_debt {
            match tx.goal_id.and_then(|gid| goal_map.get(&gid)) {
                Some((new_goal_id, _)) => (
                    "expense", Some(payable.as_str()), None, Some(category_id.as_str()),
                    Some(new_goal_id.clone()), false, "compra a crédito -> expense payable->",
                ),
                None => (
                    "expense", Some(payable.as_str()), None, Some(category_id.as_str()),
                    None, true, "compra a crédito sin goal de deuda mapeado (revisar)",
                ),
            }
        } else if let Some(gid) = tx.goal_id {
            match goal_map.get(&gid) {
                Some((new_goal_id, _)) => (
                    "transfer", Some(cash.as_str()), Some(savings.as_str()), None,
                    Some(new_goal_id.clone()), true,
                    "gasto con goal_id (aporte mal encaminado) -> transfer cash->savings",
                ),
                None => (
                    "expense", Some(cash.as_str()), None, Some(category_id.as_str()),
                    None, true, "gasto con goal_id huérfano -> expense (revisar)",
                ),
            }
        } else {
            ("expense", Some(cash.as_str()), None, Some(category_id.as_str()), None, false, "gasto -> expense cash->")
        };

        let entry_id = insert_entry(
            conn,
            &tx.date,
            kind,
            tx.amount,
            from,
            to,
            entry_category,
            entry_goal.as_deref(),
            None,
            tx.note.as_deref(),
            tx.is_extraordinary,
            &tx.created_at,
        )
        .await?;

        report.rows.push(MigrationRow {
            source_table: "transactions_v1".into(),
            source_id: tx.id.to_string(),
            rule: rule.into(),
            new_entry_id: Some(entry_id.clone()),
            needs_review,
        });

        if let (Some(km), Some(old_vehicle_id)) = (tx.gas_km, tx.trip_vehicle_id) {
            if km > 0.0 {
                match vehicle_map.get(&old_vehicle_id) {
                    Some(new_vehicle_id) => {
                        let efficiency = *vehicle_efficiency.get(new_vehicle_id).unwrap_or(&1);
                        let distance_m = km_to_m(km);
                        let consumed_ml = (distance_m as f64 * 1000.0 / efficiency as f64).round() as i64;
                        conn.execute(
                            "INSERT INTO trips (id, occurred_on, vehicle_id, distance_m, consumed_ml, entry_id) \
                             VALUES (?, ?, ?, ?, ?, ?)",
                            libsql::params![
                                new_id(),
                                tx.date.clone(),
                                new_vehicle_id.clone(),
                                distance_m,
                                consumed_ml.max(1),
                                entry_id.clone()
                            ],
                        )
                        .await?;
                        report.rows.push(MigrationRow {
                            source_table: "transactions_v1".into(),
                            source_id: tx.id.to_string(),
                            rule: "gas_km -> trips".into(),
                            new_entry_id: Some(entry_id.clone()),
                            needs_review: false,
                        });
                    }
                    None => report.exceptions.push(format!(
                        "transactions_v1.id={} tenía gas_km pero trip_vehicle_id={} no existe",
                        tx.id, old_vehicle_id
                    )),
                }
            }
        }
    }

    Ok(report)
}

/// Migración 002, ancla 1 (sección 10 de docs/schema-v2.md) — saldo real de
/// caja hoy. **Requiere el valor real del usuario; no se puede inventar.**
/// Calcula el saldo `cash` resultante del traslado 001 y genera un `entry`
/// de ajuste (`income` o `expense` según el signo) por la diferencia contra
/// `real_cash_balance_cop`, en la categoría de sistema "Ajuste de saldo
/// inicial" (`is_system = 1`, excluida de todo reporte). No se llama desde
/// ningún otro lugar del código — es responsabilidad de quien corra el
/// corte real invocarla con el dato correcto.
pub async fn apply_cash_balance_anchor(
    conn: &Connection,
    real_cash_balance_cop: i64,
    occurred_on: &str,
) -> AppResult<Option<String>> {
    let balances = crate::repositories::accounts::balances(conn).await?;
    let diff = real_cash_balance_cop - balances.cash;
    if diff == 0 {
        return Ok(None);
    }

    let cash = account_id_by_code(conn, "cash").await?;
    let mut categories: HashMap<(String, String), String> = HashMap::new();
    let kind = if diff > 0 { "income" } else { "expense" };
    let adj_cat = ensure_category(
        conn, &mut categories, "Ajuste de saldo inicial", kind, false, None, true, Some("adjustment"),
    ).await?;

    let (from, to) = if diff > 0 { (None, Some(cash.as_str())) } else { (Some(cash.as_str()), None) };
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let entry_id = insert_entry(
        conn, occurred_on, kind, diff.abs(), from, to, Some(&adj_cat), None, None,
        Some("Ajuste contra el saldo real de caja al cortar a v2"), false, &now,
    ).await?;
    Ok(Some(entry_id))
}

/// `migracion_reporte.csv`: una fila por registro trasladado, luego una
/// sección de excepciones. Revisar antes de dar la migración por buena.
pub fn report_to_csv(report: &MigrationReport) -> String {
    let mut out = String::from("tabla_origen,id_origen,regla,entry_id_nuevo,revisar\n");
    for row in &report.rows {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&row.source_table),
            csv_escape(&row.source_id),
            csv_escape(&row.rule),
            row.new_entry_id.as_deref().map(csv_escape).unwrap_or_default(),
            row.needs_review,
        ));
    }
    out.push_str("\n# Excepciones\n");
    for exc in &report.exceptions {
        out.push_str(&csv_escape(exc));
        out.push('\n');
    }
    out
}
