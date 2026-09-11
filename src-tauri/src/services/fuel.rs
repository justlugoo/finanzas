use crate::error::{AppError, AppResult};
use crate::models::{Entry, EntryInput, FillupV2, FuelAdjustment, TankLevel, Trip};
use crate::repositories;
use crate::utils::gallons_cost_to_ml;
use chrono::Local;
use libsql::Connection;

fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Aviso cuando un tanqueo deja el nivel calculado por encima de la
/// capacidad del tanque — la causa más probable, y la que motivó pedir esto,
/// es un rendimiento (km/gal) mal configurado: si el vehículo consume más
/// combustible por km del que la app le resta en cada viaje, el nivel
/// calculado se acumula de más con cada tanqueo hasta desbordar la
/// capacidad real.
fn overflow_warning(raw_ml: i64, cap_ml: i64) -> String {
    format!(
        "El nivel calculado ({:.1} gal) supera la capacidad del tanque ({:.1} gal). \
         Es probable que el rendimiento (km/gal) de este vehículo esté mal configurado \
         — revísalo en Configuración. También puede ser el precio del galón o un ancla \
         de nivel vieja; puedes resetear el nivel del tanque sin perder tus tanqueos.",
        raw_ml as f64 / crate::utils::ML_PER_GALLON,
        cap_ml as f64 / crate::utils::ML_PER_GALLON,
    )
}

/// Reemplaza `services/fillups.rs::vehicle_fuel_status` de v1. El clamp se
/// aplica una sola vez, al nivel — autonomía y porcentaje se derivan de ahí
/// (v1 solo topaba el porcentaje, por eso el widget mostraba 100% con una
/// autonomía que decía otra cosa).
pub async fn tank_level(conn: &Connection, vehicle_id: &str) -> AppResult<TankLevel> {
    let vehicle = repositories::vehicles_v2::get(conn, vehicle_id).await?;
    let raw = repositories::fuel::raw_level_ml(conn, vehicle_id, &today()).await?;

    let level_ml = match vehicle.tank_capacity_ml {
        Some(cap) => raw.clamp(0, cap),
        None => raw.max(0),
    };
    let autonomy_m = (level_ml as f64 * vehicle.efficiency_m_per_l as f64 / 1000.0).round() as i64;
    let tank_percentage = vehicle.tank_capacity_ml.map(|cap| level_ml as f64 / cap as f64 * 100.0);

    Ok(TankLevel { level_ml, raw_level_ml: raw, autonomy_m, tank_percentage })
}

/// Registra un tanqueo. El precio es el que trae el formulario (editable,
/// prellenado con el último conocido) — `gas_prices` ya no es una
/// dependencia dura. Si el tanqueo deja el nivel calculado por encima de la
/// capacidad del tanque, se guarda igual y se devuelve un aviso (nunca un
/// error duro).
pub async fn create_fillup(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    total_cop: i64,
    price_cop_per_gallon: i64,
    entry_id: Option<&str>,
    note: Option<&str>,
) -> AppResult<(FillupV2, Option<String>)> {
    let volume_ml = gallons_cost_to_ml(total_cop, price_cop_per_gallon);
    let fillup = repositories::fuel::insert_fillup(
        conn, vehicle_id, occurred_on, volume_ml, price_cop_per_gallon, total_cop, entry_id, note,
    )
    .await?;

    let vehicle = repositories::vehicles_v2::get(conn, vehicle_id).await?;
    let raw = repositories::fuel::raw_level_ml(conn, vehicle_id, &today()).await?;
    let warning = match vehicle.tank_capacity_ml {
        Some(cap) if raw > cap => Some(overflow_warning(raw, cap)),
        _ => None,
    };

    Ok((fillup, warning))
}

