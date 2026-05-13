import { invoke } from "@tauri-apps/api/core";
import type { CustomRoute, CustomRouteInput } from "$lib/types";

export const list = () =>
  invoke<CustomRoute[]>("get_custom_routes");

export const save = (route: CustomRouteInput) =>
  invoke<CustomRoute>("save_custom_route", { route });

export const remove = (id: number) =>
  invoke<void>("delete_custom_route", { id });
