import { invoke } from "@tauri-apps/api/core";
import type { LoanWithBalance, LoanInput, LoanPaymentInput, LoanUpdateInput } from "$lib/types";

export const create = (input: LoanInput) =>
  invoke<LoanWithBalance>("loan_create", { input });

export const list = () =>
  invoke<LoanWithBalance[]>("loan_list");

export const get = (id: number) =>
  invoke<LoanWithBalance>("loan_get", { id });

export const update = (id: number, input: LoanUpdateInput) =>
  invoke<LoanWithBalance>("loan_update", { id, input });

export const addPayment = (input: LoanPaymentInput) =>
  invoke<LoanWithBalance>("loan_add_payment", { input });

export const remove = (id: number) =>
  invoke<void>("loan_delete", { id });

export const totalPending = () =>
  invoke<number>("loans_total_pending");
