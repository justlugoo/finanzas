import { invoke } from "@tauri-apps/api/core";
import type {
  AccountBalances, CategoryProgressV2, CsvExport, Entry, EntryFilter, EntryInput, EntryPage,
  MonthComparisonV2, PeriodSummaryV2, PeriodV2,
} from "$lib/types";

export const list = (filter: EntryFilter) =>
  invoke<EntryPage>("entry_list", { filter });

export const create = (input: EntryInput) =>
  invoke<Entry>("entry_create", { input });

export const get = (id: string) =>
  invoke<Entry>("entry_get", { id });

export const update = (id: string, occurredOn: string, amountCop: number, note: string | null, isExtraordinary: boolean) =>
  invoke<Entry>("entry_update", { id, occurredOn, amountCop, note, isExtraordinary });

export const remove = (id: string) =>
  invoke<void>("entry_delete", { id });

export const removeBulk = (ids: string[]) =>
  invoke<number>("entry_delete_bulk", { ids });

export const getAccountBalances = () =>
  invoke<AccountBalances>("get_account_balances");

export const getPeriodSummary = (period: PeriodV2) =>
  invoke<PeriodSummaryV2>("get_period_summary_v2", { period });

export const getCategoryProgress = (period: PeriodV2) =>
  invoke<CategoryProgressV2[]>("get_category_progress_v2", { period });

export const getMonthComparison = () =>
  invoke<MonthComparisonV2>("get_month_comparison_v2");

export const exportCsv = (filter: EntryFilter) =>
  invoke<CsvExport>("entry_export_csv", { filter });
