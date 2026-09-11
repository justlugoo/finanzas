import { invoke } from "@tauri-apps/api/core";
import type {
  FillupInputV2, FillupResult, FillupV2, FillupWithExpenseInput, FillupWithExpenseResult,
  FuelAdjustment, FuelLevelResetInput, TankLevel, Trip, TripInputV2,
} from "$lib/types";

export const create = (input: FillupInputV2) =>
  invoke<FillupResult>("fillup_create_v2", { input });

// Crea el gasto real (cash->) y el tanqueo juntos, enlazados — lo que usa
// el flujo normal de "Registrar" (a diferencia de `create`, que asume que
// el gasto ya existe o se maneja aparte).
export const createWithExpense = (input: FillupWithExpenseInput) =>
  invoke<FillupWithExpenseResult>("fillup_create_with_expense_v2", { input });

export const list = (vehicleId?: string | null) =>
  invoke<FillupV2[]>("fillups_list_v2", { vehicleId: vehicleId ?? null });

export const vehicleFuelStatus = (vehicleId: string) =>
  invoke<TankLevel>("vehicle_fuel_status_v2", { vehicleId });

export const registerTrip = (input: TripInputV2) =>
  invoke<Trip>("trip_register_v2", { input });

// El usuario resetea el nivel del tanque cuando quiera, sin borrar tanqueos
// ni viajes — sección 7 de schema-v2.md.
export const resetFuelLevel = (input: FuelLevelResetInput) =>
  invoke<FuelAdjustment>("vehicle_reset_fuel_level", { input });
