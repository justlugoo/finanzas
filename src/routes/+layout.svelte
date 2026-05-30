<script lang="ts">
  import { page } from '$app/stores';
  import { txState } from "$lib/txState.svelte";
  import { transactionApi, goalApi } from "$lib/api";
  import { MESES_CORTO, WIDGET_RECENT_SIZE } from "$lib/constants";
  import type { CurrentBalance, PeriodSummary, TransactionPage, GoalWithProgress, Transaction } from "$lib/types";
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
  let balance      = $state<number | null>(null);
  let netWorth     = $state<number | null>(null);
  let monthSummary = $state<PeriodSummary | null>(null);
  let lastTx       = $state<Transaction | null>(null);
  let nextGoal     = $state<GoalWithProgress | null>(null);

  // ── Persist widget state ───────────────────────────────────────────────────
  $effect(() => { localStorage.setItem("widget_open", String(widgetOpen)); });

  // ── Reload data on route change ────────────────────────────────────────────
  $effect(() => {
    const _path = $page.url.pathname;
    const _v    = txState.version;
    let cancelled = false;

    Promise.all([
      transactionApi.getBalance(),
      transactionApi.getPeriodSummary({ type: "Monthly" }),
      transactionApi.list({ page: 1, page_size: WIDGET_RECENT_SIZE }),
      goalApi.list("activo"),
    ]).then(([bal, summary, recent, goals]) => {
      if (cancelled) return;
      balance      = bal.cash_on_hand;
      netWorth     = bal.net_worth;
      monthSummary = summary;
      lastTx       = recent.transactions.find(tx => !tx.note?.startsWith('Auto:') && !tx.note?.startsWith('Externo para')) ?? null;
      nextGoal     = goals.find(g => !g.goal.is_debt_goal) ?? null;
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
        <span class="widget-label">Saldo</span>
        <span
          class="widget-balance"
          class:pos={balance !== null && balance >= 0}
          class:neg={balance !== null && balance < 0}
        >
          {balance === null ? "…" : formatCOP(balance)}
        </span>
        <span class="widget-chevron" class:open={widgetOpen}>›</span>
      </button>

      <!-- Expanded panel -->
      {#if widgetOpen}
        <div class="widget-panel">

          <!-- Saldo en mano / Patrimonio -->
          {#if netWorth !== null && netWorth !== balance}
            <div class="wp-section">
              <div class="wp-label">Saldo en mano</div>
              <div class="wp-row">
                <span class="wp-key">Patrimonio</span>
                <span class="wp-val" class:pos={netWorth >= 0} class:neg={netWorth < 0}>{formatCOP(netWorth)}</span>
              </div>
              <div class="wp-row">
                <span class="wp-key">Préstamos por cobrar</span>
                <span class="wp-val loan">{formatCOP(netWorth - (balance ?? 0))}</span>
              </div>
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
                <span class="wp-val expense">−{formatCOP(monthSummary.total_expenses)}</span>
              </div>
            </div>
          {/if}

          <!-- Último registro -->
          {#if lastTx}
            <div class="wp-divider"></div>
            <div class="wp-section">
              <div class="wp-label">Último registro</div>
              <div class="wp-row">
                <span class="wp-last-cat">{lastTx.category}</span>
                <span
                  class="wp-val"
                  class:income={lastTx.type === "ingreso"}
                  class:expense={lastTx.type === "gasto"}
                >
                  {lastTx.type === "ingreso" ? "+" : "−"}{formatCOP(lastTx.amount)}
                </span>
              </div>
              <div class="wp-meta">{formatDate(lastTx.date)}{lastTx.note ? ` · ${lastTx.note}` : ""}</div>
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
                <span>{formatCOP(nextGoal.goal.target_amount)}</span>
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
    background: #08080f;
    border-right: 1px solid #1a1a2e;
    padding: 1rem 0 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 1rem 0.875rem;
    border-bottom: 1px solid #1a1a2e;
    margin-bottom: 0.5rem;
  }

  .brand-icon { width: 22px; height: 22px; object-fit: contain; flex-shrink: 0; }
  .brand-name { font-size: 0.9rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.01em; }

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
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
    text-decoration: none;
  }
  .nav-item:hover  { background: #1a1a2e; color: var(--text-primary); }
  .nav-item.active { background: color-mix(in srgb, var(--accent) 15%, #1a1a2e); color: var(--accent); }

  /* ═══════════════════════════════════════
     WIDGET
  ═══════════════════════════════════════ */
  .widget {
    flex-shrink: 0;
    border-top: 1px solid #1a1a2e;
    display: flex;
    flex-direction: column;
  }

  /* Toggle button (always visible) */
  .widget-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.65rem 1rem;
    width: 100%;
    text-align: left;
    transition: background 0.15s;
    cursor: pointer;
  }
  .widget-toggle:hover { background: #0e0e1a; }

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
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .widget-balance.pos { color: var(--success); }
  .widget-balance.neg { color: var(--danger); }

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
    animation: slideUp 0.18s ease;
    overflow: hidden;
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
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

  .wp-divider { height: 1px; background: #1a1a2e; margin: 0.25rem 0; }

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
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wp-val.income  { color: var(--success); }
  .wp-val.expense { color: var(--danger); }
  .wp-val.loan    { color: var(--accent); }
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
    height: 4px;
    background: #1a1a2e;
    border-radius: 999px;
    overflow: hidden;
    margin: 0.25rem 0;
  }

  .wp-progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 999px;
    transition: width 0.3s ease;
  }

  .wp-goal-meta {
    display: flex;
    justify-content: space-between;
    font-size: 0.65rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
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
