import { invoke } from "@tauri-apps/api/core";
import type { GoalDetail, GoalInput, GoalWithProgress } from "$lib/types";
import { create as createTransaction } from "./transactions";

export const list = (status?: string) =>
  invoke<GoalWithProgress[]>("list_goals", { status });

export const create = (input: GoalInput) =>
  invoke<GoalWithProgress>("create_goal", { input });

export const update = (id: number, input: GoalInput) =>
  invoke<GoalWithProgress>("update_goal", { id, input });

export const remove = (id: number) =>
  invoke<void>("delete_goal", { id });

export const getDetail = (id: number) =>
  invoke<GoalDetail>("get_goal_detail", { id });

export const addContribution = (goalId: number, amount: number, date: string) =>
  createTransaction({
    date,
    type: "ingreso",
    category: "Abono",
    amount,
    note: null,
    is_extraordinary: false,
    goal_id: goalId,
    gas_km: null,
    is_debt: false,
    vehicle_id: null,
  });
