<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Goal, Transaction } from "$lib/types";

  // ── Estado del formulario ──────────────────────────────────────────────────
  let kind       = $state<"ingreso" | "gasto">("gasto");
  let category   = $state("");
  let amountRaw  = $state("");
  let date       = $state(todayISO());
  let note       = $state("");
  let extraordinary = $state(false);
  let goalId     = $state<number | null>(null);

  // ── Datos cargados ─────────────────────────────────────────────────────────
  let categories = $state<string[]>([]);
  let goals      = $state<Goal[]>([]);
  let loadError  = $state<string | null>(null);

  // ── Feedback ───────────────────────────────────────────────────────────────
  let saving     = $state(false);
  let saved      = $state<Transaction | null>(null);
  let saveError  = $state<string | null>(null);

  let amount = $derived(parseInt(amountRaw.replace(/\D/g, ""), 10) || 0);

  function todayISO(): string {
    return new Date().toISOString().slice(0, 10);
  }

  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function handleAmountInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    if (!digits) { amountRaw = ""; e.currentTarget.value = ""; return; }
    const num = parseInt(digits, 10);
    amountRaw = digits;
    e.currentTarget.value = new Intl.NumberFormat("es-CO").format(num);
  }

  // Recargar categorías cuando cambia el tipo
  $effect(() => {
    const k = kind;
    let cancelled = false;

    async function loadCats() {
      try {
        const data = await invoke<string[]>("list_categories", { kind: k });
        if (!cancelled) {
          categories = data;
          if (!data.includes(category)) category = data[0] ?? "";
        }
      } catch (e) {
        if (!cancelled) loadError = JSON.stringify(e);
      }
    }

    loadCats();
    return () => { cancelled = true; };
  });

  // Cargar objetivos una vez
  $effect(() => {
    invoke<Goal[]>("list_active_goals")
      .then((g) => { goals = g; })
      .catch(() => {});
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (amount <= 0) { saveError = "El monto debe ser mayor que 0."; return; }
    if (!category)   { saveError = "Selecciona una categoría."; return; }

    saving    = true;
    saveError = null;
    saved     = null;

    try {
      const tx = await invoke<Transaction>("create_transaction", {
        input: {
          date,
          type: kind,
          category,
          amount,
          note: note.trim() || null,
          is_extraordinary: extraordinary,
          goal_id: goalId,
        },
      });
      saved         = tx;
      amountRaw     = "";
      note          = "";
      extraordinary = false;
      goalId        = null;
      date          = todayISO();
      setTimeout(() => { saved = null; }, 2000);
    } catch (e) {
      saveError = typeof e === "string" ? e : JSON.stringify(e);
    } finally {
      saving = false;
    }
  }
</script>

<main>
  <h1>Registrar</h1>

  {#if loadError}
    <div class="banner error"><strong>Error cargando datos</strong><pre>{loadError}</pre></div>
  {/if}

  {#if saved}
    <div class="banner success">
      ✓ {saved.type === "ingreso" ? "Ingreso" : "Gasto"} de {formatCOP(saved.amount)} guardado
    </div>
  {/if}

  {#if saveError}
    <div class="banner error"><strong>Error al guardar</strong><pre>{saveError}</pre></div>
  {/if}

  <form onsubmit={handleSubmit} class="form">

    <!-- Toggle tipo -->
    <div class="type-toggle">
      <button
        type="button"
        class="toggle-btn income"
        class:active={kind === "ingreso"}
        onclick={() => { kind = "ingreso"; }}
      >
        Ingreso
      </button>
      <button
        type="button"
        class="toggle-btn expense"
        class:active={kind === "gasto"}
        onclick={() => { kind = "gasto"; }}
      >
        Gasto
      </button>
    </div>

    <!-- Categoría -->
    <div class="field">
      <label for="category">Categoría</label>
      <select id="category" bind:value={category}>
        {#each categories as cat}
          <option value={cat}>{cat}</option>
        {/each}
      </select>
    </div>

    <!-- Monto -->
    <div class="field">
      <label for="amount">Monto</label>
      <input
        id="amount"
        type="text"
        inputmode="numeric"
        placeholder="0"
        value={amountRaw ? new Intl.NumberFormat("es-CO").format(amount) : ""}
        oninput={handleAmountInput}
      />
      {#if amount > 0}
        <span class="field-hint">{formatCOP(amount)}</span>
      {/if}
    </div>

    <!-- Fecha -->
    <div class="field">
      <label for="date">Fecha</label>
      <input id="date" type="date" bind:value={date} max={todayISO()} />
    </div>

    <!-- Nota -->
    <div class="field">
      <label for="note">Nota <span class="optional">(opcional)</span></label>
      <input id="note" type="text" bind:value={note} placeholder="Descripción breve…" maxlength="200" />
    </div>

    <!-- Extraordinario -->
    <label class="checkbox-row">
      <input type="checkbox" bind:checked={extraordinary} />
      <span>Gasto/ingreso extraordinario</span>
      <span class="tooltip" title="Evento no recurrente que no forma parte del presupuesto mensual habitual">?</span>
    </label>

    <!-- Objetivo (solo si hay activos) -->
    {#if goals.length > 0}
      <div class="field">
        <label for="goal">Objetivo asociado <span class="optional">(opcional)</span></label>
        <select id="goal" bind:value={goalId}>
          <option value={null}>— Ninguno —</option>
          {#each goals as g}
            <option value={g.id}>{g.name}</option>
          {/each}
        </select>
      </div>
    {/if}

    <button type="submit" class="submit-btn" disabled={saving || amount <= 0}>
      {saving ? "Guardando…" : "Guardar"}
    </button>

  </form>
</main>

<style>
  main {
    max-width: 480px;
    margin: 0 auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  /* ── Banners ── */
  .banner {
    border-radius: var(--radius);
    padding: 0.65rem 1rem;
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }

  .banner.success {
    background: color-mix(in srgb, var(--success) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--success) 40%, transparent);
    color: var(--success);
    font-weight: 500;
  }

  .banner pre { font-size: 0.72rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }

  /* ── Formulario ── */
  .form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  /* ── Toggle tipo ── */
  .type-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 4px;
    gap: 4px;
  }

  .toggle-btn {
    padding: 0.6rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn.income.active { background: color-mix(in srgb, var(--success) 20%, var(--bg-surface)); color: var(--success); }
  .toggle-btn.expense.active { background: color-mix(in srgb, var(--danger) 20%, var(--bg-surface)); color: var(--danger); }
  .toggle-btn:not(.active):hover { color: var(--text-primary); }

  /* ── Campos ── */
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  label {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .optional { font-weight: 400; color: var(--text-muted); }

  input[type="text"],
  input[type="date"],
  select {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.9rem;
    padding: 0.55rem 0.75rem;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
  }

  input:focus, select:focus { border-color: var(--accent); }

  input[type="date"]::-webkit-calendar-picker-indicator { filter: invert(0.6); }

  .field-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* ── Checkbox ── */
  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .checkbox-row input { width: 15px; height: 15px; accent-color: var(--accent); cursor: pointer; }

  .tooltip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    font-size: 0.65rem;
    color: var(--text-muted);
    cursor: help;
  }

  /* ── Botón guardar ── */
  .submit-btn {
    margin-top: 0.25rem;
    padding: 0.7rem;
    background: var(--accent);
    color: #fff;
    font-size: 0.9rem;
    font-weight: 600;
    border-radius: var(--radius);
    transition: background 0.15s, opacity 0.15s;
  }

  .submit-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .submit-btn:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
