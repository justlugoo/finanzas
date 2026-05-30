use crate::error::{AppError, AppResult};
use crate::models::{FuelFillup, FuelFillupInput, TransactionInput, VehicleFuelStatus};
use crate::repositories::{fillups as fillup_repo, gas as gas_repo, transactions as tx_repo, vehicles as vehicles_repo};
use crate::utils::is_valid_date;

// Factor de conversión físico: 1 galón = 3.785 litros (NIST / BIPM).
const LITERS_PER_GALLON: f64 = 3.785;

pub async fn create(conn: &libsql::Connection, input: FuelFillupInput) -> AppResult<FuelFillup> {
    if input.amount_cop <= 0 {
        return Err(AppError::ValidationError("el monto debe ser mayor que 0".into()));
    }
    let category = input.category.trim().to_string();
    if category.is_empty() {
        return Err(AppError::ValidationError("la categoría es obligatoria".into()));
    }
    if !is_valid_date(&input.date) {
        return Err(AppError::ValidationError(
            "fecha inválida — usa formato YYYY-MM-DD".into(),
        ));
    }

    // Confirma que el vehículo existe (no hay FK en DB — verificación explícita).
    gas_repo::get_vehicle_km_per_gallon(conn, input.vehicle_id).await?
        .ok_or_else(|| AppError::NotFound(
            format!("vehículo {} no existe", input.vehicle_id)
        ))?;

    let price_per_gallon = gas_repo::find_price_for_date(conn, &input.date).await?
        .ok_or_else(|| AppError::ValidationError(
            "no hay precio de gasolina registrado para esa fecha; \
             registra primero el precio en Configuración → Gasolina".into(),
        ))?;

    let gallons = input.amount_cop as f64 / price_per_gallon as f64;

    let tx_note = match &input.note {
        Some(n) if !n.trim().is_empty() => format!("Tanqueo: {}", n.trim()),
        _ => "Tanqueo".to_string(),
    };

    conn.execute("BEGIN", ()).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Inserta el gasto en transactions directamente (sin pasar por services/transactions
    // para no activar lógica de deuda ni cuotas — un tanqueo es un gasto simple).
    let tx_input = TransactionInput {
        date:            input.date.clone(),
        kind:            "gasto".to_string(),
        category:        category.clone(),
        amount:          input.amount_cop,
        note:            Some(tx_note),
        is_extraordinary: false,
        goal_id:         None,
        gas_km:          None,
        is_debt:         false,
        vehicle_id:      None,
        installments:    None,
    };
    let tx = match tx_repo::insert(conn, &tx_input).await {
        Ok(t) => t,
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(e);
        }
    };

    let fillup = match fillup_repo::insert(
        conn,
        &input.date,
        input.vehicle_id,
        gallons,
        price_per_gallon,
        input.amount_cop,
        input.note.as_deref(),
        Some(tx.id),
    ).await {
        Ok(f) => f,
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(e);
        }
    };

    if let Err(e) = conn.execute("COMMIT", ()).await {
        let _ = conn.execute("ROLLBACK", ()).await;
        return Err(AppError::DatabaseError(e.to_string()));
    }

    Ok(fillup)
}

pub async fn list(
    conn: &libsql::Connection,
    vehicle_id: Option<i64>,
) -> AppResult<Vec<FuelFillup>> {
    fillup_repo::list(conn, vehicle_id).await
}

pub async fn vehicle_fuel_status(
    conn: &libsql::Connection,
    vehicle_id: i64,
) -> AppResult<VehicleFuelStatus> {
    let vehicle = vehicles_repo::get_by_id(conn, vehicle_id).await?
        .ok_or_else(|| AppError::NotFound(format!("vehículo {vehicle_id} no existe")))?;

    let total_gallons_in  = fillup_repo::sum_gallons_by_vehicle(conn, vehicle_id).await?;
    let total_trip_km     = fillup_repo::sum_trip_km_by_vehicle(conn, vehicle_id).await?;

    // Galones consumidos = km recorridos / rendimiento del vehículo.
    // km_per_gallon > 0 está garantizado por validación al crear/editar el vehículo.
    let consumed_gallons = total_trip_km / vehicle.km_per_gallon;

    // El nivel puede ser negativo si el usuario tiene viajes registrados sin
    // haber cargado tanqueos previos. Se reporta tal cual: es información útil
    // que indica datos históricos faltantes. El layer de UI decide cómo mostrarlo.
    let level_gallons = total_gallons_in - consumed_gallons;

    // Autonomía nunca es negativa — cero km es el piso físico.
    let autonomy_km = level_gallons.max(0.0) * vehicle.km_per_gallon;

    let tank_percentage = vehicle.tank_liters.map(|liters| {
        let tank_gallons = liters / LITERS_PER_GALLON;
        if tank_gallons > 0.0 {
            // Clamped a [0, 100]: el tanque no puede estar al 110%.
            (level_gallons / tank_gallons * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    });

    Ok(VehicleFuelStatus {
        vehicle_id,
        vehicle_name: vehicle.name,
        km_per_gallon: vehicle.km_per_gallon,
        tank_liters: vehicle.tank_liters,
        level_gallons,
        autonomy_km,
        tank_percentage,
    })
}