/// Igual que `create_fillup`, pero además crea el `expense` real (`cash ->`)
/// asociado y lo enlaza vía `fillups.entry_id` — sección 10 de
/// schema-v2.md: "Fila de fuel_fillups con su transaction_id -> expense
/// cash-> + fillups con volume_ml recalculado". Todo o nada: si el fillup
/// falla, el gasto tampoco queda.
#[allow(clippy::too_many_arguments)]
pub async fn create_fillup_with_expense(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    total_cop: i64,
    price_cop_per_gallon: i64,
    category_id: &str,
    note: Option<&str>,
) -> AppResult<(Entry, FillupV2, Option<String>)> {
    conn.execute_batch("BEGIN;")
        .await
        .map_err(|e| AppError::DatabaseError(format!("create_fillup_with_expense (begin): {e}")))?;

    let result: AppResult<(Entry, FillupV2, Option<String>)> = async {
        let cash = repositories::accounts::id_by_code(conn, "cash").await?;
        let entry = crate::services::entries::create(
            conn,
            EntryInput {
                occurred_on: occurred_on.to_string(),
                kind: "expense".to_string(),
                amount_cop: total_cop,
                account_from: Some(cash),
                account_to: None,
                category_id: Some(category_id.to_string()),
                goal_id: None,
                loan_id: None,
                note: note.map(str::to_string),
                is_extraordinary: false,
            },
        )
        .await?;

        let volume_ml = gallons_cost_to_ml(total_cop, price_cop_per_gallon);
        let fillup = repositories::fuel::insert_fillup(
            conn, vehicle_id, occurred_on, volume_ml, price_cop_per_gallon, total_cop, Some(&entry.id), note,
        )
        .await?;

        let vehicle = repositories::vehicles_v2::get(conn, vehicle_id).await?;
        let raw = repositories::fuel::raw_level_ml(conn, vehicle_id, &today()).await?;
        let warning = match vehicle.tank_capacity_ml {
            Some(cap) if raw > cap => Some(overflow_warning(raw, cap)),
            _ => None,
        };

        Ok((entry, fillup, warning))
    }
    .await;

    match result {
        Ok(ok) => {
            conn.execute_batch("COMMIT;")
                .await
                .map_err(|e| AppError::DatabaseError(format!("create_fillup_with_expense (commit): {e}")))?;
            Ok(ok)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;").await;
            Err(e)
        }
    }
}

/// Registra un viaje congelando el consumo con el rendimiento **actual**
/// del vehículo — si luego se corrige `efficiency_m_per_l`, este viaje ya
/// registrado no se recalcula (principio 2 de schema-v2.md; test 8).
pub async fn list_fillups(conn: &Connection, vehicle_id: Option<&str>) -> AppResult<Vec<FillupV2>> {
    repositories::fuel::list_fillups(conn, vehicle_id).await
}

pub async fn register_trip(
    conn: &Connection,
    vehicle_id: &str,
    occurred_on: &str,
    distance_m: i64,
    entry_id: Option<&str>,
) -> AppResult<Trip> {
    let vehicle = repositories::vehicles_v2::get(conn, vehicle_id).await?;
    let consumed_ml = ((distance_m as f64 * 1000.0 / vehicle.efficiency_m_per_l as f64).round() as i64).max(1);
    repositories::fuel::insert_trip(conn, vehicle_id, occurred_on, distance_m, consumed_ml, entry_id).await
}

/// Resetea el nivel del tanque a `level_ml` (típicamente 0, o lo que el
/// medidor real marque) sin borrar ni un solo tanqueo o viaje — sección 7 de
/// schema-v2.md: el histórico anterior al ancla queda ahí para auditoría,
/// simplemente deja de contar para el nivel actual. Pensado para que
/// cualquier usuario lo dispare desde la app cuando quiera (después de
/// llenar el tanque a fondo, después de mucho tiempo sin registrar, etc.),
/// no solo durante una migración.
pub async fn reset_level(
    conn: &Connection,
    vehicle_id: &str,
    level_ml: i64,
    occurred_on: &str,
    note: Option<&str>,
) -> AppResult<FuelAdjustment> {
    repositories::fuel::insert_adjustment(conn, vehicle_id, occurred_on, level_ml, note).await
}
