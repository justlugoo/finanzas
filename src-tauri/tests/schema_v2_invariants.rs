//! Tests obligatorios de la sección 11 de schema-v2.md (1 al 6 — los que el
//! paso 3 del plan exige que pasen antes de conectar la UI). Corren contra
//! una base de datos v2 nueva en memoria, sin pasar por la migración 001
//! (esa traduce v1 -> v2; esto prueba el modelo v2 en sí mismo).

use finanzas_lib::models::EntryInput;
use finanzas_lib::repositories;
use finanzas_lib::services::entries;
use libsql::Connection;
use ulid::Ulid;

async fn fresh_db() -> Connection {
    let db = libsql::Builder::new_local(":memory:").build().await.expect("build db");
    let conn = db.connect().expect("connect");
    finanzas_lib::migrations::init_fresh_v2_schema(&conn).await.expect("schema v2");
    conn
}

async fn account_id(conn: &Connection, code: &str) -> String {
    repositories::accounts::id_by_code(conn, code).await.expect("cuenta de sistema")
}

async fn make_category(conn: &Connection, name: &str, kind: &str) -> String {
    let id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO categories (id, name, kind) VALUES (?, ?, ?)",
        libsql::params![id.clone(), name.to_string(), kind.to_string()],
    )
    .await
    .expect("insert categoría");
    id
}

fn entry(
    occurred_on: &str,
    kind: &str,
    amount_cop: i64,
    account_from: Option<&str>,
    account_to: Option<&str>,
    category_id: Option<&str>,
    is_extraordinary: bool,
) -> EntryInput {
    EntryInput {
        occurred_on: occurred_on.into(),
        kind: kind.into(),
        amount_cop,
        account_from: account_from.map(String::from),
        account_to: account_to.map(String::from),
        category_id: category_id.map(String::from),
        goal_id: None,
        loan_id: None,
        note: None,
        is_extraordinary,
    }
}

const START: &str = "2020-01-01";
const END: &str = "2030-01-01";

/// Test 1 — apartar a un ahorro deja el patrimonio idéntico y baja el
/// disponible exactamente en el monto apartado.
#[tokio::test]
async fn apartar_ahorro_no_mueve_patrimonio() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let savings = account_id(&conn, "savings").await;
    let salario = make_category(&conn, "Salario", "income").await;

    entries::create(&conn, entry("2026-01-01", "income", 500_000, None, Some(&cash), Some(&salario), false))
        .await
        .unwrap();

    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-02", "transfer", 100_000, Some(&cash), Some(&savings), None, false))
        .await
        .unwrap();

    let after = entries::account_balances(&conn).await.unwrap();

    assert_eq!(after.patrimonio, before.patrimonio, "patrimonio debe quedar igual");
    assert_eq!(before.disponible - after.disponible, 100_000, "disponible debe bajar 100.000");
}

/// Test 2 — abonar a una deuda baja el disponible y baja la deuda.
///
/// Nota: schema-v2.md sección 4 y sección 11 (test 2) dicen literalmente
/// que esto sube el patrimonio. Por la fórmula de la propia sección 3
/// (`patrimonio = saldo(cash)+saldo(savings)+saldo(receivable)+saldo(payable)`)
/// y por el invariante del test 10 ("las transferencias no lo mueven"), un
/// `transfer` entre dos cuentas que integran el patrimonio SIEMPRE lo deja
/// igual — subirlo aquí reintroduciría exactamente el bug de v1 que la
/// sección 1 del documento describe (pagar una deuda "sube" el balance).
/// Implementado aquí como "patrimonio sin cambios"; avisar si la intención
/// real era otra.
#[tokio::test]
async fn abonar_deuda_baja_disponible_y_deuda_patrimonio_sin_cambios() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let payable = account_id(&conn, "payable").await;
    let salario = make_category(&conn, "Salario", "income").await;
    let electro = make_category(&conn, "Electrodomésticos", "expense").await;

    entries::create(&conn, entry("2026-01-01", "income", 500_000, None, Some(&cash), Some(&salario), false))
        .await
        .unwrap();
    entries::create(&conn, entry("2026-01-02", "expense", 300_000, Some(&payable), None, Some(&electro), false))
        .await
        .unwrap();

    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-03", "transfer", 100_000, Some(&cash), Some(&payable), None, false))
        .await
        .unwrap();

    let after = entries::account_balances(&conn).await.unwrap();

    assert_eq!(before.disponible - after.disponible, 100_000, "disponible debe bajar 100.000");
    assert_eq!(before.payable - after.payable, 100_000, "la deuda mostrada debe bajar 100.000");
    assert_eq!(after.patrimonio, before.patrimonio, "un transfer nunca mueve el patrimonio");
}

