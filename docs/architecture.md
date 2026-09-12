# Arquitectura — FinCapX

Referencia técnica del proyecto para quien vaya a desarrollar sobre él: stack, estructura de carpetas, modelo de datos, comandos Tauri y convenciones de frontend. Para instalar o usar la app como usuario final, ver [`README.md`](../README.md). Para comandos de desarrollo día a día (`pnpm tauri dev`, `pnpm check`, etc.), ver [`CLAUDE.md`](../CLAUDE.md).

---

## 1. Stack

| Capa | Tecnología |
|------|-----------|
| Framework desktop | Tauri 2 |
| Backend | Rust (stable) |
| Frontend | Svelte 5 (runes API — `$state`, `$derived`, `$effect`) + SvelteKit (`adapter-static`, SPA sin SSR) |
| Estilos | CSS puro con variables (`src/app.css`) — sin frameworks UI |
| Base de datos | SQLite local vía `libsql` — sin servidor, sin sync cloud |
| Gestor de paquetes | pnpm |
| Empaquetado | `tauri build` → `.rpm` y `.deb` |

**Plataforma objetivo:** Linux. No se genera AppImage (requiere FUSE 2, no disponible por defecto en Fedora).

---

## 2. Estado actual del proyecto — leer esto antes de tocar nada

FinCapX pasó por un rediseño completo de modelo de datos (v1 → v2, detallado en la sección 5) para corregir bugs estructurales de v1: un ahorro no puede representarse sin que parezca un gasto, una compra a crédito se contaba dos veces, no existía un tipo "transferencia" que no afectara el patrimonio, etc.

**A la fecha de este documento (actualizado 2026-09-12):**

- El **frontend está escrito al 100% contra comandos v2** (`src/lib/api/*.ts` → `*_v2` en Rust, con la única excepción de `gas.ts`, que reutiliza los comandos v1 de precio de gasolina tal cual — esa parte del modelo no cambió con la migración).
- El **backend mantiene ambos módulos en paralelo**: `commands/services/repositories/{modulo}.rs` (v1, ya no llamado por el frontend, vivo solo para no romper compilación mientras se confirma la migración) y `{modulo}_v2.rs` (el que realmente se usa). Los dos siguen registrados en `invoke_handler!` (`src-tauri/src/lib.rs`).
- **El esquema v2 (tablas `accounts`, `entries`, `goals`, `loans`, `vehicles`, `fillups`, `trips`, `fuel_adjustments`, `budgets`, `budget_overrides`, `routes`, `categories`) sigue sin crearse automáticamente al iniciar la app.** `db::apply_schema()` (llamado en cada arranque desde `init_db()` en `lib.rs`) solo garantiza el esquema v1. Las tablas v2 solo existen si en algún momento se ejecutó `migrations::migrate_001_schema_v2` contra esa base de datos concreta — el binario normal nunca la invoca por sí solo; hoy solo corre en los tests de integración, en `src-tauri/examples/dry_run_migration.rs`, o manualmente contra una base real.
- Esto significa: **una base de datos completamente nueva (developer que clona el repo por primera vez, o cualquier instalación fresca de la app) no funciona tal cual** — el frontend pedirá comandos que consultan tablas inexistentes. Antes de poder usar la app end-to-end hace falta correr `migrate_001_schema_v2` una vez contra esa base (ver sección 6).
- La base de datos de **desarrollo** (`~/.local/share/finanzas-dev/local.db`, la que usa `pnpm tauri dev`) ya fue migrada manualmente durante el desarrollo de v2 y sigue así entre sesiones — por eso `pnpm tauri dev` funciona sin pasos extra en este equipo. Un clon nuevo del repo, o borrar ese archivo, requiere repetir la migración.
- La base de datos de **producción** (`~/.local/share/finanzas/local.db`, la que usa el binario instalado) **ya fue migrada a v2** — confirmado inspeccionando la base real: `PRAGMA user_version` = 1, existen las tablas v2 con datos (`entries`, `goals`, `loans`, `vehicles`, `budgets`, `fillups`, `fuel_adjustments`, etc.) junto con las tablas v1 que el migrador renombró con sufijo `_v1` (`transactions_v1`, `goals_v1`, `loans_v1`, `vehicles_v1`, `budgets_v1`, `custom_routes_v1`, `loan_payments_v1`, `fuel_fillups_v1`) — se conservan como respaldo, el migrador no las borra.
  - **Efecto secundario cosmético a tener en cuenta:** como `apply_schema()` (esquema v1) sigue corriendo en cada arranque sin importar si ya se migró, y algunas tablas v1 no tienen tabla v2 con el mismo nombre (`transactions`, `fuel_fillups`, `custom_routes` — v2 las reemplaza por `entries`, `fillups`, `routes` con nombres distintos), esas tres reaparecen vacías en cada arranque post-migración (`CREATE TABLE IF NOT EXISTS` las recrea porque el rename se las llevó). Son inofensivas — el código v2 nunca las lee ni las escribe — pero no hay que confundirlas con datos reales si se inspecciona la base a mano.

