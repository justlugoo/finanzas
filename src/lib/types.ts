// Espejo de los tipos Rust definidos en commands_contract.md

export interface Budget {
  category: string;
  monthly_amount: number;
}

export interface Transaction {
  id: number;
  date: string;
  type: string;
  category: string;
  amount: number;
  note: string | null;
  is_extraordinary: boolean;
  goal_id: number | null;
  created_at: string;
}

export interface TransactionInput {
  date: string;
  type: string;
  category: string;
  amount: number;
  note: string | null;
  is_extraordinary: boolean;
  goal_id: number | null;
}

export type Period =
  | { type: "Daily" }
  | { type: "Weekly" }
  | { type: "Monthly" }
  | { type: "Yearly" }
  | { type: "Custom"; value: { start: string; end: string } };

export interface PeriodSummary {
  total_income: number;
  total_expenses: number;
  balance: number;
  extraordinary_income: number;
  extraordinary_expenses: number;
  transactions_count: number;
}

export interface CategoryProgress {
  category: string;
  monthly_target: number;
  current_amount: number;
  percentage: number;
  is_over: boolean;
  kind: string;
}

export interface MonthComparison {
  current_month_total: number;
  previous_month_total: number;
  delta_amount: number;
  delta_percentage: number;
  by_category: CategoryComparison[];
}

export interface CategoryComparison {
  category: string;
  current: number;
  previous: number;
  delta_pct: number;
}

export interface Goal {
  id: number;
  name: string;
  target_amount: number;
  target_date: string | null;
  status: string;
  created_at: string;
}

export interface GoalWithProgress {
  goal: Goal;
  current_amount: number;
  percentage: number;
  monthly_required: number | null;
  projected_completion_date: string | null;
  on_track: boolean;
}

export interface GasPrice {
  id: number;
  date: string;
  price_per_gallon: number;
  source: string;
}

export interface SyncStatus {
  last_sync: string | null;
  pending_writes: number;
  is_online: boolean;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
}

export interface CsvExport {
  content: string;
  suggested_filename: string;
}

export interface AppError {
  kind: string;
  message?: string;
}
