# Contrato de comandos — Rust ↔ Svelte

API interno entre el backend Tauri (Rust) y el frontend (Svelte). Cada comando se invoca desde Svelte con `invoke('nombre_comando', { args })` y se implementa en Rust con el macro `#[tauri::command]`. Toda la comunicación pasa por serialización JSON (serde).

---

## 1. Convenciones

- **Naming:** `snake_case` en ambos lados.
- **Errores:** todo comando retorna `Result<T, AppError>`. El frontend recibe `T` en éxito o un objeto `{ error: ... }` en fallo.
- **Tipos numéricos:** los montos en COP siempre son `i64` (enteros, sin decimales). Las fechas son `String` en formato ISO 8601 (`YYYY-MM-DD` o `YYYY-MM-DD HH:MM:SS`).
- **Booleanos:** en Rust `bool`, en SQLite `INTEGER 0/1`. La conversión la hace el backend.
- **Async:** todos los comandos son `async`. Las operaciones de DB usan tokio + libsql.
- **Validación:** cada comando valida sus inputs antes de tocar la DB. Los `CHECK` del schema son la última línea de defensa, no la primera.

---

## 2. Tipos de error

```rust
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    NotFound(String),           // recurso inexistente
    ValidationError(String),    // input inválido
    DatabaseError(String),      // error SQL
    NetworkError(String),       // sync o scraping falló
    SyncError(String),
    ScrapingError(String),
    InvalidCredentials,
    IoError(String),            // backup, restore, archivos
}
```

---

## 3. Tipos compartidos

```rust
// ---- Transacciones ----

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub date: String,              // YYYY-MM-DD
    #[serde(rename = "type")]
    pub kind: String,              // "ingreso" | "gasto"
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct TransactionInput {
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub note: Option<String>,
    pub is_extraordinary: bool,
    pub goal_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct TransactionFilter {
    pub period: Option<Period>,
    pub kind: Option<String>,         // "ingreso" | "gasto"
    pub category: Option<String>,
    pub search_note: Option<String>,
    pub only_extraordinary: Option<bool>,
}

// ---- Período ----

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Custom { start: String, end: String },
}

// ---- Resumen ----

#[derive(Serialize)]
pub struct PeriodSummary {
    pub total_income: i64,
    pub total_expenses: i64,
    pub balance: i64,
    pub extraordinary_income: i64,
    pub extraordinary_expenses: i64,
    pub transactions_count: i64,
}

#[derive(Serialize)]
pub struct CategoryProgress {
    pub category: String,
    pub monthly_target: i64,           // 0 si no hay meta
    pub current_amount: i64,
    pub percentage: f64,               // 0.0 hasta >100
    pub is_over: bool,
    pub kind: String,                  // "ingreso" | "gasto" inferido del uso
}

#[derive(Serialize)]
pub struct MonthComparison {
    pub current_month_total: i64,
    pub previous_month_total: i64,
    pub delta_amount: i64,
    pub delta_percentage: f64,
    pub by_category: Vec<CategoryComparison>,
}

#[derive(Serialize)]
pub struct CategoryComparison {
    pub category: String,
    pub current: i64,
    pub previous: i64,
    pub delta_pct: f64,
}

// ---- Objetivos ----

#[derive(Serialize, Deserialize)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: String,                // "activo" | "completado" | "pausado"
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct GoalInput {
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct GoalWithProgress {
    pub goal: Goal,
    pub current_amount: i64,
    pub percentage: f64,
    pub monthly_required: Option<i64>,        // para cumplir target_date
    pub projected_completion_date: Option<String>,  // según ritmo actual
    pub on_track: bool,
}

#[derive(Serialize)]
pub struct GoalDetail {
    pub progress: GoalWithProgress,
    pub contributions: Vec<Transaction>,
}

// ---- Presupuestos ----

#[derive(Serialize, Deserialize)]
pub struct Budget {
    pub category: String,
    pub monthly_amount: i64,
}

// ---- Gasolina ----

#[derive(Serialize)]
pub struct GasPrice {
    pub id: i64,
    pub date: String,
    pub price_per_gallon: i64,
    pub source: String,                // "manual" | "scraping"
}

#[derive(Serialize)]
pub struct WeeklyGasPoint {
    pub week_start: String,            // lunes de la semana
    pub avg_price: i64,
    pub delta_pct_vs_prev: f64,
}

// ---- Sistema ----

#[derive(Serialize)]
pub struct SyncStatus {
    pub last_sync: Option<String>,
    pub pending_writes: i64,
    pub is_online: bool,
}

#[derive(Serialize)]
pub struct CsvExport {
    pub content: String,
    pub suggested_filename: String,
}
```

