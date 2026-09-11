import { invoke } from "@tauri-apps/api/core";
import type { Category, CategoryInput } from "$lib/types";

export const list = (kind?: "income" | "expense") =>
  invoke<Category[]>("category_list", { kind });

export const create = (input: CategoryInput) =>
  invoke<Category>("category_create", { input });

export const update = (id: string, isFixed: boolean, routeId: string | null) =>
  invoke<Category>("category_update", { id, isFixed, routeId });

export const remove = (id: string) =>
  invoke<void>("category_delete", { id });