En resumen: el código está completo, probado, y **la base de datos de producción real de este usuario ya corre sobre v2** desde que se ejecutó la migración. Sigue siendo cierto que una base de datos nueva o sin migrar (un clon fresco del repo, o cualquier otra instalación) no funciona tal cual hasta correr `migrate_001_schema_v2` una vez — no asumir que "clonar y `pnpm tauri build`" alcanza sin ese paso en ese caso.

---

## 3. Arquitectura del backend — 3 capas estrictas

```
commands/   ← adaptador delgado de Tauri (#[tauri::command]), sin lógica de negocio
    ↓
services/   ← toda la lógica de negocio, validaciones, orquestación de transacciones
    ↓
repositories/ ← solo SQL, sin lógica de negocio
```

Cada módulo de dominio (`entries`, `goals_v2`, `vehicles_v2`, etc.) replica esta estructura en los tres directorios. `models/mod.rs` concentra todos los structs (`Deserialize` para inputs de comandos, `Serialize` para lo que vuelve al frontend). `utils.rs` tiene las conversiones de unidades puras (galones↔mililitros, km↔metros, resolución de `PeriodV2`) — sección 1 del diseño original: la conversión de unidades vive en un único módulo, nunca repetida en cada servicio.

`state.rs` maneja la conexión a SQLite (`DbState`, `get_conn`); `db.rs` abre la base de datos y aplica el esquema v1 en cada arranque; `migrations/mod.rs` tiene el esquema v2 completo y el migrador v1→v2 (nunca invocado automáticamente, ver sección 2).

---

## 4. Estructura de carpetas

```
src-tauri/src/
├── commands/        # un archivo por dominio; *_v2.rs son los que usa el frontend
├── services/        # lógica de negocio, misma convención *_v2.rs
├── repositories/     # SQL puro, misma convención
├── migrations/mod.rs # esquema v2 completo + migrador v1→v2
├── models/mod.rs     # structs compartidos (inputs/outputs de comandos)
├── db.rs             # apertura de la base + esquema v1 + migraciones aditivas de columnas
├── state.rs           # DbState, conexión compartida
├── utils.rs            # conversión de unidades, resolución de períodos
├── error.rs             # AppError / AppResult
└── lib.rs                # setup de Tauri, tray, autostart, invoke_handler!

src-tauri/tests/         # tests de integración por dominio v2 (cargo test)
src-tauri/examples/       # dry_run_migration.rs — corre migrate_001 contra una copia

src/
├── app.css              # tokens del sistema de diseño (sección 7)
├── routes/
│   ├── +layout.svelte    # sidebar, navegación (Resumen/Registros/Historial/Metas/Ajustes), widget flotante de saldo, disparador del onboarding
│   ├── +page.svelte       # Resumen (dashboard)
│   ├── registrar/+page.svelte  # "Registros" en el nav — el nombre de ruta/archivo no cambió
│   ├── historial/+page.svelte
│   ├── metas/+page.svelte
│   └── config/+page.svelte     # "Ajustes" en el nav — pestañas: Presupuestos, Vehículos y gasolina, Sistema, Datos
└── lib/
    ├── types.ts           # interfaces TypeScript espejo de los structs Rust
    ├── constants.ts        # meses, tamaños de página, conversiones de unidades
    ├── txState.svelte.ts   # señal reactiva compartida (versión de transacciones, para refrescar entre pantallas)
    ├── registrarDraft.svelte.ts  # borrador del formulario de Registros — vive fuera del componente para no perderse al cambiar de pantalla
    ├── tour.svelte.ts       # estado y pasos del tour de onboarding (ver sección 7)
    ├── api/                # una función por comando Tauri, agrupada por dominio — las páginas nunca llaman invoke() directo
    └── components/         # CustomSelect, DatePicker, PaymentModal, ScrollArea, TourPoint, Onboarding
```