---

## 4. Comandos por módulo

### 4.1 Módulo Resumen

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `get_period_summary` | `period: Period` | `PeriodSummary` | KPIs agregados del período |
| `get_category_progress` | `period: Period` | `Vec<CategoryProgress>` | Una fila por categoría con meta y avance |
| `get_month_comparison` | — | `MonthComparison` | Mes actual vs mes anterior, total y por categoría |
| `get_recent_transactions` | `limit: i64` | `Vec<Transaction>` | Últimas N transacciones, orden descendente por fecha |

### 4.2 Módulo Registrar

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `list_categories` | `kind: Option<String>` | `Vec<String>` | Lista de categorías; si `kind` viene, filtra por uso histórico (ingreso/gasto) |
| `list_active_goals` | — | `Vec<GoalWithProgress>` | Para llenar el dropdown de objetivos asociables |
| `create_transaction` | `input: TransactionInput` | `Transaction` | Inserta y retorna la transacción creada |

**Validaciones de `create_transaction`:**
- `date` parseable como `YYYY-MM-DD`.
- `kind ∈ {"ingreso", "gasto"}`.
- `category` existe en `budgets`.
- `amount > 0`.
- Si `goal_id` viene, el objetivo existe y está en estado `activo`.

**Side effects:**
- Si la transacción cruza la meta de la categoría en el mes actual → emite evento `notification:budget_exceeded`.
- Si completa un objetivo → emite evento `notification:goal_reached` y actualiza el goal a `status='completado'`.
- Dispara `sync_now()` en background (no bloqueante).

### 4.3 Módulo Historial

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `list_transactions` | `filter: TransactionFilter` | `Vec<Transaction>` | Lista filtrada, orden descendente por fecha |
| `update_transaction` | `id: i64, input: TransactionInput` | `Transaction` | Actualiza y retorna |
| `delete_transaction` | `id: i64` | `()` | Elimina por id |
| `export_transactions_csv` | `filter: TransactionFilter` | `CsvExport` | CSV con headers en español, encoding UTF-8 |

**Side effects en `update` y `delete`:** disparan recálculos automáticos (las queries del dashboard siempre leen DB en vivo, no hay caché). Disparan `sync_now()` en background.

### 4.4 Módulo Objetivos

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `list_goals` | `status: Option<String>` | `Vec<GoalWithProgress>` | Si `status` viene, filtra |
| `get_goal_detail` | `id: i64` | `GoalDetail` | Objetivo + sus transacciones asociadas |
| `create_goal` | `input: GoalInput` | `Goal` | — |
| `update_goal` | `id: i64, input: GoalInput` | `Goal` | — |
| `delete_goal` | `id: i64` | `()` | Las transacciones vinculadas pierden el `goal_id` (FK ON DELETE SET NULL) |

**Cálculos del backend en `GoalWithProgress`:**
- `current_amount` = `SUM(transactions.amount) WHERE goal_id = ?`.
- `monthly_required` = `(target_amount - current_amount) / meses_restantes_hasta_target_date`. NULL si no hay `target_date`.
- `projected_completion_date` = fecha estimada según el ritmo de aportes de los últimos 3 meses. NULL si no hay aportes.
- `on_track` = `projected_completion_date <= target_date`.

### 4.5 Módulo Gasolina

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `get_current_gas_price` | — | `GasPrice` | Precio más reciente |
| `list_gas_prices` | `limit: i64` | `Vec<GasPrice>` | Histórico, descendente por fecha |
| `register_gas_price_manual` | `price: i64` | `GasPrice` | Inserta o actualiza el precio del día (UPSERT) |
| `fetch_gas_price_scraping` | — | `GasPrice` | Intenta scraping; falla si la fuente no responde o el valor es inválido |
| `get_weekly_gas_comparison` | `weeks: i64` | `Vec<WeeklyGasPoint>` | Promedio semanal de las últimas N semanas |
| `calculate_trip_cost` | `km: f64` | `i64` | `(km / consumo_moto_km_galon) * precio_actual`, en COP |

