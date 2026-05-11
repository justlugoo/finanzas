import { invoke } from "@tauri-apps/api/core";
import type { Budget, Goal, RoutesCost, GasPrice } from "./types";

interface InitialData {
  budgets:             Budget[];
  categories_ingreso:  string[];
  categories_gasto:    string[];
  route_costs:         RoutesCost;
  active_goals:        Goal[];
  current_gas_price:   GasPrice | null;
}

export const cache = $state({
  budgets:         [] as Budget[],
  catsIngreso:     [] as string[],
  catsGasto:       [] as string[],
  routeCosts:      null as RoutesCost | null,
  activeGoals:     [] as Goal[],
  currentGasPrice: null as GasPrice | null,
  ready:           false,
  loading:         false,
});

export async function loadCache() {
  if (cache.ready || cache.loading) return;
  cache.loading = true;

  for (let i = 0; i < 20; i++) {
    try {
      const d = await invoke<InitialData>("get_initial_data");
      cache.budgets         = d.budgets;
      cache.catsIngreso     = d.categories_ingreso;
      cache.catsGasto       = d.categories_gasto;
      cache.routeCosts      = d.route_costs;
      cache.activeGoals     = d.active_goals;
      cache.currentGasPrice = d.current_gas_price;
      cache.ready           = true;
      cache.loading         = false;
      return;
    } catch (e: any) {
      if (e?.kind === "DatabaseError" && e?.message?.includes("no inicializada")) {
        await new Promise(r => setTimeout(r, 300));
      } else {
        console.error("[cache] load error:", e);
        cache.loading = false;
        return;
      }
    }
  }
  cache.loading = false;
}
