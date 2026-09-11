import { invoke } from "@tauri-apps/api/core";
import type { DebtGoalInput, DebtGoalResult, GoalDetailV2, GoalInputV2, GoalWithProgressV2 } from "$lib/types";

export const list = (kind?: "saving" | "debt") =>
  invoke<GoalWithProgressV2[]>("goal_list_v2", { kind });

export const create = (input: GoalInputV2) =>
  invoke<GoalWithProgressV2>("goal_create_v2", { input });

export const createDebt = (input: DebtGoalInput) =>
  invoke<DebtGoalResult>("goal_create_debt_v2", { input });

export const update = (id: string, input: GoalInputV2) =>
  invoke<GoalWithProgressV2>("goal_update_v2", { id, input });

export const remove = (id: string) =>
  invoke<void>("goal_delete_v2", { id });

export const getDetail = (id: string) =>
  invoke<GoalDetailV2>("goal_get_detail_v2", { id });
