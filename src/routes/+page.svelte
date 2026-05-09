<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { PeriodSummary, CategoryProgress, Transaction, MonthComparison } from "$lib/types";

  type PeriodKey = "Daily" | "Weekly" | "Monthly" | "Yearly";

  const PERIOD_LABELS: Record<PeriodKey, string> = {
    Daily: "Diario",
    Weekly: "Semanal",
    Monthly: "Mensual",
    Yearly: "Anual",
  };

  const MESES = ["enero","febrero","marzo","abril","mayo","junio",
                 "julio","agosto","septiembre","octubre","noviembre","diciembre"];

  let activePeriod = $state<PeriodKey>("Monthly");
  let summary      = $state<PeriodSummary | null>(null);
  let categories   = $state<CategoryProgress[]>([]);
  let recent       = $state<Transaction[]>([]);
  let comparison   = $state<MonthComparison | null>(null);
  let loading      = $state(true);
  let error        = $state<string | null>(null);

  const prevMonthName = (() => {
    const now = new Date();
    const m = now.getMonth() === 0 ? 11 : now.getMonth() - 1;
    return MESES[m];
  })();

  $effect(() => {
    const period = activePeriod;
    let cancelled = false;

    async function load() {
      loading = true;
      error = null;

      while (!cancelled) {
        try {
          await invoke("list_budgets");
          break;
        } catch (e: unknown) {
          const err = e as { kind?: string; message?: string };
          if (err?.kind === "DatabaseError" && err?.message?.includes("no inicializada")) {
            await new Promise((r) => setTimeout(r, 300));
          } else {
            if (!cancelled) { error = JSON.stringify(e); loading = false; }
            return;
          }
        }
      }

      if (cancelled) return;

      try {
        const p = { type: period };
        const [sum, cats, txs, cmp] = await Promise.all([
          invoke<PeriodSummary>("get_period_summary", { period: p }),
          invoke<CategoryProgress[]>("get_category_progress", { period: p }),
          invoke<Transaction[]>("list_transactions", { filter: { period: p } }),
          invoke<MonthComparison>("get_month_comparison"),
        ]);
        if (!cancelled) {
          summary    = sum;
          categories = cats;
          recent     = txs.slice(0, 5);
          comparison = cmp;
          loading    = false;
        }
      } catch (e: unknown) {
        if (!cancelled) { error = JSON.stringify(e); loading = false; }
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
    const meses = ["ene","feb","mar","abr","may","jun","jul","ago","sep","oct","nov","dic"];
    return `${parseInt(day)} ${meses[parseInt(m) - 1]}`;
  }
</script>

<main>
  <header>
    <h1>Resumen</h1>
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

  <!-- KPIs -->
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
      <span class="kpi-label">Saldo</span>
      <span class="kpi-value">{loading ? "…" : formatCOP(summary?.balance ?? 0)}</span>
    </div>
  </section>

  <!-- Comparativa mes anterior -->
  {#if !loading && comparison !== null}
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

  <!-- Barras de categoría -->
  <section class="section">
    <h2>Presupuestos</h2>
    {#if loading}
      <div class="placeholder-list">
        {#each [1,2,3,4] as _}<div class="placeholder-row"></div>{/each}
      </div>
    {:else if categories.length === 0}
      <p class="empty">Sin categorías para este período.</p>
    {:else}
      <ul class="category-list">
        {#each categories as cat}
          {@const pct = Math.min(cat.percentage, 100)}
          <li class="category-row">
            <div class="cat-header">
              <span class="cat-name">{cat.category}</span>
              <span class="cat-amounts">
                <span class:over={cat.is_over}>{formatCOP(cat.current_amount)}</span>
                {#if cat.monthly_target > 0}
                  <span class="cat-target"> / {formatCOP(cat.monthly_target)}</span>
                {/if}
              </span>
            </div>
            <div class="bar-track">
              <div class="bar-fill" class:bar-over={cat.is_over} style="width: {pct}%"></div>
            </div>
            {#if cat.monthly_target > 0}
              <span class="cat-pct" class:over={cat.is_over}>{cat.percentage.toFixed(0)}%</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <!-- Últimas transacciones -->
  <section class="section">
    <h2>Últimas transacciones</h2>
    {#if loading}
      <div class="placeholder-list">
        {#each [1,2,3] as _}<div class="placeholder-row short"></div>{/each}
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
    {/if}
  </section>
</main>

<style>
  main {
    max-width: 760px;
    margin: 0 auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .period-selector {
    display: flex;
    gap: 4px;
    background: var(--bg-elevated);
    padding: 4px;
    border-radius: 8px;
  }

  .period-selector button {
    padding: 0.3rem 0.75rem;
    border-radius: 5px;
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .period-selector button:hover { color: var(--text-primary); background: var(--bg-surface); }
  .period-selector button.active { background: var(--accent); color: #fff; }

  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
    color: var(--danger);
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .banner.error pre { font-size: 0.72rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }

  /* KPIs */
  .kpis { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; }

  .kpi-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .kpi-label { font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }
  .kpi-value { font-size: 1.2rem; font-weight: 700; font-variant-numeric: tabular-nums; color: var(--text-primary); }

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
    padding: 0.6rem 1rem;
    font-size: 0.85rem;
  }

  .cmp-label { color: var(--text-secondary); }

  .cmp-value {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .cmp-value.cmp-up   { color: var(--danger); }
  .cmp-value.cmp-down { color: var(--success); }

  .cmp-detail { font-weight: 400; font-size: 0.78rem; color: var(--text-muted); margin-left: 0.4rem; }

  /* Secciones */
  .section { display: flex; flex-direction: column; gap: 0.75rem; }

  h2 { font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }

  .empty { color: var(--text-muted); font-size: 0.85rem; padding: 0.5rem 0; }

  /* Placeholders */
  .placeholder-list { display: flex; flex-direction: column; gap: 0.5rem; }

  .placeholder-row {
    height: 52px;
    border-radius: var(--radius);
    background: var(--bg-surface);
    animation: shimmer 1.4s ease-in-out infinite;
  }

  .placeholder-row.short { height: 36px; }

  @keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.7; } }

  /* Categorías */
  .category-list { list-style: none; display: flex; flex-direction: column; gap: 0.6rem; }

  .category-row {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.65rem 0.875rem;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    gap: 0.35rem 0.5rem;
    align-items: center;
  }

  .cat-header { display: contents; }
  .cat-name { font-size: 0.875rem; color: var(--text-primary); align-self: center; }

  .cat-amounts {
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    text-align: right;
    align-self: center;
  }

  .cat-amounts .over { color: var(--danger); font-weight: 600; }
  .cat-amounts:not(:has(.over)) { color: var(--text-secondary); }
  .cat-target { color: var(--text-muted); }

  .bar-track { grid-column: 1 / 2; height: 5px; background: var(--bg-elevated); border-radius: 999px; overflow: hidden; }
  .bar-fill { height: 100%; background: var(--accent); border-radius: 999px; transition: width 0.4s ease; min-width: 2px; }
  .bar-fill.bar-over { background: var(--danger); }

  .cat-pct { grid-column: 2 / 3; grid-row: 2 / 3; font-size: 0.72rem; color: var(--text-muted); text-align: right; }
  .cat-pct.over { color: var(--danger); font-weight: 600; }

  /* Transacciones */
  .tx-list { list-style: none; display: flex; flex-direction: column; gap: 1px; background: var(--border); border-radius: var(--radius); overflow: hidden; }

  .tx-row { display: grid; grid-template-columns: 48px 1fr auto; align-items: center; gap: 0.75rem; padding: 0.6rem 0.875rem; background: var(--bg-surface); }
  .tx-row:hover { background: var(--bg-elevated); }

  .tx-date { font-size: 0.75rem; color: var(--text-muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tx-category { font-size: 0.875rem; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .tx-amount { font-size: 0.875rem; font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tx-amount.income  { color: var(--success); }
  .tx-amount.expense { color: var(--danger); }
</style>
