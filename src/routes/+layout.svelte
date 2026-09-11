<script lang="ts">
  import { page } from '$app/stores';
  import { txState } from "$lib/txState.svelte";
  import { entryApi, goalApi, categoryApi } from "$lib/api";
  import { MESES_CORTO, WIDGET_RECENT_SIZE } from "$lib/constants";
  import type { AccountBalances, PeriodSummaryV2, GoalWithProgressV2, Entry } from "$lib/types";
  import '../app.css';

  let { children } = $props();

  const navItems = [
    { href: '/',          label: 'Resumen'   },
    { href: '/registrar', label: 'Registrar' },
    { href: '/historial', label: 'Historial' },
    { href: '/metas',     label: 'Metas'     },
    { href: '/config',    label: 'Config'    },
  ];

  // ── Estado del widget ──────────────────────────────────────────────────────
  let widgetOpen = $state(localStorage.getItem("widget_open") !== "false");

  // ── Datos del widget ───────────────────────────────────────────────────────
  let balances     = $state<AccountBalances | null>(null);
  let monthSummary = $state<PeriodSummaryV2 | null>(null);
  let lastTx       = $state<Entry | null>(null);
  let lastTxLabel  = $state<string>("");
  let nextGoal     = $state<GoalWithProgressV2 | null>(null);

  // ── Persist widget state ───────────────────────────────────────────────────
  $effect(() => { localStorage.setItem("widget_open", String(widgetOpen)); });

  // ── Reload data on route change ────────────────────────────────────────────
  $effect(() => {
    const _path = $page.url.pathname;
    const _v    = txState.version;
    let cancelled = false;

    const now = new Date();
    Promise.all([
      entryApi.getAccountBalances(),
      entryApi.getPeriodSummary({ type: "Month", value: { year: now.getFullYear(), month: now.getMonth() + 1 } }),
      entryApi.list({ page: 1, page_size: WIDGET_RECENT_SIZE }),
      goalApi.list("saving"),
      categoryApi.list(),
    ]).then(([bal, summary, recent, goals, cats]) => {
      if (cancelled) return;
      balances     = bal;
      monthSummary = summary;
      const catMap = new Map(cats.map(c => [c.id, c.name]));
      const tx = recent.entries[0] ?? null;
      lastTx = tx;
      lastTxLabel = tx ? (tx.type === "transfer" ? "Transferencia" : (tx.category_id && catMap.get(tx.category_id)) ?? "Sin categoría") : "";
      nextGoal = goals.find(g => g.pending > 0) ?? goals[0] ?? null;
    }).catch(() => {});

    return () => { cancelled = true; };
  });

  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function formatDate(iso: string): string {
    const [, m, d] = iso.split("-");
    return `${parseInt(d)} ${MESES_CORTO[parseInt(m) - 1]}`;
  }
