import { invoke } from "@tauri-apps/api/core";
import type { RouteInputV2, RouteV2 } from "$lib/types";

export const list = () =>
  invoke<RouteV2[]>("route_list_v2");

export const save = (input: RouteInputV2) =>
  invoke<RouteV2>("route_save_v2", { input });

export const remove = (id: string) =>
  invoke<void>("route_delete_v2", { id });
