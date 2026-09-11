//! Corre la migración 001 sobre una copia de una base real, para ver el
//! reporte antes de decidir nada. Nunca se apunta a la base real directa —
//! eso lo decide quien invoca este binario pasando la ruta.
//!
//! Uso: cargo run --example dry_run_migration -- <ruta-a-una-copia.db> <ruta-salida-reporte.csv>

use finanzas_lib::{db, migrations};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let db_path = args.next().expect("uso: dry_run_migration <copia.db> <reporte.csv>");
    let report_path = args.next().expect("uso: dry_run_migration <copia.db> <reporte.csv>");

    let database = libsql::Builder::new_local(&db_path).build().await.expect("abrir copia");
    let conn = database.connect().expect("conectar");

    // Normaliza el esquema v1 igual que hace la app real al arrancar.
    db::apply_schema(&conn).await.expect("apply_schema v1");

    let report = migrations::migrate_001_schema_v2(&conn).await.expect("migrate_001_schema_v2");

    std::fs::write(&report_path, migrations::report_to_csv(&report)).expect("escribir reporte");

    let needs_review = report.rows.iter().filter(|r| r.needs_review).count();
    println!("filas trasladadas: {}", report.rows.len());
    println!("filas marcadas para revisión: {needs_review}");
    println!("excepciones: {}", report.exceptions.len());
    for e in &report.exceptions {
        println!("  ! {e}");
    }
    println!("reporte completo en: {report_path}");

    let balances = finanzas_lib::repositories::accounts::balances(&conn).await.expect("balances");
    println!("--- saldos resultantes en la copia (para comparar contra el valor real, sin aplicar nada) ---");
    println!("cash:       {}", balances.cash);
    println!("apps:       {}", balances.apps);
    println!("savings:    {}", balances.savings);
    println!("receivable: {}", balances.receivable);
    println!("payable (deuda mostrada): {}", balances.payable);
    println!("disponible: {}", balances.disponible);
    println!("patrimonio: {}", balances.patrimonio);
}
