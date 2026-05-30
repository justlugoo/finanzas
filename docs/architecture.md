# Arquitectura — Finanzas

Referencia técnica del proyecto: stack, estructura, base de datos, comandos y convenciones.

---

## 1. Stack

| Capa | Tecnología |
|------|-----------|
| Framework desktop | Tauri 2.x |
| Backend | Rust (stable) |
| Frontend | Svelte 5 (runes API) |
| Estilos | CSS puro con variables — sin frameworks UI |
| Base de datos | SQLite local vía `libsql` (modo local, sin sync cloud) |
| Gestor de paquetes | pnpm |
| Empaquetado | `tauri build` → `.rpm` y `.deb` |

**Plataforma objetivo:** Linux (Fedora 44+). No se genera AppImage (requiere FUSE 2, no disponible por defecto en Fedora).

---

## 2. Estructura del proyecto

```
Finanzas/
├── src/                        # Frontend Svelte
│   ├── app.css                 # Variables CSS globales, tema oscuro
│   ├── lib/
│   │   ├── constants.ts        # MESES, MESES_CORTO, DIAS_SEMANA
│   │   ├── types.ts            # Interfaces TypeScript espejo de structs Rust
│   │   ├── api/                # Capa de acceso al backend (centraliza invoke)
│   │   │   ├── index.ts        # Re-exports de todos los módulos
│   │   │   ├── transactions.ts
│   │   │   ├── budgets.ts
│   │   │   ├── goals.ts
│   │   │   ├── loans.ts
│   │   │   ├── gas.ts
│   │   │   ├── vehicles.ts
│   │   │   ├── routes.ts
│   │   │   └── system.ts
│   │   └── components/
│   │       ├── CustomSelect.svelte
│   │       ├── DatePicker.svelte
│   │       ├── PaymentModal.svelte
│   │       └── ScrollArea.svelte
│   └── routes/
│       ├── +layout.svelte      # Layout global: nav + widget sidebar
│       ├── +page.svelte        # Resumen (dashboard)
│       ├── registrar/+page.svelte
│       ├── historial/+page.svelte
│       ├── metas/+page.svelte
│       └── config/+page.svelte
├── src-tauri/
│   └── src/
│       ├── main.rs             # Entry point
│       ├── lib.rs              # Setup Tauri: tray, autostart, invoke_handler
│       ├── error.rs            # AppError con Serialize
│       ├── db.rs               # Schema SQL, open_database(), apply_pragmas()
│       ├── state.rs            # DbState, ConnGuard, get_conn()
│       ├── utils.rs            # Helpers puros: format_cop, period_to_dates, etc.
│       ├── models/
│       │   └── mod.rs          # Todos los tipos: Transaction, Budget, Goal, …
│       ├── repositories/       # Solo SQL — sin lógica de negocio
│       │   ├── mod.rs
│       │   ├── transactions.rs
│       │   ├── budgets.rs
│       │   ├── goals.rs
│       │   ├── loans.rs
│       │   ├── gas.rs
│       │   ├── vehicles.rs
│       │   └── routes.rs
│       ├── services/           # Lógica de negocio — orquesta repositorios
│       │   ├── mod.rs
│       │   ├── transactions.rs
│       │   ├── budgets.rs
│       │   ├── goals.rs
│       │   ├── loans.rs
│       │   ├── metas.rs
│       │   ├── gas.rs
│       │   ├── vehicles.rs
│       │   ├── routes.rs
│       │   └── system.rs
│       └── commands/           # Handlers Tauri delgados — solo adaptadores
│           ├── mod.rs
│           ├── transactions.rs
│           ├── budgets.rs
│           ├── goals.rs
│           ├── loans.rs
│           ├── metas.rs
│           ├── gas.rs
│           ├── vehicles.rs
│           ├── routes.rs
│           └── system.rs
├── docs/
│   └── architecture.md        # Este archivo
├── README.md
├── LICENSE.md
├── package.json
├── pnpm-lock.yaml
├── pnpm-workspace.yaml
├── tsconfig.json
└── vite.config.ts
```

---

## 3. Arquitectura general

El proyecto sigue una **Arquitectura Layered** (por capas de responsabilidad técnica). Cada capa solo habla con la inmediatamente inferior — nunca con capas no adyacentes.

