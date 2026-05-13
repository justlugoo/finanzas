import { invoke } from "@tauri-apps/api/core";
import type { Vehicle, VehicleInput } from "$lib/types";

export const list = () =>
  invoke<Vehicle[]>("list_vehicles");

export const create = (input: VehicleInput) =>
  invoke<Vehicle>("create_vehicle", { input });

export const update = (id: number, input: VehicleInput) =>
  invoke<Vehicle>("update_vehicle", { id, input });

export const remove = (id: number) =>
  invoke<void>("delete_vehicle", { id });
