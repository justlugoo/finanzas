// Espejo de los tipos Rust definidos en commands_contract.md

export interface Budget {
  category: string;
  monthly_amount: number;
  route_id: number | null;
  type: "ingreso" | "gasto";
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
  is_debt: boolean;
}

export interface TransactionInput {
  date: string;
  type: string;
  category: string;
  amount: number;
  note: string | null;
  is_extraordinary: boolean;
  goal_id: number | null;
  gas_km: number | null;
  is_debt?: boolean;
}

export interface CurrentBalance {
  total_income: number;
  total_expenses: number;
  balance: number;
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
  sub_breakdown?: { label: string; amount: number }[];
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
  is_debt_goal: boolean;
}

export interface GoalWithProgress {
  goal: Goal;
  current_amount: number;
  percentage: number;
  monthly_required: number | null;
  projected_completion_date: string | null;
  on_track: boolean;
}

export interface GoalDetail {
  goal: GoalWithProgress;
  contributions: Transaction[];
}

export interface WeeklyGasPoint {
  week_start: string;
  avg_price: number;
  entry_count: number;
}

export interface RoutesCost {
  precio_galon: number;
  consumo_km_galon: number;
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

export interface TransactionPage {
  transactions: Transaction[];
  total_count: number;
}

export interface AppError {
  kind: string;
  message?: string;
}

export interface CustomRoute {
  id: number;
  name: string;
  km_round_trip: number;
  description: string | null;
}