```
┌───────────────────────────────────────────────────────┐
│                  Tauri App (Finanzas)                 │
│                                                       │
│  ┌─────────────────────────────────────────────────┐  │
│  │               Frontend (Svelte 5)               │  │
│  │  routes/  ──invoke via──►  lib/api/  ──► Tauri  │  │
│  └─────────────────────────────────────────────────┘  │
│                          ▲                            │
│  ┌───────────────────────┼──────────────────────────┐ │
│  │              Backend (Rust)                      │ │
│  │                                                  │ │
│  │  commands/  ──►  services/  ──►  repositories/  │ │
│  │  (delgado)      (lógica)          (solo SQL)    │ │
│  │                                                  │ │
│  │          models/ · state.rs · utils.rs           │ │
│  └──────────────────────────────────────────────────┘ │
│                          │                            │
│  ┌───────────────────────▼──────────────────────────┐ │
│  │    SQLite local (libsql)                         │ │
│  │    ~/.local/share/finanzas/local.db              │ │
│  └──────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────┘
```

**Responsabilidad de cada capa:**

| Capa | Responsabilidad |
|------|----------------|
| `commands/` | Adaptadores Tauri: llaman `get_conn()` y delegan al servicio. Sin lógica. |
| `services/` | Lógica de negocio: validan, coordinan repositorios, calculan y notifican. |
| `repositories/` | SQL puro: aceptan `&libsql::Connection`, retornan tipos del dominio. |
| `models/` | Tipos compartidos (structs, enums) usados en todas las capas. |
| `state.rs` | `DbState`, `ConnGuard`, `get_conn()` — gestión de la conexión lazy. |
| `utils.rs` | Helpers puros sin dependencias de dominio. |
| `lib/api/` | Centraliza todos los `invoke()` del frontend. Las páginas nunca llaman `invoke()` directamente. |

No hay sincronización cloud ni servidor externo.

### Estado compartido en Rust

```rust
pub struct DbState {
    pub db:   Arc<RwLock<Option<libsql::Database>>>,
    pub conn: Arc<tokio::sync::Mutex<Option<libsql::Connection>>>,
}
```

La conexión se crea lazy en el primer `get_conn()` y se reutiliza en todas las llamadas siguientes. Un `Mutex` garantiza acceso secuencial (libsql no es `Send` en modo local). Si la DB aún no está lista al arrancar, `get_conn()` espera hasta 3 segundos antes de retornar error.

---

## 4. Base de datos

**Diagrama ER:** [`docs/Diagrams/DiagramaER.png`](Diagrams/DiagramaER.png)  
**Schema completo:** [`docs/schema_finanzas.sql`](schema_finanzas.sql)



**Motor:** SQLite via `libsql` (modo local)  
**Ubicación:**
- Release: `~/.local/share/finanzas/local.db`
- Debug (`pnpm tauri dev`): `~/.local/share/finanzas-dev/local.db`

El aislamiento de path evita que las pruebas de desarrollo modifiquen los datos reales del usuario.

**Inicialización:** El schema se aplica en cada arranque con `CREATE TABLE IF NOT EXISTS` (idempotente). Las migraciones destructivas (ej. eliminación de columnas) se detectan vía `pragma_table_info` y reconstruyen la tabla afectada preservando datos.

