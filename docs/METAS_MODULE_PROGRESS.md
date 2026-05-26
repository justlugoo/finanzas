# Módulo Metas — Estado del desarrollo

> Documento de continuidad. Última actualización: 2026-05-26.
> Escrito para que el próximo Claude Code pueda retomar sin fricción.

---

## Contexto general del proyecto

App de finanzas personales para Linux: **Tauri 2 + Rust (backend) + Svelte 5 (frontend)**.
SQLite en `~/.local/share/finanzas/local.db`. Amounts en INTEGER COP, fechas en `YYYY-MM-DD`.

Arquitectura Rust estricta en 3 capas:
```
commands/ → services/ → repositories/
```

---

## Estado del working tree (branch: main)

**Hay cambios sin commitear.** Son válidos y compilando — el usuario los está validando en la app antes de commitear. **NO los descartes.**

Archivos modificados sin commitear al inicio de esta sesión (y sus cambios):
- `src-tauri/src/db.rs` — tablas/migraciones de Fase 1 gas + columna `installments` en goals
- `src-tauri/src/models/mod.rs` — campos gas, FuelFillup structs (placeholders), `installments` en Goal, **y ahora también `Meta`/`MetaAbono` (añadidos en esta sesión)**
- `src-tauri/src/repositories/gas.rs` — `find_price_for_date`, `get_vehicle_km_per_gallon` (placeholders Fase 2)
- `src-tauri/src/repositories/transactions.rs` — columnas gas_km/trip_vehicle_id en queries, fix de deuda (filtro `AND NOT (is_debt = 1 AND type = 'gasto')`)
- `src-tauri/src/services/gas.rs` — eliminado `insert_auto`
- `src-tauri/src/services/transactions.rs` — eliminado flujo `auto_gas`, ahora llama `insert_debt_goal` con `installments`

Archivos **nuevos** sin commitear (creados en esta sesión):
- `src-tauri/src/services/metas.rs` ← **Módulo Metas Fase 1**
- `src-tauri/src/commands/metas.rs` ← **Módulo Metas Fase 1**

Archivos modificados en esta sesión (también sin commitear):
- `src-tauri/src/services/mod.rs` — añadido `pub mod metas;`
- `src-tauri/src/commands/mod.rs` — añadido `pub mod metas;`
- `src-tauri/src/lib.rs` — añadido `commands::metas::metas_list` al invoke_handler
- `src/lib/types.ts` — `Goal.installments`, `TransactionInput.installments`
- `src/lib/api/goals.ts` — función `addContribution`
- `src/lib/components/PaymentModal.svelte` — componente nuevo (compartido Objetivos/Préstamos)
- `src/routes/prestamos/+page.svelte` — usa PaymentModal
- `src/routes/objetivos/+page.svelte` — usa PaymentModal + hint de cuotas
- `src/routes/registrar/+page.svelte` — campo cuotas en formulario de deuda

---

## Lo que se hizo en esta sesión (resumen por tarea)

### T1 — Fix deuda 100% (bug pre-existente desde v1.1.0)
En `repositories/transactions.rs`, tres funciones recibieron el filtro:
```sql
AND NOT (is_debt = 1 AND type = 'gasto')
```
Funciones afectadas: `sum_by_goal`, `sum_by_goal_recent`, `sum_for_goal_completion`.

Esto evita que el gasto origen de la deuda cuente como progreso.

### T2 — PaymentModal.svelte (componente compartido)
`src/lib/components/PaymentModal.svelte` — componente genérico que recibe props tipadas:
```typescript
{ title, subtitle?, subtitleClass?, note?, stats: StatEntry[], paid, total,
  progressDone?, items: PaymentItem[], itemsLabel?, showCategory?, showNote?,
  canPay?, onAddPayment: (amount, date) => Promise<void>, onClose }
```
Usado por Préstamos y Objetivos. Contiene: stats grid, barra de progreso, tabla de abonos, formulario de pago.