---

## 5. Modelo de datos (v2)

Los montos son `INTEGER` en COP (sin decimales). Las fechas son `TEXT` en `YYYY-MM-DD`. Los ids son ULID (`TEXT`). Borrado lógico vía `deleted_at` en las tablas que lo necesitan. Fuente de verdad exacta: `src-tauri/src/migrations/mod.rs::SCHEMA_V2`.

| Tabla | Propósito | Notas clave |
|---|---|---|
| `accounts` | Bolsas de dinero conceptuales | `cash` (disponible), `savings`, `receivable`, `payable`, `apps` (plata en Didi/Uber sin retirar). Sin multi-cuenta bancaria real todavía. |
| `categories` | Categorías de ingreso/gasto | `is_fixed` (para ingresos), `route_id` opcional (asocia una categoría de gasto a una ruta de kilometraje), `is_system` excluye categorías internas de los reportes. |
| `entries` | **Todo movimiento de dinero** | Reemplaza la vieja tabla `transactions`. `kind` = `income` / `expense` / `transfer`. Un `CHECK` compuesto obliga la combinación correcta de `account_from`/`account_to`/`category_id` según el tipo — a nivel de base de datos, no solo de aplicación. |
| `goals` | Ahorros y deudas | `kind` = `saving` / `debt`. No contienen dinero — el dinero vive en `entries`; `goals` solo describe el objetivo. |
| `loans` | Préstamos hechos a otras personas | Igual que `goals`: solo describe, el movimiento real está en `entries` (`transfer cash→receivable` al prestar). |
| `budgets` / `budget_overrides` | Presupuesto mensual por categoría | `budget_overrides` permite un monto distinto para un mes puntual sin cambiar el presupuesto base. |
| `routes` | Rutas de kilometraje frecuente | Usadas por `categories.route_id` y por el cálculo de costo por ruta en Ajustes → Vehículos y gasolina. |
| `vehicles` | Vehículos registrados | `efficiency_m_per_l` (rendimiento) y `tank_capacity_ml` — ambos obligatorios al crear/editar desde la UI (la capacidad del tanque no es solo cosmética: permite detectar un rendimiento mal configurado, ver `services::fuel::overflow_warning`). |
| `fillups` | Tanqueos reales (con recibo) | Puede enlazar a un `entries` (el gasto real pagado) vía `entry_id`. |
| `trips` | Viajes recorridos | Solo consumo de tanque (metros → mililitros vía rendimiento del vehículo), sin gasto asociado — separar "cuánto gasté" de "cuánto combustible consumí" fue uno de los motivos centrales de la migración. |
| `fuel_adjustments` | Anclas de nivel de tanque | Permite resetear el nivel calculado sin borrar `fillups`/`trips` (`vehicle_reset_fuel_level`). `raw_level_ml` suma tanqueos/viajes con `occurred_on >= fecha_del_ancla` (no `>`): como las fechas no tienen hora, un tanqueo del mismo día del reset debe contar. |
| `gas_prices` | Precio histórico del galón | Única tabla que **no cambió** con la migración — se reutiliza tal cual de v1. |

