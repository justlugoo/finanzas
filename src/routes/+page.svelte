<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import "../app.css";
  import type { Budget } from "$lib/types";

  let budgets: Budget[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    let cancelled = false;

    async function load() {
      while (!cancelled) {
        try {
          const data = await invoke<Budget[]>("list_budgets");
          if (!cancelled) {
            budgets = data;
            loading = false;
          }
          return;
        } catch (e: unknown) {
          const err = e as { kind?: string; message?: string };
          // DB todavía inicializando: reintentar hasta que esté lista
          if (err?.kind === "DatabaseError" && err?.message?.includes("no inicializada")) {
            await new Promise((r) => setTimeout(r, 300));
            continue;
          }
          if (!cancelled) {
            error = JSON.stringify(e);
            loading = false;
          }
          return;
        }
      }
    }

    load();
    return () => { cancelled = true; };
  });

  function formatCOP(amount: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency",
      currency: "COP",
      minimumFractionDigits: 0,
    }).format(amount);
  }
</script>

<main>
  <header>
    <h1>Finanzas</h1>
    <p class="subtitle">D-1 — Conexión Turso</p>
  </header>

  {#if loading}
    <div class="state">
      <span class="dot loading"></span>
      Conectando con Turso…
    </div>
  {:else if error}
    <div class="state error">
      <strong>Error al conectar</strong>
      <pre>{error}</pre>
    </div>
  {:else}
    <section>
      <h2>Presupuestos ({budgets.length} categorías)</h2>
      <table>
        <thead>
          <tr>
            <th>Categoría</th>
            <th>Meta mensual</th>
          </tr>
        </thead>
        <tbody>
          {#each budgets as b}
            <tr>
              <td>{b.category}</td>
              <td class="amount">{b.monthly_amount === 0 ? "—" : formatCOP(b.monthly_amount)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 640px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
  }

  header {
    margin-bottom: 2rem;
  }

  h1 {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: var(--bg-surface);
    border-radius: var(--radius);
    color: var(--text-secondary);
  }

  .state.error {
    color: var(--danger);
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .state.error pre {
    font-size: 0.75rem;
    opacity: 0.7;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    flex-shrink: 0;
  }

  .dot.loading {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  table {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-surface);
    border-radius: var(--radius);
    overflow: hidden;
  }

  th {
    text-align: left;
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
  }

  tr:last-child td {
    border-bottom: none;
  }

  tr:hover td {
    background: var(--bg-elevated);
  }

  .amount {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
</style>
