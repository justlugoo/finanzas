<script lang="ts">
  import { entryApi, vehicleApi, fillupApi, categoryApi, metaApi } from "$lib/api";
  import type { AccountBalances, PeriodSummaryV2, CategoryProgressV2, MonthComparisonV2, PeriodV2, MetaV2 } from "$lib/types";
  import { txState } from "$lib/txState.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";
  import TourPoint from "$lib/components/TourPoint.svelte";
  import { MESES, MESES_CORTO, DASHBOARD_RECENT_SIZE, mlToGallons, metersToKm } from "$lib/constants";
  import { isActivePoint } from "$lib/tour.svelte";

  type PeriodKey = "Day" | "Week" | "Month" | "Year" | "All";

  const PERIOD_LABELS: Record<PeriodKey, string> = {
    All: "Total", Year: "Anual", Month: "Mensual", Week: "Semanal", Day: "Diario",
  };

  function toISODate(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  // Lunes de la semana de `d` — semana de calendario, no "los últimos 7 días"
  // (mismo criterio que Historial: si hoy es miércoles, arranca el lunes de
  // ESTA semana, no hace 7 días).
  function mondayOf(d: Date): Date {
    const day = d.getDay(); // 0 = domingo … 6 = sábado
    const diffToMonday = day === 0 ? -6 : 1 - day;
    const monday = new Date(d);
    monday.setDate(d.getDate() + diffToMonday);
    return monday;
  }

  function periodValue(key: PeriodKey): PeriodV2 {
    const now = new Date();
    const end = toISODate(now);
    switch (key) {
      case "Day":   return { type: "Custom", value: { start: end, end } };
      case "Week":  return { type: "Custom", value: { start: toISODate(mondayOf(now)), end } };
      case "Month": return { type: "Month", value: { year: now.getFullYear(), month: now.getMonth() + 1 } };
      case "Year":  return { type: "Year", value: { year: now.getFullYear() } };
      case "All":   return { type: "Custom", value: { start: "1970-01-01", end } };
    }
  }

  let activePeriod = $state<PeriodKey>("Month");
  let globalBal    = $state<AccountBalances | null>(null);
  let summary      = $state<PeriodSummaryV2 | null>(null);
  let categories   = $state<CategoryProgressV2[]>([]);
  let recent       = $state<{ id: string; date: string; type: string; category: string; amount: number }[]>([]);
  let comparison   = $state<MonthComparisonV2 | null>(null);
  let loading      = $state(true);
  let error        = $state<string | null>(null);
  let budgetView   = $state<"ingresos" | "gastos">("ingresos");

  function daysInMonth(year: number, month: number): number {
    return new Date(year, month, 0).getDate();
  }

  // El presupuesto guardado siempre es mensual — para Diario/Semanal/Anual
  // se reescala a la porción correspondiente del mes actual. "Total" queda
  // afuera a propósito: proyectar una meta mensual a "toda la vida de uso de
  // la app" no tiene sentido (el usuario no espera acumular ese dinero en
  // años), así que se muestra tal cual, sin multiplicar.
  function budgetScaleFactor(period: PeriodKey): number {
    const now = new Date();
    const dim = daysInMonth(now.getFullYear(), now.getMonth() + 1);
    switch (period) {
      case "Day":   return 1 / dim;
      case "Week":  return 7 / dim;
      case "Month": return 1;
      case "Year":  return 12;
      case "All":   return 1;
    }
  }

  let scaledCategories = $derived.by(() => {
    const factor = budgetScaleFactor(activePeriod);
    return categories.map(c => {
      const target = Math.round(c.monthly_target * factor);
      return {
        ...c,
        monthly_target: target,
        percentage: target > 0 ? (c.current_amount / target) * 100 : 0,
        is_over: target > 0 && c.current_amount > target,
      };
    });
  });

  let incomeFixed    = $derived(scaledCategories.filter(c => c.type === "income" && c.is_fixed));
  let incomeVariable = $derived(scaledCategories.filter(c => c.type === "income" && !c.is_fixed));
  let expenseTracked = $derived(scaledCategories.filter(c => c.type === "expense"));

  let incomeTotals = $derived.by(() => {
    const all = [...incomeFixed, ...incomeVariable];
    const target  = all.reduce((s, c) => s + c.monthly_target, 0);
    const current = all.reduce((s, c) => s + c.current_amount, 0);
    const pct     = target > 0 ? (current / target) * 100 : 0;
    return { target, current, pct };
  });

  let expenseTotals = $derived.by(() => {
    const target  = expenseTracked.reduce((s, c) => s + c.monthly_target, 0);
    const current = expenseTracked.reduce((s, c) => s + c.current_amount, 0);
    const pct     = target > 0 ? (current / target) * 100 : 0;
    return { target, current, pct };
  });

  const prevMonthName = (() => {
    const now = new Date();
    const m = now.getMonth() === 0 ? 11 : now.getMonth() - 1;
    return MESES[m];
  })();

  $effect(() => {
    const period = activePeriod;
    const _v = txState.version;
    let cancelled = false;

    async function load() {
      loading = true;
      error = null;

      while (!cancelled) {
        try {
          const p = periodValue(period);
          const [sum, cats, page, cmp, bal, cats2] = await Promise.all([
            entryApi.getPeriodSummary(p),
            entryApi.getCategoryProgress(p),
            entryApi.list({ page_size: DASHBOARD_RECENT_SIZE }),
            entryApi.getMonthComparison(),
            entryApi.getAccountBalances(),
            categoryApi.list(undefined, true), // incluir archivadas: resuelve nombre en "últimas transacciones"
          ]);
          if (!cancelled) {
            summary    = sum;
            categories = cats;
            const catMap = new Map(cats2.map(c => [c.id, c.name]));
            recent = page.entries.map(e => ({
              id: e.id,
              date: e.occurred_on,
              type: e.type,
              category: e.type === "transfer" ? "Transferencia" : (e.category_id && catMap.get(e.category_id)) ?? "Sin categoría",
              amount: e.amount_cop,
            }));
            comparison = cmp;
            globalBal  = bal;
            loading    = false;
          }
          return;
        } catch (e: unknown) {
          const err = e as { kind?: string; message?: string };
          if (err?.kind === "DatabaseError" && err?.message?.includes("no inicializada")) {
            await new Promise((r) => setTimeout(r, 300));
          } else {
            if (!cancelled) { console.error("[dashboard] load error:", e); error = "Error cargando datos. Recarga la app."; loading = false; }
            return;
          }
        }
      }
    }

    load();
    return () => { cancelled = true; };
  });

  // ── Estado del tanque ─────────────────────────────────────────────────────
  let fuelStatuses = $state<{ vehicle_name: string; level_gallons: number; autonomy_km: number; tank_percentage: number | null; overCapacity: boolean }[]>([]);
  let fuelLoading  = $state(true);
  let hasVehiclesWithoutTank = $state(false);

  $effect(() => {
    const _v = txState.version;
    let cancelled = false;

    async function loadFuel() {
      fuelLoading = true;
      try {
        const vehicles = await vehicleApi.list();
        const withTank = vehicles.filter(v => v.tank_capacity_ml != null);
        hasVehiclesWithoutTank = vehicles.length > 0 && withTank.length === 0;
        if (withTank.length === 0) { fuelStatuses = []; return; }
        const levels = await Promise.all(withTank.map(v => fillupApi.vehicleFuelStatus(v.id)));
        if (!cancelled) {
          fuelStatuses = withTank.map((v, i) => ({
            vehicle_name: v.name,
            level_gallons: mlToGallons(levels[i].level_ml),
            autonomy_km: metersToKm(levels[i].autonomy_m),
            tank_percentage: levels[i].tank_percentage,
            // `raw_level_ml` > `level_ml` (recortado a la capacidad) solo pasa
            // cuando el nivel calculado se desbordó — señal persistente de que
            // el rendimiento (km/gal) del vehículo probablemente está mal, no
            // solo un aviso que se ve una vez al guardar el tanqueo.
            overCapacity: levels[i].raw_level_ml > levels[i].level_ml,
          }));
        }
      } catch (e) {
        console.error("[dashboard] fuel status error:", e);
        if (!cancelled) fuelStatuses = [];
      } finally {
        if (!cancelled) fuelLoading = false;
      }
    }

    loadFuel();
    return () => { cancelled = true; };
  });

  // ── Objetivos (préstamos/deudas/ahorros pendientes) — sección al lado de
  // Tanque, con la misma información que se ve en Metas ────────────────────
  let metas        = $state<MetaV2[]>([]);
  let metasLoading = $state(true);

  $effect(() => {
    const _v = txState.version;
    let cancelled = false;

    metaApi.list().then(ms => {
      if (!cancelled) { metas = ms; metasLoading = false; }
    }).catch(e => {
      console.error("[dashboard] metas error:", e);
      if (!cancelled) metasLoading = false;
    });

    return () => { cancelled = true; };
  });

  let pendingMetas = $derived(metas.filter(m => m.estado === "pendiente").slice(0, 3));

  function metaPct(m: MetaV2): number {
    return m.total > 0 ? Math.min((m.abonado / m.total) * 100, 100) : 0;
  }

  function metaPendingLabel(tipo: string): string {
    if (tipo === "me_deben") return "por cobrar";
    if (tipo === "debo")     return "por pagar";
    return "por juntar";
  }

  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function formatDate(d: string): string {
    const [, m, day] = d.split("-");
    return `${parseInt(day)} ${MESES_CORTO[parseInt(m) - 1]}`;
  }

</script>

<div class="page-shell">
  <header class="page-header">
    <div class="header-left">
      <h1>Resumen</h1>
      {#if activePeriod === "Month"}
        {@const now = new Date()}
        <span class="period-label">{MESES[now.getMonth()]} {now.getFullYear()}</span>
      {/if}
    </div>
    <nav class="period-selector">
      {#each (Object.keys(PERIOD_LABELS) as PeriodKey[]) as key}
        <button class:active={activePeriod === key} onclick={() => { activePeriod = key; }}>
          {PERIOD_LABELS[key]}
        </button>
      {/each}
    </nav>
  </header>

  {#if error}
    <div class="banner error"><strong>Error</strong><pre>{error}</pre></div>
  {/if}

  <!-- Barra de estado: los dos números que más importan, siempre visibles, fuera del scroll -->
  <section class="status-bar">
    <div
      class="status-item"
      class:balance-pos={!loading && (globalBal?.disponible ?? 0) >= 0}
      class:balance-neg={!loading && (globalBal?.disponible ?? 0) < 0}
    >
      {#if isActivePoint("resumen", 0)}<TourPoint text="Cuánto tienes disponible ahora" />{/if}
      <span class="status-label">Disponible</span>
      <span class="status-value">{loading ? "…" : formatCOP(globalBal?.disponible ?? 0)}</span>
    </div>
    <div class="status-divider"></div>
    <div class="status-item">
      {#if isActivePoint("resumen", 1)}<TourPoint text="Ahorros y deudas incluidos" />{/if}
      <span class="status-label">Patrimonio</span>
      <span class="status-value status-value-secondary">{loading ? "…" : formatCOP(globalBal?.patrimonio ?? 0)}</span>
    </div>
  </section>

  <div class="resumen-grid">
    <!-- Left column: período + presupuestos -->
    <div class="left-col">
      <ScrollArea class="left-scroll" scrollbar="thin">

      <section class="period-panel">
        <div class="section-header">
          <h2>Este período</h2>
          {#if !loading && comparison !== null && comparison.previous_month_total > 0}
            {@const delta = comparison.delta_percentage}
            {@const up = delta > 0}
            <span
              class="period-cmp"
              class:cmp-up={up}
              class:cmp-down={!up && delta < 0}
              title="{formatCOP(comparison.current_month_total)} vs {formatCOP(comparison.previous_month_total)} el mes pasado"
            >
              Gastos vs {prevMonthName} {up ? "↑" : delta < 0 ? "↓" : "—"} {Math.abs(delta).toFixed(1)}%
            </span>
          {:else if loading}
            <span class="placeholder-inline"></span>
          {/if}
        </div>
        <div class="period-stats">
          <div class="period-stat">
            <span class="period-stat-label">Ingresos</span>
            <span class="period-stat-value income">{loading ? "…" : formatCOP(summary?.total_income ?? 0)}</span>
          </div>
          <div class="period-stat-div"></div>
          <div class="period-stat" title="Incluye compras a crédito (deudas nuevas) desde el día en que se registran, aunque no hayas pagado nada todavía">
            <span class="period-stat-label">Gastos</span>
            <span class="period-stat-value expense">{loading ? "…" : formatCOP(summary?.total_expense ?? 0)}</span>
          </div>
          <div class="period-stat-div"></div>
          <div
            class="period-stat"
            class:balance-pos={!loading && (summary?.balance ?? 0) >= 0}
            class:balance-neg={!loading && (summary?.balance ?? 0) < 0}
          >
            <span class="period-stat-label">Saldo período</span>
            <span class="period-stat-value">{loading ? "…" : formatCOP(summary?.balance ?? 0)}</span>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-header">
          <h2>Presupuestos</h2>
          <div class="budget-toggle">
            <button class:active={budgetView === "ingresos"} onclick={() => { budgetView = "ingresos"; }}>Ingresos</button>
            <button class:active={budgetView === "gastos"}   onclick={() => { budgetView = "gastos"; }}>Gastos</button>
          </div>
        </div>

        {#if loading}
          <div class="placeholder-list">
            {#each [1,2,3,4] as _}<div class="placeholder-row"></div>{/each}
          </div>

        {:else if budgetView === "ingresos"}
          {#if incomeFixed.length === 0 && incomeVariable.length === 0}
            <p class="empty">Sin ingresos registrados en este período.</p>
          {:else}
            <ul class="category-list">
              {#if incomeFixed.length > 0}
                <li class="group-label">FIJOS</li>
                {#each incomeFixed as cat}
                  {@const pct = Math.min(cat.percentage, 100)}
                  <li class="category-row">
                    <div class="cat-header">
                      <div class="cat-name-col">
                        <span class="cat-name">{cat.category_name}</span>
                      </div>
                      <span class="cat-amounts">
                        <span class:income-over={cat.is_over}>{formatCOP(cat.current_amount)}</span>
                        {#if cat.monthly_target > 0}
                          <span class="cat-target"> / {formatCOP(cat.monthly_target)}</span>
                        {/if}
                      </span>
                    </div>
                    {#if cat.monthly_target > 0}
                      <div class="progress-row">
                        <div class="bar-track">
                          <div class="bar-fill" class:bar-income-over={cat.is_over} style="width: {pct}%"></div>
                        </div>
                        <span class="cat-pct" class:income-over={cat.is_over}>{cat.percentage.toFixed(0)}% META</span>
                      </div>
                    {/if}
                  </li>
                {/each}
              {/if}
              {#if incomeVariable.length > 0}
                <li class="group-label">VARIABLES</li>
                {#each incomeVariable as cat}
                  <li class="category-row">
                    <div class="cat-header">
                      <div class="cat-name-col">
                        <span class="cat-name">{cat.category_name}</span>
                      </div>
                      <span class="cat-amounts">{formatCOP(cat.current_amount)}</span>
                    </div>
                    {#if cat.monthly_target > 0}
                      {@const pct = Math.min(cat.percentage, 100)}
                      <div class="progress-row">
                        <div class="bar-track">
                          <div class="bar-fill" class:bar-income-over={cat.is_over} style="width: {pct}%"></div>
                        </div>
                        <span class="cat-pct" class:income-over={cat.is_over}>{cat.percentage.toFixed(0)}% META</span>
                      </div>
                    {:else}
                      <span class="cat-no-meta">sin meta definida</span>
                    {/if}
                  </li>
                {/each}
              {/if}
            </ul>
            {#if incomeTotals.target > 0}
              {@const barPct = Math.min(incomeTotals.pct, 100)}
              {@const over   = incomeTotals.current > incomeTotals.target}
              <div class="totals-row">
                <div class="totals-header">
                  <span class="totals-label">TOTAL INGRESOS</span>
                  <span class="totals-amounts">
                    <span class:income-over={over}>{formatCOP(incomeTotals.current)}</span>
                    <span class="totals-target"> / {formatCOP(incomeTotals.target)}</span>
                  </span>
                </div>
                <div class="progress-row">
                  <div class="bar-track">
                    <div class="bar-fill bar-income-over" style="width: {barPct}%"></div>
                  </div>
                  <span class="cat-pct" class:income-over={over}>{incomeTotals.pct.toFixed(0)}% META</span>
                </div>
              </div>
            {/if}
          {/if}

        {:else}
          {#if expenseTracked.length === 0}
            <p class="empty">Sin gastos registrados en este período.</p>
          {:else}
            <ul class="category-list">
              {#each expenseTracked as cat}
                {@const pct = Math.min(cat.percentage, 100)}
                <li class="category-row">
                  <div class="cat-header">
                    <div class="cat-name-col">
                      <span class="cat-name">{cat.category_name}</span>
                    </div>
                    <span class="cat-amounts">
                      <span class:over={cat.is_over}>{formatCOP(cat.current_amount)}</span>
                      {#if cat.monthly_target > 0}
                        <span class="cat-target"> / {formatCOP(cat.monthly_target)}</span>
                      {/if}
                    </span>
                  </div>
                  {#if cat.monthly_target > 0}
                    <div class="progress-row">
                      <div class="bar-track">
                        <div class="bar-fill" class:bar-over={cat.is_over} style="width: {pct}%"></div>
                      </div>
                      <span class="cat-pct" class:over={cat.is_over}>{cat.percentage.toFixed(0)}% LÍMITE</span>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
            {#if expenseTotals.target > 0}
              {@const barPct = Math.min(expenseTotals.pct, 100)}
              {@const over   = expenseTotals.current > expenseTotals.target}
              <div class="totals-row">
                <div class="totals-header">
                  <span class="totals-label">TOTAL GASTOS</span>
                  <span class="totals-amounts">
                    <span class:over={over}>{formatCOP(expenseTotals.current)}</span>
                    <span class="totals-target"> / {formatCOP(expenseTotals.target)}</span>
                  </span>
                </div>
                <div class="progress-row">
                  <div class="bar-track">
                    <div class="bar-fill" class:bar-over={over} style="width: {barPct}%"></div>
                  </div>
                  <span class="cat-pct" class:over={over}>{expenseTotals.pct.toFixed(0)}% LÍMITE</span>
                </div>
              </div>
            {/if}
          {/if}
        {/if}
      </section>
      </ScrollArea>
    </div>

    <!-- Right column: recent transactions -->
    <div class="right-col">
      <ScrollArea class="right-scroll" scrollbar="thin">
      <section class="section">
        <h2>Últimas transacciones</h2>
        {#if loading}
          <div class="placeholder-list">
            {#each [1,2,3,4,5,6] as _}<div class="placeholder-row short"></div>{/each}
          </div>
        {:else if recent.length === 0}
          <p class="empty">Sin transacciones en este período.</p>
        {:else}
          <ul class="tx-list">
            {#each recent as tx}
              <li class="tx-row">
                <span class="tx-date">{formatDate(tx.date)}</span>
                <span class="tx-category">{tx.category}</span>
                <span class="tx-amount" class:income={tx.type === "income"} class:expense={tx.type === "expense"}>
                  {tx.type === "income" ? "+" : tx.type === "expense" ? "−" : "→"}{formatCOP(tx.amount)}
                </span>
              </li>
            {/each}
          </ul>
          <a href="/historial" class="ver-todo">Ver todo →</a>
        {/if}
      </section>

      <!-- ── Widget tanque ── -->
      {#if !fuelLoading && (fuelStatuses.length > 0 || hasVehiclesWithoutTank)}
        <section class="section">
          <h2>Tanque</h2>

          {#if fuelStatuses.length > 0}
            {#each fuelStatuses as fs}
              {@const pct = fs.tank_percentage ?? 0}
              <div class="fuel-card">
                <div class="fuel-header">
                  <span class="fuel-name">{fs.vehicle_name}</span>
                  {#if fs.tank_percentage != null}
                    <span class="fuel-pct">{Math.round(pct)}%</span>
                  {/if}
                </div>

                <div class="bar-track fuel-track">
                  <div class="fuel-bar" class:fuel-bar-low={pct < 20} style="width: {pct}%"></div>
                </div>

                <div class="fuel-meta">
                  <span class="fuel-autonomy">~{Math.round(fs.autonomy_km)} km</span>
                  <span class="fuel-gallons">{fs.level_gallons.toFixed(1)} gal</span>
                </div>

                {#if fs.overCapacity}
                  <p class="fuel-warning">
                    ⚠ El nivel calculado superó la capacidad del tanque — revisa el rendimiento (km/gal) de este vehículo en <a href="/config">Ajustes</a>.
                  </p>
                {/if}
              </div>
            {/each}
          {:else}
            <p class="fuel-setup-hint">
              Agrega la capacidad del tanque de tu vehículo en
              <a href="/config">Ajustes</a> para ver la autonomía.
            </p>
          {/if}
        </section>
      {/if}

      <!-- ── Objetivos (préstamos, deudas, ahorros pendientes) ── -->
      {#if !metasLoading && pendingMetas.length > 0}
        <section class="section">
          <h2>Objetivos</h2>
          {#each pendingMetas as m (m.id)}
            <div class="meta-mini">
              <div class="meta-mini-top">
                <span class="meta-mini-name">{m.nombre}</span>
                <span class="meta-mini-pct">{metaPct(m).toFixed(0)}%</span>
              </div>
              <div class="bar-track">
                <div class="bar-fill" style="width: {metaPct(m)}%"></div>
              </div>
              <div class="meta-mini-amounts">
                <span class="meta-mini-pending">{formatCOP(m.pendiente)}</span>
                <span class="meta-mini-label">{metaPendingLabel(m.tipo)}</span>
              </div>
            </div>
          {/each}
          <a href="/metas" class="ver-todo">Ver todo →</a>
        </section>
      {/if}

      </ScrollArea>
    </div>
  </div>
</div>

<style>
  .page-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .page-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.875rem 1rem 0.5rem;
    border-bottom: 1px solid var(--border);
    gap: 0.75rem;
  }

  .header-left {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }

  h1 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .period-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    text-transform: capitalize;
  }

  .banner.error {
    flex-shrink: 0;
    margin: 0.5rem 1rem 0;
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    border-radius: var(--radius);
    padding: 0.65rem 1rem;
    color: var(--danger);
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .banner.error pre { font-size: 0.72rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }

  .resumen-grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: 0;
    overflow: hidden;
    min-height: 0;
  }

  .left-col {
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: 0.875rem 0.75rem 0.875rem 1rem;
    border-right: 1px solid var(--border);
  }

  .right-col {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 0.875rem 1rem;
  }

  :global(.left-scroll) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  :global(.right-scroll) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* Period selector */
  .period-selector {
    display: flex;
    gap: 3px;
    background: var(--bg-elevated);
    padding: 3px;
    border-radius: var(--radius);
  }

  .period-selector button {
    padding: 0.28rem 0.65rem;
    border-radius: var(--radius);
    font-size: 0.72rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .period-selector button:hover { color: var(--text-primary); background: var(--bg-surface); }
  .period-selector button.active { background: var(--accent); color: var(--bg-base); }

  /* ── Barra de estado (Disponible / Patrimonio) ── */
  .status-bar {
    flex-shrink: 0;
    display: flex;
    align-items: stretch;
    margin: 0.875rem 1rem 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .status-item {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.8rem 1.25rem;
  }

  .status-divider { width: 1px; background: var(--border); }

  .status-label {
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .status-value {
    font-size: 1.6rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .status-item.balance-pos .status-value { color: var(--accent); }
  .status-item.balance-neg .status-value { color: var(--danger); }
  .status-value-secondary { color: var(--text-secondary) !important; }


  /* ── Panel del período ── */
  .period-panel {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.875rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .period-cmp {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  .period-cmp.cmp-up   { color: var(--danger); }
  .period-cmp.cmp-down { color: var(--success); }

  .period-stats { display: flex; align-items: stretch; }
  .period-stat-div { width: 1px; background: var(--border); margin: 0 0.75rem; }

  .period-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .period-stat-label { font-size: 0.65rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }
  .period-stat-value { font-size: 1.2rem; font-weight: 700; font-variant-numeric: tabular-nums; font-family: var(--font-mono); color: var(--text-primary); }

  .period-stat-value.income  { color: var(--success); }
  .period-stat-value.expense { color: var(--danger); }
  .period-stat.balance-pos .period-stat-value { color: var(--success); }
  .period-stat.balance-neg .period-stat-value { color: var(--danger); }

  .placeholder-inline {
    display: inline-block;
    width: 90px;
    height: 12px;
    background: var(--bg-elevated);
    animation: shimmer 1.4s ease-in-out infinite;
  }

  /* Secciones */
  .section {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.875rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  h2 { font-size: 0.68rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }

  .empty { color: var(--text-muted); font-size: 0.82rem; padding: 0.4rem 0; }

  /* Placeholders */
  .placeholder-list { display: flex; flex-direction: column; gap: 0.4rem; }

  .placeholder-row {
    height: 48px;
    border-radius: var(--radius);
    background: var(--bg-surface);
    animation: shimmer 1.4s ease-in-out infinite;
  }

  .placeholder-row.short { height: 32px; }

  @keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.7; } }

  /* Presupuestos header + toggle */
  .section-header { display: flex; align-items: center; justify-content: space-between; }

  .budget-toggle {
    display: flex;
    gap: 2px;
    background: var(--bg-elevated);
    padding: 2px;
    border-radius: var(--radius);
  }

  .budget-toggle button {
    padding: 0.18rem 0.55rem;
    border-radius: var(--radius);
    font-size: 0.68rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .budget-toggle button.active { background: var(--accent); color: var(--bg-base); }
  .budget-toggle button:not(.active):hover { color: var(--text-primary); }

  /* Grupo label */
  .group-label {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    text-transform: uppercase;
    list-style: none;
    padding: 0.25rem 0 0;
  }

  /* Categorías */
  .category-list { list-style: none; display: flex; flex-direction: column; gap: 0.5rem; }

  .category-row {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .cat-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.5rem; }

  .cat-name-col { display: flex; flex-direction: column; gap: 0.12rem; min-width: 0; }
  .cat-name { font-size: 0.82rem; color: var(--text-primary); }

  .cat-amounts { font-size: 0.78rem; font-variant-numeric: tabular-nums; font-family: var(--font-mono); text-align: right; white-space: nowrap; flex-shrink: 0; color: var(--text-secondary); }
  .cat-amounts .over { color: var(--danger); font-weight: 600; }
  .cat-amounts .income-over { color: var(--success); font-weight: 600; }
  .cat-target { color: var(--text-muted); }

  .progress-row { display: flex; align-items: center; gap: 0.4rem; }
  .bar-track { flex: 1; height: 3px; background: var(--bg-elevated); overflow: hidden; }
  .bar-fill { height: 100%; background: var(--accent); transition: width 0.3s ease; min-width: 2px; }
  .bar-fill.bar-over { background: var(--danger); }
  .bar-fill.bar-income-over { background: var(--success); }

  .cat-pct { font-size: 0.68rem; color: var(--text-muted); white-space: nowrap; }
  .cat-pct.over { color: var(--danger); font-weight: 600; }
  .cat-pct.income-over { color: var(--success); font-weight: 600; }

  .cat-no-meta { font-size: 0.68rem; color: var(--text-muted); font-style: italic; }

  /* Totales */
  .totals-row {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .totals-header { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }

  .totals-label { font-size: 0.62rem; font-weight: 700; letter-spacing: 0.08em; color: var(--text-muted); text-transform: uppercase; }

  .totals-amounts { font-size: 0.8rem; font-variant-numeric: tabular-nums; font-family: var(--font-mono); font-weight: 600; text-align: right; white-space: nowrap; color: var(--text-secondary); }
  .totals-amounts .over        { color: var(--danger); }
  .totals-amounts .income-over { color: var(--success); }
  .totals-target { color: var(--text-muted); font-weight: 400; }

  /* Transacciones recientes */
  .tx-list { list-style: none; display: flex; flex-direction: column; gap: 1px; background: var(--border); border-radius: var(--radius); overflow: hidden; }

  .tx-row { display: grid; grid-template-columns: 44px 1fr auto; align-items: center; gap: 0.6rem; padding: 0.55rem 0.75rem; background: var(--bg-surface); }
  .tx-row:hover { background: var(--bg-elevated); }

  .tx-date { font-size: 0.72rem; color: var(--text-muted); font-variant-numeric: tabular-nums; font-family: var(--font-mono); white-space: nowrap; }
  .tx-category { font-size: 0.82rem; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .tx-amount { font-size: 0.82rem; font-weight: 600; font-variant-numeric: tabular-nums; font-family: var(--font-mono); white-space: nowrap; }
  .tx-amount.income  { color: var(--success); }
  .tx-amount.expense { color: var(--danger); }

  .ver-todo {
    display: block;
    font-size: 0.78rem;
    color: var(--accent);
    text-decoration: none;
    padding: 0.35rem 0;
    text-align: right;
  }

  .ver-todo:hover { color: var(--accent-hover); }

  /* ── Objetivos ── */
  .meta-mini {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .meta-mini-top { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .meta-mini-name {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta-mini-pct { font-size: 0.72rem; font-weight: 600; color: var(--accent); font-family: var(--font-mono); flex-shrink: 0; }

  .meta-mini-amounts { display: flex; align-items: baseline; gap: 0.35rem; font-size: 0.75rem; }
  .meta-mini-pending { font-weight: 600; color: var(--text-secondary); font-variant-numeric: tabular-nums; font-family: var(--font-mono); }
  .meta-mini-label { color: var(--text-muted); }

  /* ── Widget tanque ── */
  .fuel-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .fuel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .fuel-name {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fuel-pct {
    font-size: 0.72rem;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--accent);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .fuel-track { margin: 0; }

  .fuel-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
    min-width: 2px;
  }

  .fuel-bar.fuel-bar-low { background: var(--danger); }

  .fuel-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums; font-family: var(--font-mono);
  }

  .fuel-autonomy { color: var(--text-secondary); font-weight: 500; }
  .fuel-gallons  { color: var(--text-muted); }

  .fuel-warning {
    font-size: 0.7rem;
    color: var(--accent);
    line-height: 1.4;
    margin: 0;
  }
  .fuel-warning a { color: inherit; text-decoration: underline; }

  .fuel-setup-hint {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
  }

  .fuel-setup-hint a { color: var(--accent); text-decoration: none; }
  .fuel-setup-hint a:hover { text-decoration: underline; }
</style>
