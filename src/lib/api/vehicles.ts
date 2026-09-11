import { invoke } from "@tauri-apps/api/core";
import type { VehicleInputV2, VehicleV2 } from "$lib/types";

export const list = () =>
  invoke<VehicleV2[]>("vehicle_list_v2");

export const create = (input: VehicleInputV2) =>
  invoke<VehicleV2>("vehicle_create_v2", { input });

export const update = (id: string, input: VehicleInputV2) =>
  invoke<VehicleV2>("vehicle_update_v2", { id, input });

export const remove = (id: string) =>
  invoke<void>("vehicle_delete_v2", { id });
