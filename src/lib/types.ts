// Tipos v2 (docs/schema-v2.md) — espejo de los structs Rust en src-tauri/src/models/mod.rs.

export interface Account {
  id: string;
  code: string;
  name: string;
  kind: "asset" | "liability";
  is_system: boolean;
}

export interface AccountBalances {
  cash: number;
  apps: number;
  savings: number;
  receivable: number;
  payable: number;
  disponible: number;
  patrimonio: number;
}

export interface Category {
  id: string;
  name: string;
  kind: "income" | "expense";
  is_fixed: boolean;
  route_id: string | null;
  is_system: boolean;
  code: string | null;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CategoryInput {
  name: string;
  kind: "income" | "expense";
  is_fixed: boolean;
  route_id: string | null;
}

export interface Entry {
  id: string;
  occurred_on: string;
  type: "income" | "expense" | "transfer";
  amount_cop: number;
  account_from: string | null;
  account_to: string | null;
  category_id: string | null;
  goal_id: string | null;
  loan_id: string | null;
  note: string | null;
  is_extraordinary: boolean;
  created_at: string;
  updated_at: string;
}

export interface EntryInput {
  occurred_on: string;
  type: "income" | "expense" | "transfer";
  amount_cop: number;
  account_from: string | null;
  account_to: string | null;
  category_id: string | null;
  goal_id: string | null;
  loan_id: string | null;
  note: string | null;
  is_extraordinary: boolean;
}

export interface EntryFilter {
  start?: string | null;
  end?: string | null;
  type?: string | null;
  category_id?: string | null;
  search_note?: string | null;
  only_extraordinary?: boolean | null;
  page?: number | null;
  page_size?: number | null;
}

export interface EntryPage {
  entries: Entry[];
  total_count: number;
  filtered_income: number;
  filtered_expense: number;
}

export interface BudgetV2 {
  category_id: string;
  monthly_cop: number;
  updated_at: string;
}

export interface CategoryBudgetRow {
  category: Category;
  monthly_cop: number;
}

export type PeriodV2 =
  | { type: "Month"; value: { year: number; month: number } }
  | { type: "Year"; value: { year: number } }
  | { type: "Custom"; value: { start: string; end: string } };

export interface PeriodRange {
  start: string;
  end: string;
  partial: boolean;
}

export interface PeriodSummaryV2 {
  range: PeriodRange;
  total_income: number;
  total_expense: number;
  balance: number;
  extraordinary_income: number;
  extraordinary_expense: number;
  entries_count: number;
}

export interface CategoryProgressV2 {
  category_id: string;
  category_name: string;
  type: "income" | "expense";
  is_fixed: boolean;
  monthly_target: number;
  current_amount: number;
  percentage: number;
  is_over: boolean;
}

export interface CategoryComparisonV2 {
  category_id: string;
  category_name: string;
  current: number;
  previous: number;
  delta_pct: number;
}

export interface MonthComparisonV2 {
  current_month_total: number;
  previous_month_total: number;
  delta_amount: number;
  delta_percentage: number;
  by_category: CategoryComparisonV2[];
}

export interface GoalV2 {
  id: string;
  name: string;
  target_cop: number;
  target_date: string | null;
  kind: "saving" | "debt";
  installments: number | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface GoalInputV2 {
  name: string;
  target_cop: number;
  target_date: string | null;
  type: "saving" | "debt";
  installments?: number | null;
}

export interface GoalWithProgressV2 {
  goal: GoalV2;
  current_amount: number;
  pending: number;
  percentage: number;
}

export interface GoalDetailV2 {
  goal: GoalWithProgressV2;
  contributions: Entry[];
}

export interface DebtGoalInput {
  name: string;
  target_cop: number;
  occurred_on: string;
  category_id: string;
  installments?: number | null;
  note?: string | null;
}

export interface DebtGoalResult {
  goal: GoalWithProgressV2;
  entry: Entry;
}

export interface LoanV2 {
  id: string;
  person_name: string;
  principal_cop: number;
  lent_on: string;
  note: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface LoanInputV2 {
  person_name: string;
  principal_cop: number;
  lent_on: string;
  note: string | null;
}

export interface LoanWithBalanceV2 {
  loan: LoanV2;
  paid: number;
  pending: number;
  status: "pendiente" | "pagado";
}

export interface MetaAbonoV2 {
  id: string;
  date: string;
  amount: number;
}

export interface MetaV2 {
  id: string; // "loan:<id>" | "goal:<id>"
  tipo: "me_deben" | "debo" | "quiero_juntar";
  nombre: string;
  total: number;
  abonado: number;
  pendiente: number;
  estado: string;
  fecha: string | null;
  nota: string | null;
  cuotas: number | null;
  abonos: MetaAbonoV2[];
}

export interface MetaPaymentInput {
  meta_id: string;
  amount_cop: number;
  occurred_on: string;
  note?: string | null;
}

export interface VehicleV2 {
  id: string;
  name: string;
  efficiency_m_per_l: number;
  tank_capacity_ml: number | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface VehicleInputV2 {
  name: string;
  km_per_gallon: number;
  tank_gallons: number;
}

export interface FillupV2 {
  id: string;
  occurred_on: string;
  vehicle_id: string;
  volume_ml: number;
  price_cop_per_gallon: number;
  total_cop: number;
  entry_id: string | null;
  note: string | null;
}

export interface FillupInputV2 {
  vehicle_id: string;
  occurred_on: string;
  total_cop: number;
  price_cop_per_gallon: number;
  entry_id?: string | null;
  note?: string | null;
}

export interface FillupResult {
  fillup: FillupV2;
  warning: string | null;
}

export interface FillupWithExpenseInput {
  vehicle_id: string;
  occurred_on: string;
  total_cop: number;
  price_cop_per_gallon: number;
  category_id: string;
  note?: string | null;
}

export interface FillupWithExpenseResult {
  entry: Entry;
  fillup: FillupV2;
  warning: string | null;
}

export interface TripInputV2 {
  vehicle_id: string;
  occurred_on: string;
  distance_m: number;
  entry_id?: string | null;
}

export interface Trip {
  id: string;
  occurred_on: string;
  vehicle_id: string;
  distance_m: number;
  consumed_ml: number;
  entry_id: string | null;
}

export interface TankLevel {
  level_ml: number;
  raw_level_ml: number;
  autonomy_m: number;
  tank_percentage: number | null;
}

export interface FuelAdjustment {
  id: string;
  occurred_on: string;
  vehicle_id: string;
  level_ml: number;
  note: string | null;
}

export interface FuelLevelResetInput {
  vehicle_id: string;
  level_ml: number;
  occurred_on: string;
  note?: string | null;
}

export interface RouteV2 {
  id: string;
  name: string;
  distance_m: number;
  description: string | null;
}

export interface RouteInputV2 {
  name: string;
  km_round_trip: number;
  description?: string | null;
}

// ── Sin cambios respecto a v1: gas_prices no se toca con la migración ──
export interface GasPrice {
  id: number;
  date: string;
  price_per_gallon: number;
  source: string;
}

export interface WeeklyGasPoint {
  week_start: string;
  avg_price: number;
  entry_count: number;
}

export interface RoutesCost {
  precio_galon: number;
}

export interface CsvExport {
  content: string;
  suggested_filename: string;
}

export interface AppError {
  kind: string;
  message?: string;
}