**PRAGMAs activos** (aplicados en cada conexión):

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA cache_size   = -65536;   -- 64 MB caché de páginas
PRAGMA mmap_size    = 268435456; -- 256 MB mmap
PRAGMA temp_store   = MEMORY;
```

### Tablas

9 tablas en total. Las dos tablas de préstamos (`loans`, `loan_payments`) se crearon en v1.1.0 como migración aditiva — bases de datos anteriores las adquieren automáticamente en el primer arranque post-actualización.

#### `transactions`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| date | TEXT | `YYYY-MM-DD` |
| type | TEXT | `ingreso` \| `gasto` |
| category | TEXT | Nombre de categoría (string directo) |
| amount | INTEGER | Valor en COP (siempre positivo) |
| note | TEXT | Nullable |
| is_extraordinary | INTEGER | `0` \| `1` — excluye del cálculo de `CategoryProgress.current_amount` (progreso de presupuesto), pero sí se suma en los totales generales de `PeriodSummary` |
| goal_id | INTEGER | FK → `goals.id` ON DELETE SET NULL. Nullable |
| created_at | TEXT | ISO timestamp |
| is_debt | INTEGER | `0` \| `1` — gasto financiado a futuro |

Índices: `(date)`, `(category)`, `(date, category)`.

#### `budgets`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| category | TEXT PK | Nombre de categoría |
| monthly_amount | INTEGER | Meta mensual en COP (puede ser 0) |
| route_id | INTEGER | FK → `custom_routes.id` ON DELETE SET NULL. Nullable |
| type | TEXT | `ingreso` \| `gasto` |
| is_fixed | INTEGER | `0` \| `1` — solo relevante para ingresos |

#### `goals`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| name | TEXT | Nombre del objetivo |
| target_amount | INTEGER | Monto objetivo en COP |
| target_date | TEXT | `YYYY-MM-DD`. Nullable |
| status | TEXT | `activo` \| `completado` \| `pausado`. El módulo Metas auto-deriva `completado` cuando `current_amount >= target_amount`; `pausado` es solo a nivel DB |
| created_at | TEXT | ISO timestamp |
| is_debt_goal | INTEGER | `0` \| `1` — objetivo de tipo deuda |

`current_amount` no se almacena: se calcula con `SELECT SUM(amount) FROM transactions WHERE goal_id = ?`.

Los goals con `is_debt_goal = 1` son idempotentes por nombre: al registrar un gasto con `is_debt = true`, el backend busca primero un goal existente con ese nombre (`"Deuda: <nota>"` o `"Deuda: <categoría>"`). Si existe lo reutiliza; si no, lo crea. La transacción origen queda vinculada al goal vía `goal_id`.

#### `gas_prices`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| date | TEXT UNIQUE | `YYYY-MM-DD` — un precio por día (UPSERT) |
| price_per_gallon | INTEGER | COP. Rango válido: 1.000–100.000 |
| source | TEXT | `manual` \| `scraping` |

#### `custom_routes`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| name | TEXT | Nombre descriptivo |
| km_round_trip | REAL | Kilómetros ida y vuelta |
| description | TEXT | Nullable |

#### `vehicles`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| name | TEXT | Nombre del vehículo |
| km_per_gallon | REAL | Rendimiento. Debe ser > 0 |

#### `config`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| key | TEXT PK | Identificador de la configuración |
| value | TEXT | Valor serializado como string |

#### `loans` _(v1.1.0)_
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| person_name | TEXT | Nombre del deudor (texto libre) |
| amount | INTEGER | Monto original prestado en COP. Debe ser > 0 |
| date | TEXT | `YYYY-MM-DD` — fecha del préstamo |
| note | TEXT | Nullable |
| status | TEXT | `pendiente` \| `pagado`. Calculado automáticamente al registrar abonos |
| created_at | TEXT | ISO timestamp |

Índices: `(status)`, `(person_name)`.

#### `loan_payments` _(v1.1.0)_
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| loan_id | INTEGER | Referencia lógica a `loans.id` — **sin FK explícita** (libsql compila con `SQLITE_DEFAULT_FOREIGN_KEYS=1` y las FK explícitas bloquean INSERTs) |
| amount | INTEGER | Monto del abono en COP. Debe ser > 0 |
| date | TEXT | `YYYY-MM-DD` — fecha del abono |
| created_at | TEXT | ISO timestamp |

Índice: `(loan_id)`.

**Lógica de negocio de préstamos:**
- `paid` y `pending` son campos calculados: `paid = SUM(loan_payments.amount)`, `pending = amount - paid`.
- Al registrar un abono, el servicio verifica que `paid + nuevo_abono ≤ amount` (rechaza si se supera).
- Si `paid ≥ amount` tras el abono, el repositorio actualiza `loans.status = 'pagado'` automáticamente.
- `loans_total_pending` calcula en SQL la suma de `pending` de todos los préstamos con `status = 'pendiente'`.

---

## 5. Convenciones Rust

**Arquitectura por capas:**
- `repositories/` solo ejecutan SQL. No validan, no calculan, no notifican.
- `services/` son los únicos que validan datos de entrada, coordinan repositorios, aplican cálculos y envían notificaciones.
- `commands/` son adaptadores delgados: llaman `get_conn()` y delegan al servicio. No contienen lógica de negocio.
- `models/mod.rs` concentra todos los tipos. Ninguna capa define tipos propios fuera de este módulo.

**Tipos y serialización:**
- Errores: `AppResult<T>` = `Result<T, AppError>`. `AppError` implementa `Serialize` para que Tauri lo envíe al frontend.
- Todos los comandos son `async` y reciben `State<'_, DbState>`.
- Montos en COP: siempre `i64` (enteros, sin decimales).
- Fechas: `String` en formato `YYYY-MM-DD` o `YYYY-MM-DD HH:MM:SS`.
- Booleanos: `bool` en Rust, `INTEGER 0/1` en SQLite. La conversión la hace el backend.
- Campo `type` en structs Rust: `#[serde(rename = "type")] kind: String` (`type` es palabra reservada).
- Parámetros camelCase del frontend (ej. `monthlyAmount`) son deserializados automáticamente a snake_case por Tauri/serde.

