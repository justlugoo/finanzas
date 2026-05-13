import { invoke } from "@tauri-apps/api/core";
import type { GasPrice, RoutesCost, WeeklyGasPoint } from "$lib/types";

export const getCurrent = () =>
  invoke<GasPrice | null>("get_current_gas_price");

export const list = (limit?: number) =>
  invoke<GasPrice[]>("list_gas_prices", { limit });

export const registerManual = (price: number) =>
  invoke<GasPrice>("register_gas_price_manual", { price });

export const getWeeklyComparison = () =>
  invoke<WeeklyGasPoint[]>("get_weekly_gas_comparison");

export const getRouteCosts = () =>
  invoke<RoutesCost>("get_route_costs");
