import { invoke } from "@tauri-apps/api/core";
import type { Budget } from "$lib/types";

export const list = () =>
  invoke<Budget[]>("list_budgets");

export const create = (category: string, monthlyAmount: number, kind: string, isFixed?: boolean) =>
  invoke<Budget>("create_budget", { category, monthlyAmount, kind, isFixed });

export const updateAmount = (category: string, monthlyAmount: number) =>
  invoke<Budget>("update_budget", { category, monthlyAmount });

export const updateRoute = (category: string, routeId: number | null) =>
  invoke<void>("update_budget_route", { category, routeId });

export const updateFixed = (category: string, isFixed: boolean) =>
  invoke<Budget>("update_budget_fixed", { category, isFixed });

export const remove = (category: string) =>
  invoke<void>("delete_budget", { category });
