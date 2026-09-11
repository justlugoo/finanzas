//! Paso 6 de docs/schema-v2.md §12 — un solo `meta_add_payment` para los
//! tres tipos de meta. El caller (frontend, luego) solo manda `meta_id`,
//! monto y fecha; nunca decide si eso es un ingreso o un gasto.

use finanzas_lib::repositories;
use finanzas_lib::services::{entries, meta_payments};
use libsql::Connection;
use ulid::Ulid;

async fn fresh_db() -> Connection {
    let db = libsql::Builder::new_local(":memory:").build().await.expect("build db");
    let conn = db.connect().expect("connect");
    finanzas_lib::migrations::init_fresh_v2_schema(&conn).await.expect("schema v2");
    conn
}

async fn make_goal(conn: &Connection, name: &str, target_cop: i64, kind: &str) -> String {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO goals (id, name, target_cop, kind) VALUES (?, ?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), target_cop, kind.to_string()],
    )
    .await
    .unwrap();
    id
}

async fn make_loan(conn: &Connection, person: &str, principal_cop: i64, lent_on: &str) -> String {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO loans (id, person_name, principal_cop, lent_on) VALUES (?, ?, ?, ?)",
        libsql::params![id.clone(), person.to_string(), principal_cop, lent_on.to_string()],
    )
    .await
    .unwrap();
    id
}

/// "Me deben": abonar un préstamo por cobrar sube el disponible y no toca
/// el patrimonio (sigue siendo un `transfer`).
#[tokio::test]
async fn abono_a_prestamo_por_cobrar() {
    let conn = fresh_db().await;
    let loan_id = make_loan(&conn, "Un amigo", 50_000, "2026-01-01").await;
    let before = entries::account_balances(&conn).await.unwrap();

    let entry = meta_payments::add_payment(&conn, &format!("loan:{loan_id}"), 50_000, "2026-02-01", None)
        .await
        .unwrap();
    assert_eq!(entry.kind, "transfer");

    let after = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after.disponible - before.disponible, 50_000);
    assert_eq!(after.patrimonio, before.patrimonio);
}

/// "Debo": abonar una deuda hace un transfer cash->payable con el goal_id.
#[tokio::test]
async fn abono_a_deuda() {
    let conn = fresh_db().await;
    let goal_id = make_goal(&conn, "Deuda de prueba", 1_000_000, "debt").await;

    let entry = meta_payments::add_payment(&conn, &format!("goal:{goal_id}"), 200_000, "2026-02-01", Some("primer abono"))
        .await
        .unwrap();
    assert_eq!(entry.kind, "transfer");
    assert_eq!(entry.goal_id.as_deref(), Some(goal_id.as_str()));

    let payable = repositories::accounts::id_by_code(&conn, "payable").await.unwrap();
    assert_eq!(entry.account_to.as_deref(), Some(payable.as_str()));
}

/// "Quiero juntar": aportar a un ahorro hace un transfer cash->savings.
#[tokio::test]
async fn aporte_a_ahorro() {
    let conn = fresh_db().await;
    let goal_id = make_goal(&conn, "Vacaciones", 2_000_000, "saving").await;

    let entry = meta_payments::add_payment(&conn, &format!("goal:{goal_id}"), 100_000, "2026-02-01", None)
        .await
        .unwrap();
    assert_eq!(entry.kind, "transfer");
    let savings = repositories::accounts::id_by_code(&conn, "savings").await.unwrap();
    assert_eq!(entry.account_to.as_deref(), Some(savings.as_str()));
}

#[tokio::test]
async fn meta_id_invalido_es_rechazado() {
    let conn = fresh_db().await;
    let result = meta_payments::add_payment(&conn, "algo:123", 1_000, "2026-01-01", None).await;
    assert!(result.is_err());
}
