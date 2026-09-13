//! Ejercita el CRUD completo agregado para poder cortar a v2 (categorías,
//! presupuestos, entries, goals, loans, metas, vehículos, tanqueos, rutas,
//! sistema) a nivel de servicio — el mismo código que llaman los comandos
//! Tauri, sin necesitar el runtime de Tauri para probarlo.

use finanzas_lib::models::{CategoryInput, EntryFilter, EntryInput, GoalInputV2, LoanInputV2, PeriodV2, RouteInputV2, VehicleInputV2};
use finanzas_lib::services;
use libsql::Connection;

async fn fresh_db() -> Connection {
    let db = libsql::Builder::new_local(":memory:").build().await.expect("build db");
    let conn = db.connect().expect("connect");
    finanzas_lib::migrations::init_fresh_v2_schema(&conn).await.expect("schema v2");
    // `gas_prices` es una tabla heredada del schema v1 que nunca se remodeló
    // en v2 — en producción la crea `db::apply_schema()` en cada arranque
    // (CREATE TABLE IF NOT EXISTS), pero aplicar ahí el batch v1 completo
    // choca con las tablas que sí cambiaron de forma en v2 (vehicles/loans/
    // goals/budgets), así que aquí solo se agrega la tabla puntual que falta.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS gas_prices (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            date                TEXT    NOT NULL UNIQUE,
            price_per_gallon    INTEGER NOT NULL CHECK (price_per_gallon BETWEEN 1000 AND 100000),
            source              TEXT    NOT NULL CHECK (source IN ('manual', 'scraping'))
        );",
    ).await.expect("gas_prices table");
    conn
}