/// Test 3 — prestar y cobrar no cambian el patrimonio; el disponible sí.
#[tokio::test]
async fn prestar_y_cobrar_no_cambian_patrimonio() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let receivable = account_id(&conn, "receivable").await;
    let salario = make_category(&conn, "Salario", "income").await;

    entries::create(&conn, entry("2026-01-01", "income", 500_000, None, Some(&cash), Some(&salario), false))
        .await
        .unwrap();
    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-02", "transfer", 50_000, Some(&cash), Some(&receivable), None, false))
        .await
        .unwrap();
    let after_lend = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_lend.patrimonio, before.patrimonio);
    assert_eq!(before.disponible - after_lend.disponible, 50_000);

    entries::create(&conn, entry("2026-01-10", "transfer", 50_000, Some(&receivable), Some(&cash), None, false))
        .await
        .unwrap();
    let after_collect = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_collect.patrimonio, before.patrimonio);
    assert_eq!(after_collect.disponible, before.disponible);
}

/// Test 4 — una compra a crédito no toca el disponible, cuenta como gasto
/// una sola vez, y sus abonos no vuelven a contar como gasto.
#[tokio::test]
async fn compra_a_credito_cuenta_una_sola_vez() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let payable = account_id(&conn, "payable").await;
    let salario = make_category(&conn, "Salario", "income").await;
    let electro = make_category(&conn, "Electrodomésticos", "expense").await;

    entries::create(&conn, entry("2026-01-01", "income", 500_000, None, Some(&cash), Some(&salario), false))
        .await
        .unwrap();
    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-02", "expense", 300_000, Some(&payable), None, Some(&electro), false))
        .await
        .unwrap();
    let after_purchase = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_purchase.disponible, before.disponible, "la compra a crédito no toca el disponible");

    let spend_after_purchase = entries::category_spend(&conn, &electro, START, END).await.unwrap();
    assert_eq!(spend_after_purchase, 300_000);

    entries::create(&conn, entry("2026-02-01", "transfer", 100_000, Some(&cash), Some(&payable), None, false))
        .await
        .unwrap();
    let spend_after_abono = entries::category_spend(&conn, &electro, START, END).await.unwrap();
    assert_eq!(spend_after_abono, 300_000, "el abono no debe volver a contar como gasto");
}

/// Test 5 — una categoría con ingresos y gastos (dos filas: misma `name`,
/// distinto `kind`, permitido por `UNIQUE(name, kind)`) no mezcla montos.
#[tokio::test]
async fn categoria_con_ingreso_y_gasto_no_se_mezcla() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let gasolina_income = make_category(&conn, "Gasolina", "income").await;
    let gasolina_expense = make_category(&conn, "Gasolina", "expense").await;

    // Un reembolso de gasolina como ingreso, y un gasto real de gasolina.
    entries::create(&conn, entry("2026-01-01", "income", 50_000, None, Some(&cash), Some(&gasolina_income), false))
        .await
        .unwrap();
    entries::create(&conn, entry("2026-01-02", "expense", 30_000, Some(&cash), None, Some(&gasolina_expense), false))
        .await
        .unwrap();

    let income_spend = entries::category_spend(&conn, &gasolina_income, START, END).await.unwrap();
    let expense_spend = entries::category_spend(&conn, &gasolina_expense, START, END).await.unwrap();

    assert_eq!(income_spend, 50_000);
    assert_eq!(expense_spend, 30_000);
}

