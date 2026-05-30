import { invoke } from "@tauri-apps/api/core";
import type { FuelFillup, FuelFillupInput, VehicleFuelStatus } from "$lib/types";

export const create = (input: FuelFillupInput) =>
  invoke<FuelFillup>("fillup_create", { input });

export const list = (vehicleId?: number | null) =>
  invoke<FuelFillup[]>("fillups_list", { vehicleId: vehicleId ?? null });

export const vehicleFuelStatus = (vehicleId: number) =>
  invoke<VehicleFuelStatus>("vehicle_fuel_status", { vehicleId });
