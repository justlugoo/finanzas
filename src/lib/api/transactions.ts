import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryProgress, CsvExport, CurrentBalance, ImportResult,
  MonthComparison, Period, PeriodSummary, Transaction, TransactionFilter,
  TransactionInput, TransactionPage,
} from "$lib/types";

export const list = (filter: TransactionFilter) =>
  invoke<TransactionPage>("list_transactions", { filter });

export const create = (input: TransactionInput) =>
  invoke<Transaction>("create_transaction", { input });

export const update = (id: number, input: TransactionInput) =>
  invoke<Transaction>("update_transaction", { id, input });

export const remove = (id: number) =>
  invoke<void>("delete_transaction", { id });

export const removeBulk = (ids: number[]) =>
  invoke<number>("delete_transactions_bulk", { ids });

export const getBalance = () =>
  invoke<CurrentBalance>("get_current_balance");

export const getPeriodSummary = (period: Period) =>
  invoke<PeriodSummary>("get_period_summary", { period });

export const getCategoryProgress = (period: Period) =>
  invoke<CategoryProgress[]>("get_category_progress", { period });

export const getMonthComparison = () =>
  invoke<MonthComparison>("get_month_comparison");

export const listCategories = (kind?: string) =>
  invoke<string[]>("list_categories", { kind });

export const exportCsv = (filter: TransactionFilter) =>
  invoke<CsvExport>("export_transactions_csv", { filter });

export const importCsv = (csvContent: string) =>
  invoke<ImportResult>("import_transactions_csv", { csvContent });
