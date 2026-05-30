export interface Budget {
  category: string;
  monthly_amount: number;
  route_id: number | null;
  type: "ingreso" | "gasto";
  is_fixed: boolean;
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
  vehicle_id?: number | null;
  installments?: number | null;
}

export interface CurrentBalance {
  total_income: number;
  total_expenses: number;
  balance: number;
  cash_on_hand: number;
  net_worth: number;
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
  is_fixed: boolean;
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
  installments: number | null;
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

export interface GoalInput {
  name: string;
  target_amount: number;
  target_date: string | null;
  status?: string;
}

export interface WeeklyGasPoint {
  week_start: string;
  avg_price: number;
  entry_count: number;
}

export interface RoutesCost {
  precio_galon: number;
}

export interface Vehicle {
  id: number;
  name: string;
  km_per_gallon: number;
}

export interface VehicleInput {
  name: string;
  km_per_gallon: number;
}

export interface GasPrice {
  id: number;
  date: string;
  price_per_gallon: number;
  source: string;
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
  filtered_income: number;
  filtered_expenses: number;
}

export interface TransactionFilter {
  period?: Period | null;
  kind?: string | null;
  category?: string | null;
  search_note?: string | null;
  only_extraordinary?: boolean | null;
  only_debt?: boolean | null;
  page?: number | null;
  page_size?: number | null;
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

export interface CustomRouteInput {
  name: string;
  km_round_trip: number;
  description: string | null;
}

export interface Loan {
  id: number;
  person_name: string;
  amount: number;
  date: string;
  note: string | null;
  status: "pendiente" | "pagado";
  created_at: string;
}

export interface LoanPayment {
  id: number;
  loan_id: number;
  amount: number;
  date: string;
  created_at: string;
}

export interface LoanWithBalance {
  loan: Loan;
  paid: number;
  pending: number;
  payments: LoanPayment[];
}

export interface LoanInput {
  person_name: string;
  amount: number;
  date: string;
  note: string | null;
}

export interface LoanPaymentInput {
  loan_id: number;
  amount: number;
  date: string;
}

export interface LoanUpdateInput {
  person_name: string;
  amount: number;
}

export interface MetaAbono {
  id: number;
  date: string;
  amount: number;
}

export interface Meta {
  id: string;
  tipo: string;
  nombre: string;
  total: number;
  abonado: number;
  pendiente: number;
  estado: string;
  fecha: string | null;
  nota: string | null;
  cuotas: number | null;
  abonos: MetaAbono[];
  on_track: boolean | null;
  monthly_required: number | null;
  projected_completion_date: string | null;
}
