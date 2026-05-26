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
}

#[derive(Deserialize, Debug)]
pub struct VehicleInput {
    pub name: String,
    pub km_per_gallon: f64,
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
}

#[derive(Deserialize, Debug)]
pub struct FuelFillupInput {
    pub date: String,
    pub vehicle_id: i64,
    pub gallons: f64,
    pub price_per_gallon: i64,
    pub note: Option<String>,
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
}
