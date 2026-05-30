# Contexto: Módulo Metas — Cierre de Gaps (Pre-Fase 4B)

## Estado al corte de sesión

Se completaron **Fases 1–3** del módulo Metas y se realizó la **auditoría de Fase 4**.  
La próxima tarea es **cerrar los gaps de paridad** identificados para luego proceder a **Fase 4B** (eliminar /objetivos y /prestamos del nav).

---

## Lo que ya está hecho

### Backend (Rust)
- `src-tauri/src/services/metas.rs` — servicio normalizador que une loans y goals en `Vec<Meta>`
- `src-tauri/src/models/mod.rs` — struct `Meta` y `MetaAbono` ya definidos
- El comando `metas_list` ya está registrado y funciona

### Frontend
- `src/lib/types.ts` — interfaces `Meta` y `MetaAbono` añadidas al final
- `src/lib/api/metas.ts` — módulo API (solo `list`)
- `src/lib/api/index.ts` — re-exporta `metaApi`
- `src/routes/+layout.svelte` — nav incluye `/metas`
- `src/routes/metas/+page.svelte` — **página completa con**:
  - Listado unificado (loans + goals) como `Meta[]`
  - Filtros duales: por tipo (Todas/Préstamos/Deudas/Ahorros) + estado (Todos/Pendientes/Completadas)
  - Sub-secciones PENDIENTES: Deudas → Préstamos → Ahorros
  - Sección COMPLETADAS al fondo con badge de tipo
  - Colores pastel por tipo: `#8e8abd` (lavanda, me_deben), `#a99060` (ámbar, debo), `#5fa386` (salvia, quiero_juntar)
  - Modal "+ Nueva Meta": crea Préstamos via `loanApi.create`, Ahorros via `goalApi.create`, Deudas muestra mensaje informativo
  - PaymentModal al click en card: abono routed por prefix `loan:` o `goal:`

---

## Gaps a cerrar (tarea interrumpida)

Al inicio de la sesión interrumpida ya se habían leído los archivos necesarios:
- `src-tauri/src/services/metas.rs` ✅ leído
- `src-tauri/src/services/goals.rs` ✅ leído — tiene `build_progress` que calcula `on_track`, `monthly_required`, `projected_completion_date`
- `src-tauri/src/models/mod.rs` ✅ leído — struct `Meta` actual NO incluye los campos de progreso
- `src/lib/api/goals.ts` ✅ leído — expone `update(id, GoalInput)` y `remove(id)`
- `src/routes/objetivos/+page.svelte` ✅ leído — referencia de paridad
- `src/routes/prestamos/+page.svelte` ✅ leído — referencia de paridad

### Gap 1: Editar ahorro (`quiero_juntar`)
- Solo para tipo `quiero_juntar` (prefix `goal:`)
- Botón editar en tarjeta (solo si `m.tipo === "quiero_juntar"`)
- Modal con: nombre, monto, fecha meta opcional, status (activo/pausado/completado)
- Llama `goalApi.update(numId, { name, target_amount, target_date, status })`
- Tras guardar: `await loadMetas()` y cerrar modal
- Loans NO se editan — no exponer botón si tipo es `me_deben` o `debo`

### Gap 2: Eliminar (préstamo y ahorro)
- Botón eliminar en tarjetas (todos los tipos, pendientes y completadas)
- Modal de confirmación pequeño (patrón igual a /prestamos y /objetivos)
- Si prefix `loan:` → `loanApi.remove(numId)`
- Si prefix `goal:` → `goalApi.remove(numId)`
- Tras eliminar: `await loadMetas()` y limpiar `detail` si era la meta eliminada

### Gap 3: Metadatos de ahorros (on_track, monthly_required, projected_completion_date)

**Estrategia decidida: opción (a) — extender struct `Meta` en el backend**

#### Cambios en Rust necesarios:

**`src-tauri/src/models/mod.rs`** — agregar campos opcionales al struct `Meta`:
```rust
pub on_track: Option<bool>,
pub monthly_required: Option<f64>,
pub projected_completion_date: Option<String>,
```

