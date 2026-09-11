import { invoke } from "@tauri-apps/api/core";
import type { BudgetV2, CategoryBudgetRow } from "$lib/types";

export const listWithCategories = () =>
  invoke<CategoryBudgetRow[]>("budget_list_with_categories");

export const setMonthly = (categoryId: string, monthlyCop: number) =>
  invoke<BudgetV2>("budget_set_monthly", { categoryId, monthlyCop });

export const setOverride = (categoryId: string, yearMonth: string, amountCop: number) =>
  invoke<void>("budget_set_override", { categoryId, yearMonth, amountCop });
