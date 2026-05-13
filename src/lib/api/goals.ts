import { invoke } from "@tauri-apps/api/core";
import type { GoalDetail, GoalInput, GoalWithProgress } from "$lib/types";

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