**Reglas de negocio no obvias, ya resueltas en el esquema/servicios:**
- Ahorrar plata, prestarla, cobrar una deuda o abonarla nunca deberían aparecer como ingreso/gasto normal — se resuelven como `kind = transfer` entre cuentas conceptuales, y por eso nunca alteran el patrimonio total (solo mueven de una bolsa a otra), excepto abonar una deuda propia, que sí sube el patrimonio (se paga un pasivo real).
- Una compra a crédito (`goal_create_debt_v2` → `create_debt`) es un **gasto real desde el día uno** (`expense` con `account_from = payable`), no algo que se materializa solo si falta saldo.
- `is_extraordinary` en `entries` marca eventos no recurrentes: se excluyen de la comparación contra presupuesto, pero se incluyen (desglosados) en los totales reales del período.
- Un vehículo es opcional: sin ninguno registrado, el frontend oculta Tanqueo y Kilometraje en Registros (no tiene sentido pedir kilometraje a quien no tiene vehículo) — el backend no necesita saberlo, es una decisión puramente de UI (`vehicles.length === 0` en `registrar/+page.svelte`).
- `services::system_v2::factory_reset` borra **todo** dato de usuario, incluyendo `gas_prices` — solo se preservan `accounts` (filas de sistema fijas) y `categories` con `is_system = 1`. Antes excluía `gas_prices` "por ser historial de referencia"; corregido porque "restablecer de fábrica" debe dejar la app exactamente como una instalación nueva.

---

## 6. Comandos Tauri

Todos registrados en `src-tauri/src/lib.rs::invoke_handler!`. Los que usa el frontend activo son los sufijados `_v2` (más `commands::gas::*`, compartido). Los comandos sin sufijo (`create_transaction`, `loan_create`, `metas_list`, etc.) son el módulo v1: siguen registrados y compilando, pero ninguna pantalla los invoca — quedan pendientes de borrar tras confirmar un mes de uso real en v2 (ver sección 2).

| Dominio | Comandos v2 principales | Módulo `src/lib/api/` |
|---|---|---|
| Movimientos | `entry_create/list/get/update/delete/delete_bulk`, `get_account_balances`, `get_period_summary_v2`, `get_category_progress_v2`, `get_month_comparison_v2`, `entry_export_csv` | `entries.ts` |
| Cuentas | `account_list` | `accounts.ts` |
| Categorías | `category_create/list/update/delete` | `categories.ts` |
| Presupuestos | `budget_list_with_categories`, `budget_set_monthly`, `budget_set_override` | `budgets.ts` |
| Metas unificadas | `metas_list_v2` (solo lectura, combina goals+loans), `meta_add_payment` | `metas.ts`, `metaPayments.ts` |
| Objetivos de ahorro/deuda | `goal_create_v2/update/delete/get_detail_v2`, `goal_create_debt_v2` | `goals.ts` |
| Préstamos | `loan_create_v2/list/get/update/delete_v2`, `loans_total_pending_v2` | `loans.ts` |
| Vehículos | `vehicle_list/create/update/delete_v2` | `vehicles.ts` |
| Combustible | `fillup_create_v2`, `fillup_create_with_expense_v2`, `fillups_list_v2`, `vehicle_fuel_status_v2`, `trip_register_v2`, `vehicle_reset_fuel_level` | `fillups.ts` |
| Precio de gasolina (v1, sin cambios) | `get_current_gas_price`, `list_gas_prices`, `register_gas_price_manual`, `get_weekly_gas_comparison`, `get_route_costs` | `gas.ts` |
| Rutas | `route_list/save/delete_v2` | `routes.ts` |
| Sistema | `get_autostart_enabled`, `set_autostart_enabled`, `backup_database`, `factory_reset_v2` | `system.ts` |

Las páginas nunca llaman `invoke()` directamente — siempre a través de estos wrappers (`src/lib/api/index.ts` re-exporta todo como `entryApi`, `goalApi`, etc.). Los errores del backend (`AppResult<T>`) se deserializan automáticamente y llegan como excepción JS al `catch` del `invoke()`.

---

## 7. Frontend