---

## 6. Convenciones Svelte / TypeScript

**Capa de API:**
- Toda llamada al backend va a través de `$lib/api/`. Ninguna página llama `invoke()` directamente.
- Importar los módulos así: `import { transactionApi, budgetApi } from "$lib/api"`.
- Cada módulo de `$lib/api/` exporta funciones tipadas que encapsulan el `invoke()` correspondiente.

**Svelte y estilos:**
- Svelte 5 runes: `$state()`, `$derived()`, `$effect()`. No usar sintaxis legacy de Svelte 4.
- Tipos TypeScript en `src/lib/types.ts`: espejo de los structs Rust. Mantener sincronizados manualmente.
- Constantes de fechas y días en `src/lib/constants.ts`: `MESES`, `MESES_CORTO`, `DIAS_SEMANA`.
- CSS por componente en el bloque `<style>`. Variables globales en `app.css`.
- Tema: oscuro siempre. Fondo base `#0c0c14`, acento `#7c6bff`.

---

## 7. Comandos disponibles

Todos los comandos se invocan con `invoke('nombre', { params })`. Retornan `Promise<T>` o lanzan un error con forma `{ kind: string, message?: string }`.

### Transacciones

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `create_transaction` | `input: TransactionInput` | `Transaction` |
| `list_transactions` | `filter: TransactionFilter` | `TransactionPage` |
| `update_transaction` | `id: i64, input: TransactionInput` | `Transaction` |
| `delete_transaction` | `id: i64` | `()` |
| `delete_transactions_bulk` | `ids: Vec<i64>` | `i64` (cantidad eliminada) |
| `get_current_balance` | — | `CurrentBalance` — incluye `cash_on_hand` y `net_worth` desde v1.1.0 |
| `export_transactions_csv` | `filter: TransactionFilter` | `CsvExport` — header: `ID,Fecha,Tipo,Categoría,Monto (COP),Nota,Extraordinario,ID Objetivo,Es deuda,Creado en` |
| `import_transactions_csv` | `content: String` | `ImportResult` — lee columnas `ID Objetivo` y `Es deuda`; si no están presentes (CSVs antiguos), defaultean a `NULL`/`false` sin error |

**`TransactionInput`:**
```
date, type, category, amount, note?, is_extraordinary,
goal_id?, gas_km?, is_debt?, vehicle_id?
```
Si `gas_km > 0`, `vehicle_id` es obligatorio. El backend inserta automáticamente una transacción de gasto de gasolina calculando `(gas_km / km_per_gallon) * precio_galon`.

### Resumen y analytics

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `get_period_summary` | `period: Period` | `PeriodSummary` |
| `get_category_progress` | `period: Period` | `Vec<CategoryProgress>` |
| `get_month_comparison` | — | `MonthComparison` |

**`Period`:** `{ type: "Daily" }` \| `{ type: "Weekly" }` \| `{ type: "Monthly" }` \| `{ type: "Yearly" }` \| `{ type: "Custom", value: { start, end } }`

### Categorías (budgets)

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `list_budgets` | — | `Vec<Budget>` |
| `create_budget` | `category, monthlyAmount, kind, isFixed?` | `Budget` |
| `update_budget` | `category, monthlyAmount` | `Budget` |
| `update_budget_fixed` | `category, isFixed` | `Budget` |
| `update_budget_route` | `category, routeId` | `()` |
| `delete_budget` | `category` | `()` |
| `list_categories` | `kind?: String` | `Vec<String>` |