</script>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <img src="/app-icon.png" class="brand-icon" alt="" aria-hidden="true" />
      <span class="brand-name">FinCapX</span>
    </div>

    <nav class="sidebar-nav">
      {#each navItems as item}
        <a
          href={item.href}
          class="nav-item"
          class:active={$page.url.pathname === item.href}
        >{item.label}</a>
      {/each}
    </nav>

    <!-- Floating balance widget -->
    <div class="widget">
      <!-- Toggle row -->
      <button
        class="widget-toggle"
        onclick={() => { widgetOpen = !widgetOpen; }}
        aria-expanded={widgetOpen}
      >
        <span class="widget-row">
          <span class="widget-label">Disponible</span>
          <span
            class="widget-balance"
            class:pos={balances !== null && balances.disponible >= 0}
            class:neg={balances !== null && balances.disponible < 0}
          >
            {balances === null ? "…" : formatCOP(balances.disponible)}
          </span>
          <span class="widget-chevron" class:open={widgetOpen}>›</span>
        </span>
        {#if balances !== null && balances.patrimonio !== balances.disponible}
          <span class="widget-row widget-row-sub">
            <span class="widget-label widget-label-sub">Patrimonio</span>
            <span class="widget-balance widget-balance-sub">{formatCOP(balances.patrimonio)}</span>
          </span>
        {/if}
      </button>

      <!-- Expanded panel -->
      {#if widgetOpen}
        <div class="widget-panel">

          <!-- Patrimonio y otras cuentas -->
          {#if balances !== null}
            <div class="wp-section">
              <div class="wp-label">Patrimonio</div>
              <div class="wp-row">
                <span class="wp-key">Total</span>
                <span class="wp-val" class:pos={balances.patrimonio >= 0} class:neg={balances.patrimonio < 0}>{formatCOP(balances.patrimonio)}</span>
              </div>
              {#if balances.savings !== 0}
                <div class="wp-row"><span class="wp-key">Ahorros</span><span class="wp-val">{formatCOP(balances.savings)}</span></div>
              {/if}
              {#if balances.apps !== 0}
                <div class="wp-row"><span class="wp-key">Cuentas digitales</span><span class="wp-val">{formatCOP(balances.apps)}</span></div>
              {/if}
              {#if balances.receivable !== 0}
                <div class="wp-row"><span class="wp-key">Por cobrar</span><span class="wp-val">{formatCOP(balances.receivable)}</span></div>
              {/if}
              {#if balances.payable !== 0}
                <div class="wp-row"><span class="wp-key">Deuda</span><span class="wp-val expense">{formatCOP(balances.payable)}</span></div>
              {/if}
            </div>
            <div class="wp-divider"></div>
          {/if}

          <!-- Este mes -->
          {#if monthSummary}
            <div class="wp-section">
              <div class="wp-label">Este mes</div>
              <div class="wp-row">
                <span class="wp-key">Ingresos</span>
                <span class="wp-val income">+{formatCOP(monthSummary.total_income)}</span>
              </div>
              <div class="wp-row">
                <span class="wp-key">Gastos</span>
                <span class="wp-val expense">−{formatCOP(monthSummary.total_expense)}</span>
              </div>
            </div>
          {/if}

          <!-- Último registro -->
          {#if lastTx}
            <div class="wp-divider"></div>
            <div class="wp-section">
              <div class="wp-label">Último registro</div>
              <div class="wp-row">
                <span class="wp-last-cat">{lastTxLabel}</span>
                <span
                  class="wp-val"
                  class:income={lastTx.type === "income"}
                  class:expense={lastTx.type === "expense"}
                >
                  {lastTx.type === "income" ? "+" : lastTx.type === "expense" ? "−" : "→"}{formatCOP(lastTx.amount_cop)}
                </span>
              </div>
              <div class="wp-meta">{formatDate(lastTx.occurred_on)}{lastTx.note ? ` · ${lastTx.note}` : ""}</div>
            </div>
          {/if}

          <!-- Objetivo -->
          {#if nextGoal}
            <div class="wp-divider"></div>
            <div class="wp-section">
              <div class="wp-label">Objetivo</div>
              <div class="wp-goal-name">{nextGoal.goal.name}</div>
              <div class="wp-progress-track">
                <div
                  class="wp-progress-fill"
                  style="width: {Math.min(nextGoal.percentage, 100)}%"
                ></div>
              </div>
              <div class="wp-goal-meta">
                <span>{formatCOP(nextGoal.current_amount)}</span>
                <span class="wp-goal-pct">{nextGoal.percentage.toFixed(0)}%</span>
                <span>{formatCOP(nextGoal.goal.target_cop)}</span>
              </div>
            </div>
          {/if}

        </div>
      {/if}
    </div>
  </aside>

  <div class="content">
    {@render children()}
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  /* ── Sidebar ── */
  .sidebar {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-base);
    border-right: 1px solid var(--border);
    padding: 1rem 0 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 1rem 0.875rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.5rem;
  }

  .brand-icon { width: 22px; height: 22px; object-fit: contain; flex-shrink: 0; }
  .brand-name { font-size: 0.9rem; font-weight: 700; color: var(--text-primary); letter-spacing: 0.02em; text-transform: uppercase; }

  .sidebar-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 0 0.5rem;
  }

  .nav-item {
    display: block;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius);
    border-left: 2px solid transparent;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s, border-color 0.15s;
    text-decoration: none;
  }
  .nav-item:hover  { background: var(--bg-elevated); color: var(--text-primary); }
  .nav-item.active {
    background: var(--bg-elevated);
    border-left-color: var(--accent);
    color: var(--accent);
  }
  .nav-item.active::before { content: "› "; }

  /* ═══════════════════════════════════════
     WIDGET
  ═══════════════════════════════════════ */
  .widget {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }

  /* Toggle button (always visible) */
  .widget-toggle {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.65rem 1rem;
    width: 100%;
    text-align: left;
    transition: background 0.15s;
    cursor: pointer;
  }
  .widget-toggle:hover { background: var(--bg-elevated); }

  .widget-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .widget-label {
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .widget-balance {
    flex: 1;
    font-size: 0.88rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums; font-family: var(--font-mono);
    color: var(--text-secondary);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .widget-balance.pos { color: var(--success); }
  .widget-balance.neg { color: var(--danger); }

  .widget-row-sub { opacity: 0.7; }
  .widget-label-sub   { font-size: 0.6rem; }
  .widget-balance-sub { font-size: 0.76rem; font-weight: 600; }

  .widget-chevron {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s;
    transform: rotate(90deg);
  }
  .widget-chevron.open { transform: rotate(-90deg); }

  /* Expanded panel */
  .widget-panel {
    display: flex;
    flex-direction: column;
    padding: 0 1rem 0.75rem;
    animation: fadeIn 0.15s ease;
    overflow: hidden;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .wp-section { display: flex; flex-direction: column; gap: 0.25rem; padding: 0.35rem 0; }

  .wp-label {
    font-size: 0.62rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    margin-bottom: 0.1rem;
  }

  .wp-divider { height: 1px; background: var(--border); margin: 0.25rem 0; }

  .wp-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
  }

  .wp-key {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .wp-val {
    font-size: 0.78rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums; font-family: var(--font-mono);
    color: var(--text-secondary);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wp-val.income  { color: var(--success); }
  .wp-val.expense { color: var(--danger); }
  .wp-val.pos     { color: var(--success); }
  .wp-val.neg     { color: var(--danger); }

  .wp-last-cat {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .wp-meta {
    font-size: 0.68rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Goal */
  .wp-goal-name {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wp-progress-track {
    height: 3px;
    background: var(--border);
    overflow: hidden;
    margin: 0.25rem 0;
  }

  .wp-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }

  .wp-goal-meta {
    display: flex;
    justify-content: space-between;
    font-size: 0.65rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums; font-family: var(--font-mono);
  }

  .wp-goal-pct {
    font-weight: 600;
    color: var(--accent);
  }

  /* ── Content area ── */
  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