**Side effect en `register_gas_price_manual` y `fetch_gas_price_scraping`:** si el cambio porcentual respecto al precio anterior excede `umbral_alerta_gasolina_pct` (config), emite evento `notification:gas_price_changed`.

### 4.6 Módulo Configuración

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `list_budgets` | — | `Vec<Budget>` | Todas las categorías con su meta |
| `update_budget` | `category: String, monthly_amount: i64` | `Budget` | UPSERT |
| `get_config_value` | `key: String` | `Option<String>` | NULL si la key no existe |
| `set_config_value` | `key: String, value: String` | `()` | UPSERT |
| `list_all_config` | — | `HashMap<String, String>` | Todo el config como diccionario |

### 4.7 Módulo Sistema

| Comando | Parámetros | Retorna | Descripción |
|---|---|---|---|
| `sync_now` | — | `SyncStatus` | Fuerza sync con Turso, retorna estado actualizado |
| `get_sync_status` | — | `SyncStatus` | Estado actual sin forzar sync |
| `has_turso_credentials` | — | `bool` | Verifica si existe el archivo de credenciales |
| `set_turso_credentials` | `url: String, token: String` | `()` | Escribe el archivo de credenciales y reconecta |
| `backup_database` | `target_path: String` | `()` | Copia el `.db` local al path indicado |
| `restore_database` | `source_path: String` | `()` | Reemplaza el `.db` local; requiere reinicio de la app |
| `open_external_link` | `url: String` | `()` | Abre URL en el navegador del sistema (para enlaces a Turso, etc.) |

---

## 5. Eventos push (Rust → Svelte)

Tauri permite que el backend emita eventos sin que el frontend los pida. Útil para notificaciones y actualizaciones reactivas. El frontend los escucha con `listen('nombre_evento', handler)`.

| Evento | Payload | Cuándo se dispara |
|---|---|---|
| `notification:budget_exceeded` | `{ category, current, target, over_by }` | Al guardar transacción que cruza meta |
| `notification:goal_reached` | `{ goal_id, goal_name }` | Al completar un objetivo |
| `notification:gas_price_changed` | `{ old_price, new_price, delta_pct }` | Al cambiar precio significativamente |
| `sync:status_changed` | `SyncStatus` | Al iniciar/terminar sync, o al perder/recuperar conexión |
| `data:transactions_changed` | `()` | Después de crear/editar/eliminar transacciones; el frontend refresca dashboard |
| `data:goals_changed` | `()` | Idem para objetivos |
| `data:gas_prices_changed` | `()` | Idem para precios |

Cuando un comando modifica datos, el backend hace dos cosas: dispara la notificación específica (si aplica) y emite el evento `data:*_changed` correspondiente. El frontend usa el segundo para invalidar sus stores y refetchar.

---

## 6. Flujo de inicialización de la app

Orden de comandos invocados al arrancar Tauri:

1. `has_turso_credentials()` — si retorna `false`, mostrar pantalla de bienvenida pidiendo URL + token.
2. (Si credenciales OK) Backend abre conexión libsql con replica local.
3. Backend ejecuta `schema.sql` (idempotente).
4. Backend hace `sync()` inicial con Turso.
5. Frontend invoca `get_period_summary({ Monthly })` y `get_category_progress({ Monthly })` para llenar el dashboard.
6. Backend registra el ícono del tray y el manejador de notificaciones.

Si `has_turso_credentials()` falla o el sync inicial timeout: la app arranca en modo offline, banner amarillo arriba indica el estado, todas las operaciones funcionan contra la replica local.

---

## 7. Resumen total

7 módulos, 36 comandos, 7 eventos push.

| Módulo | Comandos |
|---|---|
| Resumen | 4 |
| Registrar | 3 |
| Historial | 4 |
| Objetivos | 5 |
| Gasolina | 6 |
| Configuración | 5 |
| Sistema | 7 |

Cada comando es testeable en aislamiento desde Rust con un mock de la DB. El frontend Svelte tipa la interfaz con un archivo `commands.ts` que mapea cada comando a su firma.