### Objetivos

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `list_goals` | `status?: String` | `Vec<GoalWithProgress>` |
| `get_goal_detail` | `id: i64` | `GoalDetail` |
| `create_goal` | `input: GoalInput` | `GoalWithProgress` |
| `update_goal` | `id: i64, input: GoalInput` | `GoalWithProgress` |
| `delete_goal` | `id: i64` | `()` |

`status` válidos: `"activo"`, `"completado"`, `"pausado"`.  
Al eliminar un objetivo, las transacciones asociadas pierden el `goal_id` (FK ON DELETE SET NULL).

### Préstamos

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `loan_create` | `input: LoanInput` | `LoanWithBalance` |
| `loan_list` | — | `Vec<LoanWithBalance>` |
| `loan_get` | `id: i64` | `LoanWithBalance` |
| `loan_update` | `id: i64, input: LoanUpdateInput` | `LoanWithBalance` |
| `loan_add_payment` | `input: LoanPaymentInput` | `LoanWithBalance` |
| `loan_delete` | `id: i64` | `()` |
| `loans_total_pending` | — | `i64` (suma de `pending` de préstamos con `status = 'pendiente'`) |

`loan_delete` elimina en cascada manual todos los `loan_payments` del préstamo antes de borrar el registro en `loans`.  
`loan_add_payment` rechaza con `ValidationError` si el abono haría que la suma supere el monto original.  
`loan_update` rechaza con `ValidationError` si el nuevo monto es menor que la suma ya abonada.

### Metas

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `metas_list` | — | `Vec<Meta>` |

Vista unificada que agrega préstamos (`me_deben`), goals de deuda (`debo`) y goals de ahorro (`quiero_juntar`) en un solo tipo normalizado `Meta`. El campo `estado` se auto-deriva del progreso: `completado` si `abonado >= total`, `pendiente` en caso contrario.

### Gasolina

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `get_current_gas_price` | — | `GasPrice \| null` |
| `list_gas_prices` | `limit: i64` | `Vec<GasPrice>` |
| `register_gas_price_manual` | `price: i64` | `GasPrice` |
| `get_weekly_gas_comparison` | — | `Vec<WeeklyGasPoint>` |
| `get_route_costs` | — | `RoutesCost` |

`register_gas_price_manual` hace UPSERT por fecha vía `ON CONFLICT(date) DO UPDATE`. Preserva el `id` de la fila existente y fuerza `source = 'manual'`. El registro manual siempre gana en colisión de fecha (decisión explícita, no efecto secundario de `REPLACE`).

### Vehículos

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `list_vehicles` | — | `Vec<Vehicle>` |
| `create_vehicle` | `input: VehicleInput` | `Vehicle` |
| `update_vehicle` | `id: i64, input: VehicleInput` | `Vehicle` |
| `delete_vehicle` | `id: i64` | `()` |

### Rutas personalizadas

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `get_custom_routes` | — | `Vec<CustomRoute>` |
| `save_custom_route` | `route: CustomRouteInput` | `CustomRoute` |
| `delete_custom_route` | `id: i64` | `()` |

### Sistema

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `get_autostart_enabled` | — | `bool` |
| `set_autostart_enabled` | `enabled: bool` | `()` |
| `backup_database` | — | `String` (path del backup) |
| `factory_reset` | — | `()` |

`factory_reset` elimina todas las filas de: `transactions`, `goals`, `gas_prices`, `budgets`, `custom_routes`, `vehicles`. También borra `sqlite_sequence` para que los próximos inserts arranquen desde `id = 1` en todas las tablas con `AUTOINCREMENT`. La tabla `config` no se toca.

---

## 8. Tipos TypeScript principales

