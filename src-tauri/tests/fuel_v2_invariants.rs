//! Tests 7-9 de la sección 11 de docs/schema-v2.md (paso 4: combustible).

use finanzas_lib::models::CategoryInput;
use finanzas_lib::{repositories, services};
use libsql::Connection;

async fn fresh_db() -> Connection {
    let db = libsql::Builder::new_local(":memory:").build().await.expect("build db");
    let conn = db.connect().expect("connect");
    finanzas_lib::migrations::init_fresh_v2_schema(&conn).await.expect("schema v2");
    conn
}

/// Test 7 — un tanqueo que excede la capacidad deja el nivel en el tope, y
/// la autonomía es coherente con ese nivel topado (no con el sin topar).
#[tokio::test]
async fn tanqueo_que_excede_capacidad_topa_nivel_y_autonomia() {
    let conn = fresh_db().await;
    // 9 km/gal -> ~2380 m/L; tanque de 2 litros = 2000 mL.
    let vehicle = repositories::vehicles_v2::insert(&conn, "Moto", 2380, Some(2_000)).await.unwrap();

    // $60.000 a $12.000/galón = 5 galones ≈ 18927 mL, muy por encima de 2000 mL.
    let (_, warning) = services::fuel::create_fillup(&conn, &vehicle.id, "2026-01-01", 60_000, 12_000, None, None)
        .await
        .unwrap();
    assert!(warning.is_some(), "debe avisar que el tanqueo supera la capacidad");

    let level = services::fuel::tank_level(&conn, &vehicle.id).await.unwrap();
    assert_eq!(level.level_ml, 2_000, "el nivel debe quedar topado a la capacidad");
    assert!(level.raw_level_ml > 2_000, "el valor sin topar debe seguir reflejando el exceso real");

    let expected_autonomy = (2_000f64 * 2_380f64 / 1000.0).round() as i64;
    assert_eq!(level.autonomy_m, expected_autonomy, "la autonomía debe salir del nivel topado, no del crudo");
    assert_eq!(level.tank_percentage, Some(100.0));
}

/// Test 8 — cambiar `efficiency_m_per_l` de un vehículo no altera el
/// consumo de viajes ya registrados (queda congelado en `trips.consumed_ml`).
#[tokio::test]
async fn cambiar_eficiencia_no_altera_viajes_ya_registrados() {
    let conn = fresh_db().await;
    let vehicle = repositories::vehicles_v2::insert(&conn, "Carro", 10_000, None).await.unwrap(); // 10 km/L

    let trip = services::fuel::register_trip(&conn, &vehicle.id, "2026-01-01", 50_000, None).await.unwrap(); // 50 km
    let consumed_before = trip.consumed_ml; // 5000 mL

    repositories::vehicles_v2::update_efficiency(&conn, &vehicle.id, 5_000).await.unwrap(); // baja a 5 km/L

    let mut rows = conn
        .query("SELECT consumed_ml FROM trips WHERE id = ?", libsql::params![trip.id.clone()])
        .await
        .unwrap();
    let consumed_after: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(consumed_after, consumed_before, "el consumo ya registrado no debe recalcularse");

    // Un viaje NUEVO sí debe usar el rendimiento actualizado.
    let trip2 = services::fuel::register_trip(&conn, &vehicle.id, "2026-01-02", 50_000, None).await.unwrap();
    assert_eq!(trip2.consumed_ml, 10_000, "un viaje nuevo sí usa el rendimiento actual");
}

