<script lang="ts">
  import { transactionApi } from "$lib/api";
  import type { PeriodSummary, CategoryProgress, MonthComparison, TransactionPage } from "$lib/types";
  import { txState } from "$lib/txState.svelte";
  import { MESES, MESES_CORTO, DASHBOARD_RECENT_SIZE } from "$lib/constants";

  type PeriodKey = "Daily" | "Weekly" | "Monthly" | "Yearly";

  const PERIOD_LABELS: Record<PeriodKey, string> = {
    Daily: "Diario",
    Weekly: "Semanal",
    Monthly: "Mensual",
    Yearly: "Anual",
  };

  let activePeriod = $state<PeriodKey>("Monthly");
  let summary      = $state<PeriodSummary | null>(null);
  let categories   = $state<CategoryProgress[]>([]);
  let recent       = $state<{ id: number; date: string; type: string; category: string; amount: number }[]>([]);
  let comparison   = $state<MonthComparison | null>(null);
  let loading      = $state(true);
  let error        = $state<string | null>(null);
  let budgetView   = $state<"ingresos" | "gastos">("ingresos");

  let incomeFixed    = $derived(categories.filter(c => c.kind === "ingreso" && c.is_fixed));
  let incomeVariable = $derived(categories.filter(c => c.kind === "ingreso" && !c.is_fixed));
  let expenseTracked = $derived(categories.filter(c => c.kind === "gasto"));

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
          const p = { type: period };
          const [sum, cats, page, cmp] = await Promise.all([
            transactionApi.getPeriodSummary(p),
            transactionApi.getCategoryProgress(p),
            transactionApi.list({ period: p, page_size: DASHBOARD_RECENT_SIZE }),
            transactionApi.getMonthComparison(),
          ]);
          if (!cancelled) {
            summary    = sum;
            categories = cats;
            recent     = page.transactions;
            comparison = cmp;
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
      {#if activePeriod === "Monthly"}
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

  <div class="resumen-grid">
    <!-- Left column: KPIs + comparison + budgets -->
    <div class="left-col">
      <section class="kpis">
        <div class="kpi-card income">
          <span class="kpi-label">Ingresos</span>
          <span class="kpi-value">{loading ? "…" : formatCOP(summary?.total_income ?? 0)}</span>
        </div>
        <div class="kpi-card expenses">
          <span class="kpi-label">Gastos</span>
          <span class="kpi-value">{loading ? "…" : formatCOP(summary?.total_expenses ?? 0)}</span>
        </div>
        <div
          class="kpi-card"
          class:balance-pos={!loading && (summary?.balance ?? 0) >= 0}
          class:balance-neg={!loading && (summary?.balance ?? 0) < 0}
        >
          <span class="kpi-label">Saldo período</span>
          <span class="kpi-value">{loading ? "…" : formatCOP(summary?.balance ?? 0)}</span>
        </div>
      </section>

      {#if !loading && comparison !== null && comparison.previous_month_total > 0}
        {@const delta = comparison.delta_percentage}
        {@const up = delta > 0}
        <div class="comparison-row">
          <span class="cmp-label">Gastos vs {prevMonthName}</span>
          <span class="cmp-value" class:cmp-up={up} class:cmp-down={!up && delta < 0}>
            {up ? "↑" : delta < 0 ? "↓" : "—"}
            {Math.abs(delta).toFixed(1)}%
            <span class="cmp-detail">
              ({formatCOP(comparison.current_month_total)} vs {formatCOP(comparison.previous_month_total)})
            </span>
          </span>
        </div>
      {:else if loading}
        <div class="placeholder-row short"></div>
      {/if}

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
                        <span class="cat-name">{cat.category}</span>
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
                        <span class="cat-name">{cat.category}</span>
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
                      <span class="cat-name">{cat.category}</span>
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
    </div>

    <!-- Right column: recent transactions -->
    <div class="right-col">
      <section class="section">
        <h2>Últimas transacciones</h2>
        {#if loading}
          <div class="placeholder-list">
            {#each [1,2,3,4,5] as _}<div class="placeholder-row short"></div>{/each}
          </div>
        {:else if recent.length === 0}
          <p class="empty">Sin transacciones en este período.</p>
        {:else}
          <ul class="tx-list">
            {#each recent as tx}
              <li class="tx-row">
                <span class="tx-date">{formatDate(tx.date)}</span>
                <span class="tx-category">{tx.category}</span>
                <span class="tx-amount" class:income={tx.type === "ingreso"} class:expense={tx.type === "gasto"}>
                  {tx.type === "ingreso" ? "+" : "−"}{formatCOP(tx.amount)}
                </span>
              </li>
            {/each}
          </ul>
          <a href="/historial" class="ver-todo">Ver todo →</a>
        {/if}
      </section>
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
    grid-template-columns: 1fr 300px;
    gap: 0;
    overflow: hidden;
    min-height: 0;
  }

  .left-col {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.875rem 0.75rem 0.875rem 1rem;
    border-right: 1px solid var(--border);
  }

  .right-col {
    overflow-y: auto;
    padding: 0.875rem 1rem;
  }

  /* Period selector */
  .period-selector {
    display: flex;
    gap: 3px;
    background: var(--bg-elevated);
    padding: 3px;
    border-radius: 7px;
  }

  .period-selector button {
    padding: 0.28rem 0.65rem;
    border-radius: 5px;
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .period-selector button:hover { color: var(--text-primary); background: var(--bg-surface); }
  .period-selector button.active { background: var(--accent); color: #fff; }

  /* KPIs */
  .kpis { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.6rem; }

  .kpi-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .kpi-label { font-size: 0.65rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }
  .kpi-value { font-size: 1.1rem; font-weight: 700; font-variant-numeric: tabular-nums; color: var(--text-primary); }

  .kpi-card.income .kpi-value       { color: var(--success); }
  .kpi-card.expenses .kpi-value     { color: var(--danger); }
  .kpi-card.balance-pos .kpi-value  { color: var(--success); }
  .kpi-card.balance-neg .kpi-value  { color: var(--danger); }

  /* Comparativa */
  .comparison-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5rem 0.875rem;
    font-size: 0.82rem;
  }

  .cmp-label { color: var(--text-secondary); }
  .cmp-value { font-weight: 600; font-variant-numeric: tabular-nums; color: var(--text-muted); }
  .cmp-value.cmp-up   { color: var(--danger); }
  .cmp-value.cmp-down { color: var(--success); }
  .cmp-detail { font-weight: 400; font-size: 0.75rem; color: var(--text-muted); margin-left: 0.35rem; }

  /* Secciones */
  .section { display: flex; flex-direction: column; gap: 0.6rem; }

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
    border-radius: 6px;
  }

  .budget-toggle button {
    padding: 0.18rem 0.55rem;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .budget-toggle button.active { background: var(--accent); color: #fff; }
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

  .cat-amounts { font-size: 0.78rem; font-variant-numeric: tabular-nums; text-align: right; white-space: nowrap; flex-shrink: 0; color: var(--text-secondary); }
  .cat-amounts .over { color: var(--danger); font-weight: 600; }
  .cat-amounts .income-over { color: var(--success); font-weight: 600; }
  .cat-target { color: var(--text-muted); }

  .progress-row { display: flex; align-items: center; gap: 0.4rem; }
  .bar-track { flex: 1; height: 4px; background: var(--bg-elevated); border-radius: 999px; overflow: hidden; }
  .bar-fill { height: 100%; background: var(--accent); border-radius: 999px; transition: width 0.4s ease; min-width: 2px; }
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

  .totals-amounts { font-size: 0.8rem; font-variant-numeric: tabular-nums; font-weight: 600; text-align: right; white-space: nowrap; color: var(--text-secondary); }
  .totals-amounts .over        { color: var(--danger); }
  .totals-amounts .income-over { color: var(--success); }
  .totals-target { color: var(--text-muted); font-weight: 400; }

  /* Transacciones recientes */
  .tx-list { list-style: none; display: flex; flex-direction: column; gap: 1px; background: var(--border); border-radius: var(--radius); overflow: hidden; }

  .tx-row { display: grid; grid-template-columns: 44px 1fr auto; align-items: center; gap: 0.6rem; padding: 0.55rem 0.75rem; background: var(--bg-surface); }
  .tx-row:hover { background: var(--bg-elevated); }

  .tx-date { font-size: 0.72rem; color: var(--text-muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tx-category { font-size: 0.82rem; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .tx-amount { font-size: 0.82rem; font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
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
</style>