/// Test 10 — invariante general: para todo período,
/// `Σ entries.income − Σ entries.expense = Δ patrimonio`, y las
/// transferencias no lo mueven (mezcla income/expense/transfer sobre las 5
/// cuentas, incluida una compra a crédito y su abono parcial).
#[tokio::test]
async fn income_menos_expense_igual_delta_patrimonio() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let apps = account_id(&conn, "apps").await;
    let savings = account_id(&conn, "savings").await;
    let receivable = account_id(&conn, "receivable").await;
    let payable = account_id(&conn, "payable").await;
    let salario = make_category(&conn, "Salario", "income").await;
    let plataformas = make_category(&conn, "Plataformas", "income").await;
    let comida = make_category(&conn, "Comida", "expense").await;
    let electro = make_category(&conn, "Electrodomésticos", "expense").await;

    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-01", "income", 500_000, None, Some(&cash), Some(&salario), false)).await.unwrap();
    entries::create(&conn, entry("2026-01-02", "expense", 100_000, Some(&cash), None, Some(&comida), false)).await.unwrap();
    entries::create(&conn, entry("2026-01-03", "transfer", 50_000, Some(&cash), Some(&savings), None, false)).await.unwrap();
    entries::create(&conn, entry("2026-01-04", "transfer", 20_000, Some(&cash), Some(&receivable), None, false)).await.unwrap();
    entries::create(&conn, entry("2026-01-05", "income", 30_000, None, Some(&apps), Some(&plataformas), false)).await.unwrap();
    entries::create(&conn, entry("2026-01-06", "transfer", 30_000, Some(&apps), Some(&cash), None, false)).await.unwrap();
    entries::create(&conn, entry("2026-01-07", "expense", 200_000, Some(&payable), None, Some(&electro), false)).await.unwrap();
    entries::create(&conn, entry("2026-01-08", "transfer", 50_000, Some(&cash), Some(&payable), None, false)).await.unwrap();

    let after = entries::account_balances(&conn).await.unwrap();
    let delta_patrimonio = after.patrimonio - before.patrimonio;

    let raw_delta = repositories::entries::raw_income_expense_delta(&conn, START, END).await.unwrap();
    assert_eq!(raw_delta, delta_patrimonio, "el libro mayor debe cuadrar exacto con el cambio de patrimonio");
    assert_eq!(raw_delta, 230_000);
}

/// Test 11 — una carrera de plataforma sube el patrimonio pero no el
/// disponible; retirar el saldo sube el disponible y deja el patrimonio
/// igual; registrar ambas cosas no cuenta la plata dos veces.
#[tokio::test]
async fn carrera_de_plataforma_y_retiro_no_duplican_plata() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let apps = account_id(&conn, "apps").await;
    let plataformas = make_category(&conn, "Plataformas", "income").await;

    let before = entries::account_balances(&conn).await.unwrap();

    entries::create(&conn, entry("2026-01-01", "income", 50_000, None, Some(&apps), Some(&plataformas), false))
        .await
        .unwrap();
    let after_ride = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_ride.patrimonio - before.patrimonio, 50_000, "la carrera debe subir el patrimonio");
    assert_eq!(after_ride.disponible, before.disponible, "la carrera no debe subir el disponible todavía");

    entries::create(&conn, entry("2026-01-05", "transfer", 50_000, Some(&apps), Some(&cash), None, false))
        .await
        .unwrap();
    let after_withdrawal = entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_withdrawal.disponible - after_ride.disponible, 50_000, "el retiro debe subir el disponible");
    assert_eq!(after_withdrawal.patrimonio, after_ride.patrimonio, "el retiro es un transfer: no mueve el patrimonio");

    // registrar ambas cosas no cuenta la plata dos veces
    assert_eq!(after_withdrawal.patrimonio - before.patrimonio, 50_000);
    assert_eq!(after_withdrawal.disponible - before.disponible, 50_000);
}

/// Test 6 — `Σ(gastos por categoría) + extraordinarios = total de gastos
/// del período`, sin residuo.
#[tokio::test]
async fn suma_de_categorias_mas_extraordinarios_cuadra_con_el_total() {
    let conn = fresh_db().await;
    let cash = account_id(&conn, "cash").await;
    let salario = make_category(&conn, "Salario", "income").await;
    let comida = make_category(&conn, "Comida", "expense").await;
    let transporte = make_category(&conn, "Transporte", "expense").await;

    entries::create(&conn, entry("2026-01-01", "income", 1_000_000, None, Some(&cash), Some(&salario), false))
        .await
        .unwrap();
    entries::create(&conn, entry("2026-01-02", "expense", 100_000, Some(&cash), None, Some(&comida), false))
        .await
        .unwrap();
    entries::create(&conn, entry("2026-01-03", "expense", 50_000, Some(&cash), None, Some(&transporte), false))
        .await
        .unwrap();
    entries::create(&conn, entry("2026-01-04", "expense", 200_000, Some(&cash), None, Some(&comida), true))
        .await
        .unwrap();

    let totals = entries::period_totals(&conn, START, END).await.unwrap();
    assert_eq!(totals.total_expense, 350_000);
    assert_eq!(totals.extraordinary_expense, 200_000);

    let matches = entries::expense_by_category_matches_total(&conn, START, END).await.unwrap();
    assert!(matches, "la suma por categoría + extraordinarios debe cuadrar exacto con el total");
}