```typescript
interface Transaction {
  id: number; date: string; type: string; category: string;
  amount: number; note: string | null; is_extraordinary: boolean;
  goal_id: number | null; created_at: string; is_debt: boolean;
}

interface TransactionInput {
  date: string; type: string; category: string; amount: number;
  note: string | null; is_extraordinary: boolean; goal_id: number | null;
  gas_km: number | null; is_debt?: boolean; vehicle_id?: number | null;
}

interface Budget {
  category: string; monthly_amount: number;
  route_id: number | null; type: "ingreso" | "gasto"; is_fixed: boolean;
}

interface Goal {
  id: number; name: string; target_amount: number;
  target_date: string | null; status: string;
  created_at: string; is_debt_goal: boolean;
}

interface GoalWithProgress {
  goal: Goal; current_amount: number; percentage: number;
  monthly_required: number | null;
  projected_completion_date: string | null; on_track: boolean;
}

interface Vehicle { id: number; name: string; km_per_gallon: number; }

interface CustomRoute {
  id: number; name: string; km_round_trip: number; description: string | null;
}

interface CategoryProgress {
  category: string; monthly_target: number; current_amount: number;
  percentage: number; is_over: boolean; kind: string; is_fixed: boolean;
}

type Period =
  | { type: "Daily" } | { type: "Weekly" }
  | { type: "Monthly" } | { type: "Yearly" }
  | { type: "Custom"; value: { start: string; end: string } };

// v1.1.0: cash_on_hand y net_worth añadidos
interface CurrentBalance {
  total_income: number;
  total_expenses: number;
  balance: number;
  cash_on_hand: number;  // balance − total préstamos pendientes por cobrar
  net_worth: number;     // balance (= cash_on_hand + préstamos pendientes)
}

// v1.1.0: préstamos a terceros
interface Loan {
  id: number; person_name: string; amount: number;
  date: string; note: string | null;
  status: "pendiente" | "pagado"; created_at: string;
}

interface LoanPayment {
  id: number; loan_id: number; amount: number;
  date: string; created_at: string;
}

interface LoanWithBalance {
  loan: Loan;
  paid: number;    // SUM(loan_payments.amount)
  pending: number; // loan.amount - paid
  payments: LoanPayment[];
}

interface LoanInput {
  person_name: string; amount: number;
  date: string; note: string | null;
}

interface LoanPaymentInput {
  loan_id: number; amount: number; date: string;
}

interface LoanUpdateInput {
  person_name: string; amount: number;
}

// Metas: vista unificada de préstamos, deudas y ahorros
interface MetaAbono { id: number; date: string; amount: number; }

interface Meta {
  id: string;           // "loan:{id}" | "goal:{id}"
  tipo: string;         // "me_deben" | "debo" | "quiero_juntar"
  nombre: string;
  total: number;
  abonado: number;
  pendiente: number;
  estado: string;       // "completado" | "pendiente" — auto-derivado
  fecha: string | null;
  nota: string | null;
  cuotas: number | null;
  abonos: MetaAbono[];
  on_track: boolean | null;                    // solo quiero_juntar
  monthly_required: number | null;             // solo quiero_juntar
  projected_completion_date: string | null;    // solo quiero_juntar
}
```

---

## 9. System tray y ciclo de vida

- Al cerrar la ventana (botón X): si el tray está activo, la ventana se oculta en lugar de cerrarse. La app sigue corriendo en segundo plano.
- Click izquierdo en el ícono del tray: toggle mostrar/ocultar ventana.
- Menú del tray: **Abrir Finanzas** y **Salir**.
- En GNOME puro (Fedora sin extensión AppIndicator): `libappindicator` no está disponible. El tray falla silenciosamente y el cierre de la ventana termina el proceso.
- **Autoarranque:** registrado vía `tauri-plugin-autostart` con el flag `--autostart`. Si la app arranca con ese flag, la ventana permanece oculta.
- En binarios debug, el autoarranque se desregistra automáticamente para evitar que el binario de desarrollo quede apuntando al `.desktop` entry.

---

## 10. Cálculo de costo de gasolina

Al registrar un movimiento con `gas_km > 0`:

1. El frontend requiere que `vehicle_id` esté seleccionado.
2. El backend llama `insert_auto_gas(date, category, gas_km, vehicle_id)`.
3. Se consulta `km_per_gallon` del vehículo y el `price_per_gallon` más reciente.
4. Costo: `round(gas_km / km_per_gallon * price_per_gallon)` en COP.
5. Se inserta una transacción adicional de tipo `gasto` en la categoría de gasolina de la ruta asociada al presupuesto de la categoría original.
6. Si no hay precio de gasolina registrado → error descriptivo. Si no existe el vehículo → error.

---

## 11. Errores

```rust
pub enum AppError {
    NotFound(String),
    ValidationError(String),
    DatabaseError(String),
    IoError(String),
}
```

Serializado con `#[serde(tag = "kind", content = "message")]`. El frontend recibe `{ kind: "ValidationError", message: "..." }` y puede mostrar el mensaje directamente al usuario.