#[tokio::test]
async fn categorias_crud_y_conflicto_de_borrado() {
    let conn = fresh_db().await;

    let cat = services::categories::create(
        &conn,
        CategoryInput { name: "Comida".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();
    assert_eq!(cat.name, "Comida");
    assert!(!cat.is_system);

    // Duplicado (mismo nombre + kind) debe rechazarse.
    let dup = services::categories::create(
        &conn,
        CategoryInput { name: "Comida".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await;
    assert!(dup.is_err());

    let listed = services::categories::list(&conn, Some("expense"), false).await.unwrap();
    assert_eq!(listed.len(), 1);
    // Las categorías de sistema (creadas por init_fresh_v2_schema no hay
    // ninguna todavía) nunca deben aparecer en el listado para UI.
    assert!(listed.iter().all(|c| !c.is_system));

    let updated = services::categories::update(&conn, &cat.id, "Comida y bebida", true, None).await.unwrap();
    assert!(updated.is_fixed);
    assert_eq!(updated.name, "Comida y bebida", "renombrar una categoría debe estar permitido");

    // "Eliminar" una categoría sin movimientos ni presupuesto asociado debe
    // borrarla de verdad.
    let unused = services::categories::create(
        &conn,
        CategoryInput { name: "Sin usar".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();
    services::categories::delete(&conn, &unused.id).await.unwrap();
    assert!(finanzas_lib::repositories::categories::get(&conn, &unused.id).await.is_err(), "sin movimientos, debe borrarse de verdad");

    // Con un movimiento asociado, "eliminar" borra la categoría de verdad
    // igual — el movimiento queda con una referencia "huérfana"
    // (category_id sigue apuntando al id ya borrado; el frontend resuelve
    // el nombre a "Sin categoría" cuando no la encuentra).
    let cash = finanzas_lib::repositories::accounts::id_by_code(&conn, "cash").await.unwrap();
    let entry = services::entries::create(
        &conn,
        EntryInput {
            occurred_on: "2026-01-01".into(),
            kind: "expense".into(),
            amount_cop: 10_000,
            account_from: Some(cash),
            account_to: None,
            category_id: Some(cat.id.clone()),
            goal_id: None,
            loan_id: None,
            note: None,
            is_extraordinary: false,
        },
    )
    .await
    .unwrap();
    services::categories::delete(&conn, &cat.id).await.unwrap();

    assert!(
        finanzas_lib::repositories::categories::get(&conn, &cat.id).await.is_err(),
        "con movimientos asociados, eliminar debe borrar de verdad igual, no archivar"
    );

    let orphaned = services::entries::get_by_id(&conn, &entry.id).await.unwrap();
    assert_eq!(orphaned.category_id, Some(cat.id.clone()), "el movimiento viejo conserva la referencia, aunque ya no exista la categoría");
}

/// Renombrar a un nombre ya usado por otra categoría del mismo tipo debe
/// rechazarse igual que al crear (mismo UNIQUE (name, kind) del esquema).
#[tokio::test]
async fn renombrar_categoria_a_nombre_duplicado_se_rechaza() {
    let conn = fresh_db().await;
    services::categories::create(
        &conn,
        CategoryInput { name: "Servicios".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();
    let otra = services::categories::create(
        &conn,
        CategoryInput { name: "Suscripciones".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();

    let conflict = services::categories::update(&conn, &otra.id, "Servicios", false, None).await;
    assert!(conflict.is_err(), "renombrar a un nombre ya usado (mismo tipo) debe rechazarse");
}

#[tokio::test]
async fn presupuesto_mensual_y_override() {
    let conn = fresh_db().await;
    let cat = services::categories::create(
        &conn,
        CategoryInput { name: "Transporte".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();

    services::budgets_v2::set_monthly(&conn, &cat.id, 200_000).await.unwrap();
    let base = finanzas_lib::repositories::budgets_v2::effective_monthly(&conn, &cat.id, Some("2026-05")).await.unwrap();
    assert_eq!(base, 200_000, "sin override debe usar el presupuesto base");

    services::budgets_v2::set_override(&conn, &cat.id, "2026-06", 350_000).await.unwrap();
    let overridden = finanzas_lib::repositories::budgets_v2::effective_monthly(&conn, &cat.id, Some("2026-06")).await.unwrap();
    assert_eq!(overridden, 350_000, "con override de junio debe usar ese valor");
    let other_month = finanzas_lib::repositories::budgets_v2::effective_monthly(&conn, &cat.id, Some("2026-07")).await.unwrap();
    assert_eq!(other_month, 200_000, "otros meses siguen con el base");
}

#[tokio::test]
async fn entries_crud_filtros_y_resumen() {
    let conn = fresh_db().await;
    let cash = finanzas_lib::repositories::accounts::id_by_code(&conn, "cash").await.unwrap();
    let salario = services::categories::create(&conn, CategoryInput { name: "Salario".into(), kind: "income".into(), is_fixed: true, route_id: None }).await.unwrap();
    let comida = services::categories::create(&conn, CategoryInput { name: "Comida".into(), kind: "expense".into(), is_fixed: false, route_id: None }).await.unwrap();

    let income = services::entries::create(&conn, EntryInput {
        occurred_on: "2026-06-01".into(), kind: "income".into(), amount_cop: 1_000_000,
        account_from: None, account_to: Some(cash.clone()), category_id: Some(salario.id.clone()),
        goal_id: None, loan_id: None, note: Some("pago mensual".into()), is_extraordinary: false,
    }).await.unwrap();

    services::entries::create(&conn, EntryInput {
        occurred_on: "2026-06-05".into(), kind: "expense".into(), amount_cop: 80_000,
        account_from: Some(cash.clone()), account_to: None, category_id: Some(comida.id.clone()),
        goal_id: None, loan_id: None, note: None, is_extraordinary: false,
    }).await.unwrap();

    // Filtro por categoría.
    let filtered = services::entries::list(&conn, EntryFilter {
        category_id: Some(comida.id.clone()), ..Default::default()
    }).await.unwrap();
    assert_eq!(filtered.total_count, 1);
    assert_eq!(filtered.filtered_expense, 80_000);

    // Update.
    let updated = services::entries::update(&conn, &income.id, "2026-06-02", 1_050_000, Some("actualizado"), false, Some(&salario.id)).await.unwrap();
    assert_eq!(updated.amount_cop, 1_050_000);
    assert_eq!(updated.occurred_on, "2026-06-02");

    // Reasignar categoría a otra del mismo tipo (income) debe funcionar.
    let freelance = services::categories::create(&conn, CategoryInput { name: "Freelance".into(), kind: "income".into(), is_fixed: false, route_id: None }).await.unwrap();
    let recategorized = services::entries::update(&conn, &income.id, "2026-06-02", 1_050_000, None, false, Some(&freelance.id)).await.unwrap();
    assert_eq!(recategorized.category_id, Some(freelance.id.clone()));

    // Reasignar a una categoría de gasto (tipo distinto) debe rechazarse.
    assert!(services::entries::update(&conn, &income.id, "2026-06-02", 1_050_000, None, false, Some(&comida.id)).await.is_err());

    // Resumen de período (mes de junio 2026, ya pasado por completo).
    let summary = services::entries::period_summary(&conn, &PeriodV2::Month { year: 2026, month: 6 }).await.unwrap();
    assert_eq!(summary.total_income, 1_050_000);
    assert_eq!(summary.total_expense, 80_000);
    assert_eq!(summary.balance, 970_000);
    assert!(!summary.range.partial);

    // Progreso por categoría.
    services::budgets_v2::set_monthly(&conn, &comida.id, 100_000).await.unwrap();
    let progress = services::entries::category_progress(&conn, "2026-06-01", "2026-06-30", Some("2026-06")).await.unwrap();
    let comida_progress = progress.iter().find(|p| p.category_id == comida.id).unwrap();
    assert_eq!(comida_progress.current_amount, 80_000);
    assert_eq!(comida_progress.monthly_target, 100_000);
    assert!(!comida_progress.is_over);

    // Delete + delete_bulk.
    let expense_id = filtered.entries[0].id.clone();
    services::entries::delete(&conn, &expense_id).await.unwrap();
    assert!(services::entries::get_by_id(&conn, &expense_id).await.is_err());

    let bulk_deleted = services::entries::delete_bulk(&conn, vec![updated.id.clone()]).await.unwrap();
    assert_eq!(bulk_deleted, 1);
}

#[tokio::test]
async fn goals_v2_crud_y_progreso() {
    let conn = fresh_db().await;
    let goal = services::goals_v2::create(&conn, GoalInputV2 {
        name: "Vacaciones".into(), target_cop: 1_000_000, target_date: None, kind: "saving".into(), installments: None,
    }).await.unwrap();
    assert_eq!(goal.current_amount, 0);

    let cash = finanzas_lib::repositories::accounts::id_by_code(&conn, "cash").await.unwrap();
    services::meta_payments::add_payment(&conn, &format!("goal:{}", goal.goal.id), 300_000, "2026-01-01", None).await.unwrap();
    let _ = cash;

    let refreshed = services::goals_v2::get_detail(&conn, &goal.goal.id).await.unwrap();
    assert_eq!(refreshed.goal.current_amount, 300_000);
    assert_eq!(refreshed.goal.pending, 700_000);
    assert_eq!(refreshed.contributions.len(), 1);

    let updated = services::goals_v2::update(&conn, &goal.goal.id, GoalInputV2 {
        name: "Vacaciones 2027".into(), target_cop: 1_200_000, target_date: None, kind: "saving".into(), installments: None,
    }).await.unwrap();
    assert_eq!(updated.goal.name, "Vacaciones 2027");

    services::goals_v2::delete(&conn, &goal.goal.id).await.unwrap();
    assert!(services::goals_v2::get_detail(&conn, &goal.goal.id).await.is_err());
}

#[tokio::test]
async fn loans_v2_crud_y_total_pendiente() {
    let conn = fresh_db().await;

    // Fondear la caja primero para poder prestar de ahí.
    let cash = finanzas_lib::repositories::accounts::id_by_code(&conn, "cash").await.unwrap();
    let salario = services::categories::create(&conn, CategoryInput { name: "Salario".into(), kind: "income".into(), is_fixed: false, route_id: None }).await.unwrap();
    services::entries::create(&conn, EntryInput {
        occurred_on: "2025-12-01".into(), kind: "income".into(), amount_cop: 1_000_000,
        account_from: None, account_to: Some(cash), category_id: Some(salario.id.clone()),
        goal_id: None, loan_id: None, note: None, is_extraordinary: false,
    }).await.unwrap();
    let before = services::entries::account_balances(&conn).await.unwrap();

    // Bug reportado por el usuario (2026-09-10): crear un préstamo debía
    // bajar el disponible de inmediato — antes no lo hacía porque solo se
    // guardaba la fila en `loans`, sin mover plata de verdad.
    let loan = services::loans_v2::create(&conn, LoanInputV2 {
        person_name: "Un amigo".into(), principal_cop: 200_000, lent_on: "2026-01-01".into(), note: None,
    }).await.unwrap();
    assert_eq!(loan.pending, 200_000);
    assert_eq!(loan.status, "pendiente");

    let after_lend = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(before.disponible - after_lend.disponible, 200_000, "prestar debe bajar el disponible");
    assert_eq!(after_lend.patrimonio, before.patrimonio, "prestar no cambia el patrimonio (sigue siendo tuyo, por cobrar)");

    // Debe quedar en el historial: una entry de tipo transfer con este loan_id.
    let history = services::entries::list(&conn, EntryFilter { ..Default::default() }).await.unwrap();
    assert!(history.entries.iter().any(|e| e.loan_id.as_deref() == Some(loan.loan.id.as_str()) && e.kind == "transfer"));

    services::meta_payments::add_payment(&conn, &format!("loan:{}", loan.loan.id), 200_000, "2026-02-01", None).await.unwrap();
    let refreshed = services::loans_v2::get(&conn, &loan.loan.id).await.unwrap();
    assert_eq!(refreshed.status, "pagado");
    assert_eq!(refreshed.pending, 0);

    let after_collect = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_collect.disponible, before.disponible, "cobrar el préstamo completo debe devolver el disponible al punto de partida, ni un peso de más");

    let total_pending = services::loans_v2::total_pending(&conn).await.unwrap();
    assert_eq!(total_pending, 0);

    // No se puede bajar el monto por debajo de lo ya cobrado.
    assert!(services::loans_v2::update(&conn, &loan.loan.id, "Un amigo", 100_000).await.is_err());

    // Editar el monto sincroniza el movimiento original.
    let loan2 = services::loans_v2::create(&conn, LoanInputV2 {
        person_name: "Otro amigo".into(), principal_cop: 50_000, lent_on: "2026-03-01".into(), note: None,
    }).await.unwrap();
    services::loans_v2::update(&conn, &loan2.loan.id, "Otro amigo", 80_000).await.unwrap();
    let balances_after_edit = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(after_collect.disponible - balances_after_edit.disponible, 80_000, "editar el monto debe ajustar el movimiento real");

    // Borrar el préstamo borra también su movimiento — no deja huérfanos.
    services::loans_v2::delete(&conn, &loan.loan.id).await.unwrap();
    assert!(services::loans_v2::get(&conn, &loan.loan.id).await.is_err());
    services::loans_v2::delete(&conn, &loan2.loan.id).await.unwrap();
    let balances_after_delete = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(balances_after_delete.disponible, before.disponible, "borrar los préstamos debe devolver el disponible al punto de partida");
}

#[tokio::test]
async fn crear_deuda_desde_metas_baja_patrimonio_de_inmediato() {
    let conn = fresh_db().await;
    let tecnologia = services::categories::create(
        &conn,
        CategoryInput { name: "Tecnologia".into(), kind: "expense".into(), is_fixed: false, route_id: None },
    )
    .await
    .unwrap();
    let before = services::entries::account_balances(&conn).await.unwrap();

    // Crear una deuda es una decisión de financiamiento, no algo ligado a
    // que el disponible no alcance — se centraliza en Metas (a pedido del
    // usuario, 2026-09-10) y baja el patrimonio de inmediato (es un gasto
    // real), sin tocar el disponible (todavía no sale efectivo).
    let result = services::goals_v2::create_debt(&conn, finanzas_lib::models::DebtGoalInput {
        name: "Deuda de prueba".into(), target_cop: 4_000_000, occurred_on: "2026-01-01".into(),
        category_id: tecnologia.id.clone(), installments: Some(12), note: None,
    }).await.unwrap();

    assert_eq!(result.entry.kind, "expense");
    assert_eq!(result.entry.amount_cop, 4_000_000);
    assert_eq!(result.goal.current_amount, 0);
    assert_eq!(result.goal.pending, 4_000_000);

    let after = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(after.disponible, before.disponible, "una deuda no toca el disponible al crearla");
    assert_eq!(before.patrimonio - after.patrimonio, 4_000_000, "una deuda baja el patrimonio de inmediato, es un gasto real");

    // Un abono sí baja el disponible y la deuda, pero no vuelve a contar como gasto.
    services::meta_payments::add_payment(&conn, &format!("goal:{}", result.goal.goal.id), 500_000, "2026-02-01", None).await.unwrap();
    let after_payment = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(before.disponible - after_payment.disponible, 500_000);
    assert_eq!(after_payment.patrimonio, after.patrimonio, "el abono no vuelve a mover el patrimonio");

    // Editar el monto de la deuda sincroniza el gasto original.
    let updated = services::goals_v2::update(&conn, &result.goal.goal.id, GoalInputV2 {
        name: "Deuda de prueba editada".into(), target_cop: 4_500_000, target_date: None, kind: "debt".into(), installments: Some(12),
    }).await.unwrap();
    assert_eq!(updated.pending, 4_500_000 - 500_000);
    // El abono es un transfer: nunca mueve el patrimonio (test 2/10). El
    // delta de patrimonio contra el punto de partida es siempre el monto
    // total de la deuda vigente, pagada o no.
    let balances_after_edit = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(before.patrimonio - balances_after_edit.patrimonio, 4_500_000, "el nuevo monto debe reflejarse en el patrimonio");

    // Borrar la deuda borra también sus movimientos.
    services::goals_v2::delete(&conn, &result.goal.goal.id).await.unwrap();
    let balances_after_delete = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(balances_after_delete.patrimonio, before.patrimonio, "borrar la deuda debe devolver el patrimonio al punto de partida");
    assert_eq!(balances_after_delete.disponible, before.disponible);
}

#[tokio::test]
async fn metas_v2_unifica_prestamos_y_goals() {
    let conn = fresh_db().await;
    services::loans_v2::create(&conn, LoanInputV2 { person_name: "Ana".into(), principal_cop: 50_000, lent_on: "2026-01-01".into(), note: None }).await.unwrap();
    services::goals_v2::create(&conn, GoalInputV2 { name: "Moto".into(), target_cop: 500_000, target_date: None, kind: "debt".into(), installments: None }).await.unwrap();
    services::goals_v2::create(&conn, GoalInputV2 { name: "Viaje".into(), target_cop: 300_000, target_date: None, kind: "saving".into(), installments: None }).await.unwrap();

    let metas = services::metas_v2::list(&conn).await.unwrap();
    let tipos: Vec<&str> = metas.iter().map(|m| m.tipo.as_str()).collect();
    assert!(tipos.contains(&"me_deben"));
    assert!(tipos.contains(&"debo"));
    assert!(tipos.contains(&"quiero_juntar"));
}

#[tokio::test]
async fn vehiculos_y_tanqueos_v2() {
    let conn = fresh_db().await;
    let vehicle = services::vehicles_v2::create(&conn, VehicleInputV2 {
        name: "Moto".into(), km_per_gallon: 90.0, tank_gallons: 3.0,
    }).await.unwrap();
    assert!(vehicle.tank_capacity_ml.is_some());

    let (fillup, warning) = services::fuel::create_fillup(&conn, &vehicle.id, "2026-01-01", 20_000, 12_000, None, None).await.unwrap();
    assert!(warning.is_none(), "un tanqueo normal no debe disparar aviso");
    assert!(fillup.volume_ml > 0);

    let listed = services::fuel::list_fillups(&conn, Some(&vehicle.id)).await.unwrap();
    assert_eq!(listed.len(), 1);

    let updated = services::vehicles_v2::update(&conn, &vehicle.id, VehicleInputV2 {
        name: "Moto nueva".into(), km_per_gallon: 90.0, tank_gallons: 3.0,
    }).await.unwrap();
    assert_eq!(updated.name, "Moto nueva");

    services::vehicles_v2::delete(&conn, &vehicle.id).await.unwrap();
    assert!(services::vehicles_v2::list(&conn).await.unwrap().is_empty());
}

#[tokio::test]
async fn rutas_v2_crud() {
    let conn = fresh_db().await;
    let route = services::routes_v2::save(&conn, RouteInputV2 {
        name: "Casa-Trabajo".into(), km_round_trip: 20.0, description: None,
    }).await.unwrap();
    assert_eq!(route.distance_m, 20_000);

    let listed = services::routes_v2::list(&conn).await.unwrap();
    assert_eq!(listed.len(), 1);

    services::routes_v2::delete(&conn, &route.id).await.unwrap();
    assert!(services::routes_v2::list(&conn).await.unwrap().is_empty());
}

#[tokio::test]
async fn factory_reset_v2_no_borra_cuentas_de_sistema() {
    let conn = fresh_db().await;
    let cat = services::categories::create(&conn, CategoryInput { name: "Comida".into(), kind: "expense".into(), is_fixed: false, route_id: None }).await.unwrap();
    services::vehicles_v2::create(&conn, VehicleInputV2 { name: "Carro".into(), km_per_gallon: 30.0, tank_gallons: 12.0 }).await.unwrap();
    finanzas_lib::repositories::gas::upsert(&conn, "2026-09-01", 15000).await.unwrap();

    services::system_v2::factory_reset(&conn).await.unwrap();

    assert!(services::categories::list(&conn, None, false).await.unwrap().is_empty());
    assert!(services::vehicles_v2::list(&conn).await.unwrap().is_empty());
    // El historial de precios de gasolina también es dato de usuario: no debe sobrevivir.
    assert!(services::gas::list(&conn, None).await.unwrap().is_empty());
    // Las 5 cuentas de sistema deben sobrevivir siempre.
    let balances = services::entries::account_balances(&conn).await.unwrap();
    assert_eq!(balances.cash, 0);
    let _ = cat;
}