### T3 — Cuotas informativas (installments)
- Migración aditiva en `goals` table: `ALTER TABLE goals ADD COLUMN installments INTEGER DEFAULT NULL`
- `insert_debt_goal` acepta `Option<i64> installments`
- Campo opcional en formulario de registrar (solo para gastos-deuda)
- Hint visual en tarjetas de Objetivos: `≈ COP X/mes · N cuotas`

### T4 — Módulo Metas Fase 1 (capa normalizadora backend)
**Completado en esta sesión. cargo check: OK (solo warnings pre-existentes de Fase 2).**

Archivos creados/modificados:

**`src-tauri/src/models/mod.rs`** — añadidos al final:
```rust
pub struct MetaAbono { pub id: i64, pub date: String, pub amount: i64 }

pub struct Meta {
    pub id: String,        // "loan:{id}" | "goal:{id}"
    pub tipo: String,      // "me_deben" | "debo" | "quiero_juntar"
    pub nombre: String,
    pub total: i64,
    pub abonado: i64,
    pub pendiente: i64,
    pub estado: String,    // "pendiente" | "completado"
    pub fecha: Option<String>,
    pub nota: Option<String>,
    pub cuotas: Option<i64>,
    pub abonos: Vec<MetaAbono>,
}
```

**`src-tauri/src/services/metas.rs`** — lógica de normalización:
- Loans → `tipo = "me_deben"`, `estado = loan.status == "pagado" ? "completado" : "pendiente"`
- Goals con `is_debt_goal=1` → `tipo = "debo"`
- Goals con `is_debt_goal=0` → `tipo = "quiero_juntar"`
- Abonos de loans: directo de `lb.payments`
- Abonos de goals: `tx_repo::list_by_goal` filtrado con `!(tx.is_debt && tx.kind == "gasto")`
- Reutiliza `services::loans::list`, `repositories::goals::list`, `services::goals::build_progress`, `repositories::transactions::list_by_goal`
- **No toca** el backend de goals ni loans existente

**`src-tauri/src/commands/metas.rs`**:
```rust
#[tauri::command]
pub async fn metas_list(state: State<'_, DbState>) -> AppResult<Vec<Meta>> {
    let conn = get_conn(&state).await?;
    svc::list(&conn).await
}
```

---

## Próximas fases del Módulo Metas (pendientes de implementar)

### Fase 2 — Frontend: página `/metas`
El usuario NO ha especificado el diseño exacto todavía. Lo que se sabe:
- La página unifica Préstamos + Objetivos en una vista
- Llamada: `invoke<Meta[]>("metas_list")`
- Filtros por `tipo`: "me_deben" / "debo" / "quiero_juntar"
- PaymentModal.svelte ya está disponible para reutilizar

**Importante:** Confirmar con el usuario el diseño antes de implementar. La instrucción anterior fue "NO toques la UI todavía."

### Fase 3 (posible) — Acciones desde Metas
- Registrar abono a préstamo → `invoke("loan_add_payment", ...)`
- Registrar contribución a objetivo → `invoke("create_transaction", { goal_id, type: "ingreso", category: "Abono", ... })`
- El backend ya expone todo lo necesario

---

## Instrucciones del usuario vigentes

- **NO commitear sin permiso explícito** — siempre revisar primero, el usuario pide el commit
- **NO tocar** los backends existentes de goals ni loans — la capa metas va POR ENCIMA
- **NO hacer push** — el usuario no ha pedido push en ningún momento de la sesión
- `cargo check` debe pasar antes de cualquier commit

---

## Cómo verificar el estado actual

```bash
# Verificar que compila
cd src-tauri && cargo check

# Ver archivos modificados
git status

# Ver diff completo
git diff

# Validar frontend
cd .. && pnpm check
```

---

## Commit pendiente sugerido (cuando el usuario lo apruebe)

El usuario mencionó querer commits lógicos separados. Una propuesta:

```
feat: módulo Metas Fase 1 — capa normalizadora backend (metas_list)
feat: PaymentModal compartido, abonos en Objetivos y cuotas informativas en deudas
fix: progreso de deuda no cuenta el gasto origen (sum_by_goal filter)
```

Pero **esperar confirmación explícita** antes de hacer cualquier commit.