**`src-tauri/src/services/metas.rs`** — llenar los campos para goals de tipo `quiero_juntar`:
```rust
// Para el bloque de goals, ya se tiene `progress: GoalWithProgress`
// agregar al Meta::push:
on_track: if progress.goal.is_debt_goal { None } else { Some(progress.on_track) },
monthly_required: if progress.goal.is_debt_goal { None } else { progress.monthly_required },
projected_completion_date: if progress.goal.is_debt_goal { None } else { progress.projected_completion_date },
// Para loans: todos None
```

**`src/lib/types.ts`** — agregar al interface `Meta`:
```typescript
on_track: boolean | null;
monthly_required: number | null;
projected_completion_date: string | null;
```

#### Cambios en la UI (`src/routes/metas/+page.svelte`):
- En card de ahorro pendiente con `on_track === false`: badge "Atrasado" en ámbar (igual que `/objetivos`: `color: #f59e0b`)
- Si `monthly_required !== null`: mostrar `≈ {formatCOP(monthly_required)}/mes` (similar a `.cuotas-hint`)
- Si `!on_track && projected_completion_date`: mostrar `Estimado: {projected_completion_date}` discreto

**NO hacer**: barra de color dinámica, showCategory/showNote en PaymentModal para ahorros.

---

## Archivos a modificar (en orden)

1. `src-tauri/src/models/mod.rs` — añadir 3 campos Optional a struct `Meta`
2. `src-tauri/src/services/metas.rs` — llenar esos campos en el loop de goals
3. `src/lib/types.ts` — añadir 3 campos al interface `Meta`
4. `src/routes/metas/+page.svelte` — editar + eliminar + badges de metadatos

---

## Estructura clave de Meta.id

```
"loan:{id}"  → loanApi.remove(id)  [NO editable]
"goal:{id}"  → goalApi.remove(id) o goalApi.update(id) si tipo === "quiero_juntar"
```

Parseo siempre: `const [prefix, rawId] = meta.id.split(":")` → `parseInt(rawId, 10)`

---

## Reglas de estilo del proyecto

- Paleta: base `#0c0c14`, accent `#7c6bff`, success `var(--success)`, danger `var(--danger)`
- "Atrasado" badge: `color: #f59e0b` (ámbar, igual que `/objetivos`)
- Modal overlay: `position:fixed; display:flex; align-items:center; justify-content:center` — NO usar `transform:translate` en el modal hijo (rompe CustomSelect)
- PaymentModal compartido: `src/lib/components/PaymentModal.svelte`
- ScrollArea compartido: `src/lib/components/ScrollArea.svelte`
- CustomSelect compartido: `src/lib/components/CustomSelect.svelte`
- DatePicker compartido: `src/lib/components/DatePicker.svelte`

---

## Tabla de paridad actualizada (post-gaps)

| Feature | /objetivos | /prestamos | /metas actual | /metas post-gaps |
|---|---|---|---|---|
| Listado unificado | — | — | ✅ | ✅ |
| Filtros tipo + estado | — | — | ✅ | ✅ |
| Crear Préstamo | — | ✅ | ✅ | ✅ |
| Crear Ahorro | ✅ | — | ✅ | ✅ |
| Editar Ahorro | ✅ | — | ❌ | ✅ |
| Eliminar | ✅ | ✅ | ❌ | ✅ |
| on_track badge | ✅ | — | ❌ | ✅ |
| monthly_required | ✅ | — | ❌ | ✅ |
| projected_completion_date | ✅ | — | ❌ | ✅ |
| cuotas hint | ✅ | — | ✅ | ✅ |
| PaymentModal abono | ✅ | ✅ | ✅ | ✅ |
| showCategory+showNote | ✅ | — | (pendiente deliberado) | (pendiente deliberado) |

---

## Fase 4B (PENDIENTE — no iniciar sin completar gaps)

Una vez cerrados todos los gaps:
1. Quitar `{ href: '/objetivos', label: 'Objetivos' }` del array `navItems` en `src/routes/+layout.svelte`
2. Quitar `{ href: '/prestamos', label: 'Préstamos' }` del mismo array
3. Revisar `src/routes/config/+page.svelte` por referencias de texto a esas rutas
4. Las páginas físicas pueden quedar — solo se retiran del nav

---

## Comando de verificación

```bash
pnpm check   # 0 errores antes de cualquier commit
```

Backend requiere reinicio de `pnpm tauri dev` tras cambios en Rust.