- **Runas de Svelte 5** en todo el código, no la store API legacy.
- Cada página (`routes/*/+page.svelte`) trae su propio `<script>` con estado, efectos y helpers — los módulos de estado compartido fuera de un componente son la excepción, no la regla: `txState.svelte.ts` (contador de versión que las páginas observan para refrescarse cuando otra pantalla crea/edita/borra algo) y `registrarDraft.svelte.ts` (el borrador del formulario de Registros — vive en un módulo aparte para sobrevivir a que el usuario navegue a otra sección y vuelva; un `$state` local normal se habría reiniciado solo al desmontarse el componente).
- `types.ts` es el espejo manual de los structs `Deserialize`/`Serialize` de Rust — si se cambia un campo en `models/mod.rs`, hay que reflejarlo aquí a mano (no hay generación automática de tipos).
- Componentes compartidos: `CustomSelect` (select propio, con menú `position:fixed` calculado en JS para escapar de contenedores con `overflow`), `DatePicker` (input de texto enmascarado `DD/MM/AAAA`, sin popup nativo, precarga la fecha de hoy sin exigir que el usuario la reescriba), `PaymentModal` (modal genérico de detalle + registrar abono, usado por Metas), `ScrollArea` (wrapper de scroll con scrollbar delgada, usado en vez de `overflow` directo en cualquier panel que necesite recortar contenido).

### Tour de onboarding

Dispara por versión instalada (`localStorage["onboarding_seen_version"]` vs. `getVersion()` de Tauri), nunca por si la base de datos está vacía — así "Restablecer datos de fábrica" no lo vuelve a mostrar a un usuario que ya conoce la app. Estado y definición de pasos en `tour.svelte.ts` (`TOUR_STEPS`: presupuestos → registros → historial → metas → resumen, cada uno con `pointCount` puntos que se muestran de a uno, nunca todos juntos de una sección).

`TourPoint.svelte` es el mensaje flotante en sí: se ancla al elemento DOM padre donde se renderiza (`rootEl.parentElement`), mide su posición con `getBoundingClientRect()` y se dibuja con `position:fixed` — el mismo patrón que el menú de `CustomSelect` — para escapar de cualquier `ScrollArea`/`overflow:hidden` y quedar siempre por encima de todo. Se voltea automáticamente arriba/abajo si no cabe, se clampea para no salirse de la ventana, y nunca se superpone con la barra de navegación del propio tour (detectada vía `[data-tour-navbar]` en `Onboarding.svelte`). Acepta `side="right"` para anclarse al lado de un bloque ancho (ej. el `<form>` de Registros) en vez de arriba/abajo.

`Onboarding.svelte` es el modal de confirmación inicial + la barra inferior de navegación del tour (Omitir / Atrás / Siguiente) — nunca bloquea el avance ni exige crear datos, es un recorrido informativo puro.

---

## 8. Sistema de diseño — "Terminal Ligero"

Tema oscuro fijo, sin modo claro. Tokens en `src/app.css`:

- `--bg-base` / `--bg-surface` / `--bg-elevated` — tres niveles de fondo, sin sombras para dar profundidad.
- `--border` — toda separación de secciones es `border`/`border-bottom`, nunca `box-shadow`.
- `--accent` (ámbar) — único acento decorativo de toda la app.
- `--success` / `--danger` — solo para semántica financiera real (ingreso/gasto, saldo positivo/negativo), nunca decorativos.
- `--font` (Inter) / `--font-mono` (JetBrains Mono), autohospedadas vía `@fontsource/*` — la app debe funcionar sin internet.
- `--radius: 0` — toda la app hereda de esta variable; nunca hardcodear un radio en un componente nuevo.

Convenciones repetidas en toda la app: mono uppercase + letter-spacing en botones y badges; mono para montos/fechas/ids; sans para texto libre; paneles planos con borde en vez de contenido flotando sin límites; filas de listas con acciones (editar/borrar) ocultas hasta hacer hover, no botones cuadrados siempre visibles; ningún `<table>` HTML — listas de filas flexbox en su lugar.

---

## 9. Base de datos

SQLite local vía `libsql`, sin servidor ni sincronización cloud.

| Modo | Ruta |
|---|---|
| Desarrollo (`pnpm tauri dev`) | `~/.local/share/finanzas-dev/local.db` |
| Producción (build instalado) | `~/.local/share/finanzas/local.db` |

Backup manual desde **Ajustes → Sistema**. Ambas bases de este equipo (desarrollo y producción) ya están migradas a v2 — ver sección 2 para el detalle y para las tablas v1 vacías que reaparecen como efecto secundario cosmético. Una base de datos nueva o de otra instalación sigue necesitando la migración manual.
