import { invoke } from "@tauri-apps/api/core";
import type { Category, CategoryInput } from "$lib/types";

// `includeArchived` — una categoría "eliminada" que tenía movimientos
// asociados queda archivada, no borrada (ver services::categories::delete
// en el backend). Por defecto se excluye (selección activa: chips de
// Registrar, lista de Presupuestos); pásalo en `true` donde haga falta
// resolver el nombre real de movimientos viejos (Historial, "Último
// registro"), para que no aparezcan como "Sin categoría".
export const list = (kind?: "income" | "expense", includeArchived?: boolean) =>
  invoke<Category[]>("category_list", { kind, includeArchived });

export const create = (input: CategoryInput) =>
  invoke<Category>("category_create", { input });

export const update = (id: string, name: string, isFixed: boolean, routeId: string | null) =>
  invoke<Category>("category_update", { id, name, isFixed, routeId });

export const remove = (id: string) =>
  invoke<void>("category_delete", { id });
