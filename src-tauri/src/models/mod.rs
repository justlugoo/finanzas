use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct Budget {
    pub category: String,
    pub monthly_amount: i64,
    pub route_id: Option<i64>,
    pub r#type: String,
    pub is_fixed: bool,
}

#[derive(Serialize, Debug)]
pub struct Transaction {
    pub id: i64,
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
    pub created_at: String,
    pub is_debt: bool,
    pub gas_km: Option<f64>,
    pub trip_vehicle_id: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct CurrentBalance {
    pub total_income: i64,
    pub total_expenses: i64,
    pub balance: i64,
    pub cash_on_hand: i64,
    pub net_worth: i64,
}

#[derive(Serialize, Debug)]
pub struct TransactionPage {
    pub transactions: Vec<Transaction>,
    pub total_count: i64,
    pub filtered_income: i64,
    pub filtered_expenses: i64,
}

#[derive(Deserialize, Debug)]
pub struct TransactionInput {
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
    #[serde(default)]
    pub gas_km: Option<f64>,
    #[serde(default)]
    pub is_debt: bool,
    #[serde(default)]
    pub vehicle_id: Option<i64>,
    #[serde(default)]
    pub installments: Option<i64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct TransactionFilter {
    pub period: Option<Period>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub search_note: Option<String>,
    pub only_extraordinary: Option<bool>,
    pub only_debt: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Custom { start: String, end: String },
}

#[derive(Serialize, Debug)]
pub struct PeriodSummary {
    pub total_income: i64,
    pub total_expenses: i64,
    pub balance: i64,
    pub extraordinary_income: i64,
    pub extraordinary_expenses: i64,
    pub transactions_count: i64,
}

#[derive(Serialize, Debug)]
pub struct CategoryProgress {
    pub category: String,
    pub monthly_target: i64,
    pub current_amount: i64,
    pub percentage: f64,
    pub is_over: bool,
    pub kind: String,
    pub is_fixed: bool,
}

pub struct CategoryProgressRaw {
    pub category: String,
    pub monthly_amount: i64,
    pub current_amount: i64,
    pub kind: String,
    pub is_fixed: bool,
}

#[derive(Serialize, Debug)]
pub struct CategoryComparison {
    pub category: String,
    pub current: i64,
    pub previous: i64,
    pub delta_pct: f64,
}

#[derive(Serialize, Debug)]
pub struct MonthComparison {
    pub current_month_total: i64,
    pub previous_month_total: i64,
    pub delta_amount: i64,
    pub delta_percentage: f64,
    pub by_category: Vec<CategoryComparison>,
}

#[derive(Serialize, Debug)]
pub struct CsvExport {
    pub content: String,
    pub suggested_filename: String,
}

#[derive(Serialize, Debug)]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: String,
    pub created_at: String,
    pub is_debt_goal: bool,
    pub installments: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct GoalInput {
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GoalWithProgress {
    pub goal: Goal,
    pub current_amount: i64,
    pub percentage: f64,
    pub monthly_required: Option<f64>,
    pub projected_completion_date: Option<String>,
    pub on_track: bool,
}

#[derive(Serialize, Debug)]
pub struct GoalDetail {
    pub goal: GoalWithProgress,
    pub contributions: Vec<Transaction>,
}

#[derive(Serialize, Debug)]
pub struct GasPrice {
    pub id: i64,
    pub date: String,
    pub price_per_gallon: i64,
    pub source: String,
}

#[derive(Serialize, Debug)]
pub struct WeeklyGasPoint {
    pub week_start: String,
    pub avg_price: f64,
    pub entry_count: i64,
}

#[derive(Serialize, Debug)]
pub struct RoutesCost {
    pub precio_galon: i64,
}

#[derive(Serialize, Debug)]
pub struct Vehicle {
    pub id: i64,
    pub name: String,
    pub km_per_gallon: f64,
    pub tank_liters: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct VehicleInput {
    pub name: String,
    pub km_per_gallon: f64,
    #[serde(default)]
    pub tank_liters: Option<f64>,
}

#[derive(Serialize, Debug)]
pub struct CustomRoute {
    pub id: i64,
    pub name: String,
    pub km_round_trip: f64,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CustomRouteInput {
    pub name: String,
    #[serde(default)]
    pub km_round_trip: f64,
    pub description: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct FuelFillup {
    pub id: i64,
    pub date: String,
    pub vehicle_id: i64,
    pub gallons: f64,
    pub price_per_gallon: i64,
    pub total_cost: i64,
    pub note: Option<String>,
    pub created_at: String,
    pub transaction_id: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct FuelFillupInput {
    pub date: String,
    pub vehicle_id: i64,
    pub amount_cop: i64,
    pub category: String,
    pub note: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct VehicleFuelStatus {
    pub vehicle_id: i64,
    pub vehicle_name: String,
    pub km_per_gallon: f64,
    pub tank_liters: Option<f64>,
    pub level_gallons: f64,
    pub autonomy_km: f64,
    pub tank_percentage: Option<f64>,
}

#[derive(Serialize, Debug)]
pub struct Loan {
    pub id: i64,
    pub person_name: String,
    pub amount: i64,
    pub date: String,
    pub note: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize, Debug)]
pub struct LoanPayment {
    pub id: i64,
    pub loan_id: i64,
    pub amount: i64,
    pub date: String,
    pub created_at: String,
}

#[derive(Deserialize, Debug)]
pub struct LoanInput {
    pub person_name: String,
    pub amount: i64,
    pub date: String,
    pub note: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LoanPaymentInput {
    pub loan_id: i64,
    pub amount: i64,
    pub date: String,
}

#[derive(Deserialize, Debug)]
pub struct LoanUpdateInput {
    pub person_name: String,
    pub amount: i64,
}

#[derive(Serialize, Debug)]
pub struct LoanWithBalance {
    pub loan: Loan,
    pub paid: i64,
    pub pending: i64,
    pub payments: Vec<LoanPayment>,
}

#[derive(Serialize, Debug)]
pub struct MetaAbono {
    pub id: i64,
    pub date: String,
    pub amount: i64,
}

// ─────────────────────────────────────────────────────────────────────────
// Esquema v2 (ver schema-v2.md). Conviven con los tipos v1 de arriba
// mientras dura la migración — varios nombres llevan sufijo V2 porque el
// nombre de tabla se reutiliza pero el de struct Rust ya está tomado por v1
// (Goal, Loan, Vehicle, Budget, CustomRoute, FuelFillup).
// ─────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone)]
pub struct Account {
    pub id: String,
    pub code: String,
    pub name: String,
    pub kind: String, // "asset" | "liability"
    pub is_system: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub kind: String, // "income" | "expense"
    pub is_fixed: bool,
    pub route_id: Option<String>,
    /// Categoría de sistema (ej. "Saldo inicial", code="opening"): excluida
    /// de todo reporte de ingresos/gastos/presupuesto/comparativa mensual.
    pub is_system: bool,
    pub code: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CategoryInput {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub is_fixed: bool,
    #[serde(default)]
    pub route_id: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub occurred_on: String,
    #[serde(rename = "type")]
    pub kind: String, // "income" | "expense" | "transfer"
    pub amount_cop: i64,
    pub account_from: Option<String>,
    pub account_to: Option<String>,
    pub category_id: Option<String>,
    pub goal_id: Option<String>,
    pub loan_id: Option<String>,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EntryInput {
    pub occurred_on: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub amount_cop: i64,
    #[serde(default)]
    pub account_from: Option<String>,
    #[serde(default)]
    pub account_to: Option<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub goal_id: Option<String>,
    #[serde(default)]
    pub loan_id: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub is_extraordinary: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct BudgetV2 {
    pub category_id: String,
    pub monthly_cop: i64,
    pub updated_at: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct BudgetOverride {
    pub category_id: String,
    pub year_month: String,
    pub amount_cop: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GoalInputV2 {
    pub name: String,
    pub target_cop: i64,
    pub target_date: Option<String>,
    #[serde(rename = "type")]
    pub kind: String, // "saving" | "debt"
    #[serde(default)]
    pub installments: Option<i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct GoalWithProgressV2 {
    pub goal: GoalV2,
    pub current_amount: i64,
    pub pending: i64,
    pub percentage: f64,
}

/// Entrada para crear una deuda de una vez — centralizado en Metas, no en
/// Registrar (una deuda es una decisión de financiamiento, no algo que
/// solo aparece cuando el disponible no alcanza). Crea el `goal` (kind
/// "debt") y el `expense` real desde `payable` juntos, atómicamente.
#[derive(Deserialize, Debug, Clone)]
pub struct DebtGoalInput {
    pub name: String,
    pub target_cop: i64,
    pub occurred_on: String,
    pub category_id: String,
    #[serde(default)]
    pub installments: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DebtGoalResult {
    pub goal: GoalWithProgressV2,
    pub entry: Entry,
}

#[derive(Serialize, Debug, Clone)]
pub struct GoalDetailV2 {
    pub goal: GoalWithProgressV2,
    pub contributions: Vec<Entry>,
}

#[derive(Serialize, Debug, Clone)]
pub struct GoalV2 {
    pub id: String,
    pub name: String,
    pub target_cop: i64,
    pub target_date: Option<String>,
    pub kind: String, // "saving" | "debt"
    pub installments: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoanInputV2 {
    pub person_name: String,
    pub principal_cop: i64,
    pub lent_on: String,
    pub note: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct LoanWithBalanceV2 {
    pub loan: LoanV2,
    pub paid: i64,
    pub pending: i64,
    pub status: String, // "pendiente" | "pagado" — derivado, no almacenado
}

#[derive(Serialize, Debug, Clone)]
pub struct LoanV2 {
    pub id: String,
    pub person_name: String,
    pub principal_cop: i64,
    pub lent_on: String,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Entrada en unidades "humanas" (km/galón, galones) — toda la app usa
/// galones de forma consistente (precio, nivel, rendimiento, capacidad); la
/// conversión a metros/litro y mililitros ocurre en `services::vehicles_v2`
/// (principio 1 de schema-v2.md: la conversión vive en un único módulo).
/// La capacidad del tanque es obligatoria: sin ella no hay forma de detectar
/// un rendimiento mal configurado (el nivel calculado nunca desborda nada
/// si no hay un tope contra el cual desbordar).
#[derive(Deserialize, Debug, Clone)]
pub struct VehicleInputV2 {
    pub name: String,
    pub km_per_gallon: f64,
    pub tank_gallons: f64,
}

#[derive(Serialize, Debug, Clone)]
pub struct VehicleV2 {
    pub id: String,
    pub name: String,
    pub efficiency_m_per_l: i64,
    pub tank_capacity_ml: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FillupInputV2 {
    pub vehicle_id: String,
    pub occurred_on: String,
    pub total_cop: i64,
    pub price_cop_per_gallon: i64,
    #[serde(default)]
    pub entry_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct FillupResult {
    pub fillup: FillupV2,
    pub warning: Option<String>,
}

/// Entrada de `fillup_create_with_expense_v2` — a diferencia de
/// `FillupInputV2`, esta crea también el `expense` real asociado, por eso
/// pide `category_id` (para el gasto) y no acepta `entry_id` (lo genera).
#[derive(Deserialize, Debug, Clone)]
pub struct FillupWithExpenseInput {
    pub vehicle_id: String,
    pub occurred_on: String,
    pub total_cop: i64,
    pub price_cop_per_gallon: i64,
    pub category_id: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct FillupWithExpenseResult {
    pub entry: Entry,
    pub fillup: FillupV2,
    pub warning: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TripInputV2 {
    pub vehicle_id: String,
    pub occurred_on: String,
    pub distance_m: i64,
    #[serde(default)]
    pub entry_id: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct FillupV2 {
    pub id: String,
    pub occurred_on: String,
    pub vehicle_id: String,
    pub volume_ml: i64,
    pub price_cop_per_gallon: i64,
    pub total_cop: i64,
    pub entry_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Trip {
    pub id: String,
    pub occurred_on: String,
    pub vehicle_id: String,
    pub distance_m: i64,
    pub consumed_ml: i64,
    pub entry_id: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct FuelAdjustment {
    pub id: String,
    pub occurred_on: String,
    pub vehicle_id: String,
    pub level_ml: i64,
    pub note: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RouteInputV2 {
    pub name: String,
    pub km_round_trip: f64,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RouteV2 {
    pub id: String,
    pub name: String,
    pub distance_m: i64,
    pub description: Option<String>,
}

/// Saldos derivados de `entries`, sección 3 de schema-v2.md.
/// `payable` se reporta invertido: una deuda de 500.000 aparece como 500.000,
/// no como -500.000. El álgebra interna (en la query) es uniforme.
/// `apps` es plata ganada (Didi/Uber) aún sin retirar: cuenta para el
/// patrimonio pero no para el disponible.
#[derive(Serialize, Debug, Clone, Default)]
pub struct AccountBalances {
    pub cash: i64,
    pub apps: i64,
    pub savings: i64,
    pub receivable: i64,
    pub payable: i64,
    pub disponible: i64,
    pub patrimonio: i64,
}

/// Entrada del comando "resetear nivel de tanque" — función normal de la
/// app (no de migración): cualquier usuario la puede disparar cuando
/// quiera, sin borrar tanqueos ni viajes ya registrados.
#[derive(Deserialize, Debug, Clone)]
pub struct FuelLevelResetInput {
    pub vehicle_id: String,
    pub level_ml: i64,
    pub occurred_on: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Entrada de `meta_add_payment` (paso 6, sección 12 de schema-v2.md) — un
/// solo comando para los tres tipos de meta. `meta_id` sigue el formato de
/// `Meta.id` en v1: `"loan:<id>"` o `"goal:<id>"`. El frontend deja de
/// decidir si eso es un `ingreso` o un `gasto`: solo manda el monto.
#[derive(Deserialize, Debug, Clone)]
pub struct MetaPaymentInput {
    pub meta_id: String,
    pub amount_cop: i64,
    pub occurred_on: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// `Period` nuevo (sección 8 de schema-v2.md) — reemplaza `period_to_dates`
/// + `scale_monthly` de v1. Distinto del `Period` de arriba (usado por
/// `TransactionFilter`, que sigue vivo mientras dura la migración).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum PeriodV2 {
    Month { year: i32, month: u32 },
    Year { year: i32 },
    Custom { start: String, end: String },
}

/// Todo período resuelve a un rango cerrado. `partial = true` cuando `end`
/// se recortó a hoy porque el período todavía no ha terminado — la UI usa
/// esto para rotular "1–10 de septiembre" en vez de "septiembre", nunca
/// para alterar el cálculo en sí.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct PeriodRange {
    pub start: String,
    pub end: String,
    pub partial: bool,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct EntryFilter {
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub category_id: Option<String>,
    pub search_note: Option<String>,
    pub only_extraordinary: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct EntryPage {
    pub entries: Vec<Entry>,
    pub total_count: i64,
    pub filtered_income: i64,
    pub filtered_expense: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct CategoryProgressV2 {
    pub category_id: String,
    pub category_name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub is_fixed: bool,
    pub monthly_target: i64,
    pub current_amount: i64,
    pub percentage: f64,
    pub is_over: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct CategoryComparisonV2 {
    pub category_id: String,
    pub category_name: String,
    pub current: i64,
    pub previous: i64,
    pub delta_pct: f64,
}

#[derive(Serialize, Debug, Clone)]
pub struct MonthComparisonV2 {
    pub current_month_total: i64,
    pub previous_month_total: i64,
    pub delta_amount: i64,
    pub delta_percentage: f64,
    pub by_category: Vec<CategoryComparisonV2>,
}

/// Nivel de tanque calculado (sección 7 de schema-v2.md). `level_ml` ya
/// está topado a `[0, tank_capacity_ml]`; `raw_level_ml` es el valor sin
/// topar, útil para detectar cuándo un tanqueo superó la capacidad.
#[derive(Serialize, Debug, Clone)]
pub struct TankLevel {
    pub level_ml: i64,
    pub raw_level_ml: i64,
    pub autonomy_m: i64,
    pub tank_percentage: Option<f64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PeriodSummaryV2 {
    pub range: PeriodRange,
    pub total_income: i64,
    pub total_expense: i64,
    pub balance: i64,
    pub extraordinary_income: i64,
    pub extraordinary_expense: i64,
    pub entries_count: i64,
}

/// Categoría + su presupuesto mensual en una sola fila — lo que la UI de
/// Configuración necesita para listar "Presupuestos mensuales" sin hacer
/// una consulta por categoría. `monthly_cop = 0` si no hay fila en `budgets`.
#[derive(Serialize, Debug, Clone)]
pub struct CategoryBudgetRow {
    pub category: Category,
    pub monthly_cop: i64,
}

/// Totales reales del período sobre `entries` — solo `income`/`expense`
/// (los `transfer` quedan fuera). Ver sección 9 de schema-v2.md.
#[derive(Serialize, Debug, Clone, Default)]
pub struct PeriodTotals {
    pub total_income: i64,
    pub total_expense: i64,
    pub extraordinary_income: i64,
    pub extraordinary_expense: i64,
}

/// Reporte de la migración 001 — una fila por registro trasladado más una
/// sección de excepciones. Se vuelca a `migracion_reporte.csv`.
#[derive(Serialize, Debug, Clone)]
pub struct MigrationRow {
    pub source_table: String,
    pub source_id: String,
    pub rule: String,
    pub new_entry_id: Option<String>,
    pub needs_review: bool,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MigrationReport {
    pub rows: Vec<MigrationRow>,
    pub exceptions: Vec<String>,
}

/// Igual que `MetaAbono`/`Meta` pero con IDs ULID (String) en vez de i64 —
/// necesario porque v2 identifica todo con ULID, no autoincrement.
#[derive(Serialize, Debug, Clone)]
pub struct MetaAbonoV2 {
    pub id: String,
    pub date: String,
    pub amount: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct MetaV2 {
    pub id: String, // "loan:<id>" | "goal:<id>"
    pub tipo: String,
    pub nombre: String,
    pub total: i64,
    pub abonado: i64,
    pub pendiente: i64,
    pub estado: String,
    pub fecha: Option<String>,
    pub nota: Option<String>,
    pub cuotas: Option<i64>,
    pub abonos: Vec<MetaAbonoV2>,
}

#[derive(Serialize, Debug)]
pub struct Meta {
    pub id: String,
    pub tipo: String,
    pub nombre: String,
    pub total: i64,
    pub abonado: i64,
    pub pendiente: i64,
    pub estado: String,
    pub fecha: Option<String>,
    pub nota: Option<String>,
    pub cuotas: Option<i64>,
    pub abonos: Vec<MetaAbono>,
    pub on_track: Option<bool>,
    pub monthly_required: Option<f64>,
    pub projected_completion_date: Option<String>,
}
