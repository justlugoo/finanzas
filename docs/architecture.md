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
│   │   └── components/
│   │       ├── CustomSelect.svelte
│   │       └── DatePicker.svelte
│   └── routes/
│       ├── +layout.svelte      # Layout global: nav + widget sidebar
│       ├── +page.svelte        # Resumen (dashboard)
│       ├── registrar/+page.svelte
│       ├── historial/+page.svelte
│       ├── objetivos/+page.svelte
│       └── config/+page.svelte
├── src-tauri/
│   └── src/
│       ├── main.rs             # Entry point
│       ├── lib.rs              # Setup Tauri: DbState, tray, autostart, comandos
│       ├── error.rs            # AppError con Serialize
│       ├── db.rs               # Schema SQL, open_database(), apply_pragmas()
│       └── commands.rs         # Todos los comandos #[tauri::command]
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

```
┌─────────────────────────────────────┐
│          Tauri App (Finanzas)       │
│                                     │
│  ┌──────────────┐  ┌─────────────┐  │
│  │  Frontend    │  │  Backend    │  │
│  │  Svelte 5    │◄►│  Rust       │  │
│  │  (WebView)   │  │  (commands) │  │
│  └──────────────┘  └──────┬──────┘  │
│                           │         │
│  ┌────────────────────────▼──────┐  │
│  │  SQLite local (libsql)        │  │
│  │  ~/.local/share/finanzas/     │  │
│  │  local.db                     │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

El frontend invoca comandos Rust mediante `invoke()` de Tauri. El backend lee y escribe directamente en la DB local. No hay sincronización cloud ni servidor externo.

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

**Motor:** SQLite via `libsql` (modo local)  
**Ubicación:** `~/.local/share/finanzas/local.db`  
**Inicialización:** El schema se aplica en cada arranque con `CREATE TABLE IF NOT EXISTS` (idempotente). No hay sistema de migraciones formal — solo `ALTER TABLE ADD COLUMN` para columnas añadidas posteriormente.

**PRAGMAs activos** (aplicados en cada conexión):

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA cache_size   = -65536;   -- 64 MB caché de páginas
PRAGMA mmap_size    = 268435456; -- 256 MB mmap
PRAGMA temp_store   = MEMORY;
```

### Tablas

#### `transactions`
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER PK | Autoincremental |
| date | TEXT | `YYYY-MM-DD` |
| type | TEXT | `ingreso` \| `gasto` |
| category | TEXT | Nombre de categoría (string directo) |
| amount | INTEGER | Valor en COP (siempre positivo) |
| note | TEXT | Nullable |
| is_extraordinary | INTEGER | `0` \| `1` — excluye de progreso de presupuesto |
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
| status | TEXT | `activo` \| `completado` \| `pausado` |
| created_at | TEXT | ISO timestamp |
| is_debt_goal | INTEGER | `0` \| `1` — objetivo de tipo deuda |

`current_amount` no se almacena: se calcula con `SELECT SUM(amount) FROM transactions WHERE goal_id = ?`.

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

---

## 5. Convenciones Rust

- Errores: `AppResult<T>` = `Result<T, AppError>`. `AppError` implementa `Serialize` para que Tauri lo envíe al frontend.
- Todos los comandos son `async` y reciben `State<'_, DbState>`.
- Montos en COP: siempre `i64` (enteros, sin decimales).
- Fechas: `String` en formato `YYYY-MM-DD` o `YYYY-MM-DD HH:MM:SS`.
- Booleanos: `bool` en Rust, `INTEGER 0/1` en SQLite. La conversión la hace el backend.
- Campo `type` en structs Rust: `#[serde(rename = "type")] kind: String` (`type` es palabra reservada).
- Parámetros camelCase del frontend (ej. `monthlyAmount`) son deserializados automáticamente a snake_case por Tauri/serde.

---

## 6. Convenciones Svelte / TypeScript

- Svelte 5 runes: `$state()`, `$derived()`, `$effect()`. No usar sintaxis legacy de Svelte 4.
- Invocar comandos: `invoke<ReturnType>('nombre_comando', { args })`.
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
| `get_current_balance` | — | `CurrentBalance` |
| `export_transactions_csv` | `filter: TransactionFilter` | `CsvExport` |
| `import_transactions_csv` | `content: String` | `ImportResult` |

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

### Gasolina

| Comando | Parámetros | Retorna |
|---------|-----------|---------|
| `get_current_gas_price` | — | `GasPrice \| null` |
| `list_gas_prices` | `limit: i64` | `Vec<GasPrice>` |
| `register_gas_price_manual` | `price: i64` | `GasPrice` |
| `get_weekly_gas_comparison` | — | `Vec<WeeklyGasPoint>` |
| `get_route_costs` | — | `RoutesCost` |

`register_gas_price_manual` hace UPSERT por fecha (un precio por día).

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

`factory_reset` elimina todas las filas de: `transactions`, `goals`, `gas_prices`, `budgets`, `custom_routes`, `vehicles`. La tabla `config` no se toca.

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