/// Test 9 — un `fuel_adjustment` ignora todo el histórico anterior de ese
/// vehículo.
#[tokio::test]
async fn ancla_ignora_historico_anterior() {
    let conn = fresh_db().await;
    let vehicle = repositories::vehicles_v2::insert(&conn, "Carro", 10_000, None).await.unwrap();

    // Actividad "vieja" deliberadamente inconsistente — debe quedar ignorada.
    repositories::fuel::insert_fillup(&conn, &vehicle.id, "2026-01-01", 50_000, 1, 1, None, None)
        .await
        .unwrap();
    services::fuel::register_trip(&conn, &vehicle.id, "2026-01-02", 100_000, None).await.unwrap();

    let anchor_ml = 7_571; // ~2 galones
    repositories::fuel::insert_adjustment(&conn, &vehicle.id, "2026-02-01", anchor_ml, None).await.unwrap();

    let level = services::fuel::tank_level(&conn, &vehicle.id).await.unwrap();
    assert_eq!(level.level_ml, anchor_ml, "el ancla debe ignorar toda la actividad previa a su fecha");

    // Actividad posterior al ancla sí debe contar.
    services::fuel::register_trip(&conn, &vehicle.id, "2026-02-02", 10_000, None).await.unwrap(); // consume 1000 mL
    let level_after = services::fuel::tank_level(&conn, &vehicle.id).await.unwrap();
    assert_eq!(level_after.level_ml, anchor_ml - 1_000);
}

/// `create_fillup_with_expense` crea el gasto real y el tanqueo juntos,
/// enlazados — y baja el disponible exactamente el monto pagado.
#[tokio::test]
async fn create_fillup_with_expense_crea_gasto_enlazado() {
    let conn = fresh_db().await;
    let vehicle = repositories::vehicles_v2::insert(&conn, "Moto", 9_000, Some(10_000)).await.unwrap();
    let gasolina = services::categories::create(
        &conn,
        CategoryInput { name: "Gasolina".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();
    let cash = repositories::accounts::id_by_code(&conn, "cash").await.unwrap();

    services::entries::create(&conn, finanzas_lib::models::EntryInput {
        occurred_on: "2026-01-01".into(), kind: "income".into(), amount_cop: 100_000,
        account_from: None, account_to: Some(cash), category_id: Some(gasolina.id.clone()),
        goal_id: None, loan_id: None, note: None, is_extraordinary: false,
    }).await.unwrap();
    let before = services::entries::account_balances(&conn).await.unwrap();

    let (entry, fillup, warning) = services::fuel::create_fillup_with_expense(
        &conn, &vehicle.id, "2026-01-02", 20_000, 12_000, &gasolina.id, Some("tanqueo semanal"),
    )
    .await
    .unwrap();

    assert_eq!(entry.kind, "expense");
    assert_eq!(entry.amount_cop, 20_000);
    assert_eq!(fillup.entry_id.as_deref(), Some(entry.id.as_str()));
    assert!(warning.is_none());

    let after = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(before.disponible - after.disponible, 20_000);

    let listed = services::fuel::list_fillups(&conn, Some(&vehicle.id)).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].entry_id.as_deref(), Some(entry.id.as_str()));
}

/// El usuario puede resetear el nivel a mano en cualquier momento (por
/// ejemplo a 0, para volver a contar desde el próximo tanqueo) sin que eso
/// borre ni un solo tanqueo o viaje ya registrado.
#[tokio::test]
async fn reset_level_no_borra_historico() {
    let conn = fresh_db().await;
    let vehicle = repositories::vehicles_v2::insert(&conn, "Carro", 10_000, None).await.unwrap();

    repositories::fuel::insert_fillup(&conn, &vehicle.id, "2026-01-01", 20_000, 1, 1, None, None).await.unwrap();
    services::fuel::register_trip(&conn, &vehicle.id, "2026-01-02", 50_000, None).await.unwrap();

    services::fuel::reset_level(&conn, &vehicle.id, 0, "2026-01-10", Some("reseteo manual del usuario")).await.unwrap();

    let level = services::fuel::tank_level(&conn, &vehicle.id).await.unwrap();
    assert_eq!(level.level_ml, 0);

    let fillup_count: i64 = {
        let mut rows = conn.query("SELECT COUNT(*) FROM fillups WHERE vehicle_id = ?", libsql::params![vehicle.id.clone()]).await.unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    };
    let trip_count: i64 = {
        let mut rows = conn.query("SELECT COUNT(*) FROM trips WHERE vehicle_id = ?", libsql::params![vehicle.id.clone()]).await.unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    };
    assert_eq!(fillup_count, 1, "el tanqueo anterior al reset no debe borrarse");
    assert_eq!(trip_count, 1, "el viaje anterior al reset no debe borrarse");
}
