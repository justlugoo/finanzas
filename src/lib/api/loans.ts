import { invoke } from "@tauri-apps/api/core";
import type { LoanInputV2, LoanWithBalanceV2 } from "$lib/types";

export const create = (input: LoanInputV2) =>
  invoke<LoanWithBalanceV2>("loan_create_v2", { input });

export const list = () =>
  invoke<LoanWithBalanceV2[]>("loan_list_v2");

export const get = (id: string) =>
  invoke<LoanWithBalanceV2>("loan_get_v2", { id });

export const update = (id: string, personName: string, principalCop: number) =>
  invoke<LoanWithBalanceV2>("loan_update_v2", { id, personName, principalCop });

export const remove = (id: string) =>
  invoke<void>("loan_delete_v2", { id });

export const totalPending = () =>
  invoke<number>("loans_total_pending_v2");
