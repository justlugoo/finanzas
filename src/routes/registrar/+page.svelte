<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Goal, Transaction, RoutesCost } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";

  // ── Estado del formulario ──────────────────────────────────────────────────
  let kind       = $state<"ingreso" | "gasto">("gasto");
  let category   = $state("");
  let amountRaw  = $state("");
  let date       = $state(todayISO());
  let note       = $state("");
  let extraordinary = $state(false);
  let goalId     = $state<number | null>(null);

  // ── Carrera: sub-selector de persona ──────────────────────────────────────
  let carreraPersona   = $state<"mama" | "cunada" | "otra" | null>(null);
  let carreraOtraKmRaw = $state("");
  let carreraOtraKm    = $derived(parseFloat(carreraOtraKmRaw) || 0);

  // ── Gasolina adicional (para todo lo que no sea Carrera con gas auto) ─────
  let gasKmRaw   = $state("");
  let gasKm      = $derived(parseFloat(gasKmRaw) || 0);
  let savedGasKm = $state(0);

  // ── Datos cargados ─────────────────────────────────────────────────────────
  let categories = $state<string[]>([]);
  let goals      = $state<Goal[]>([]);
  let routeCosts = $state<RoutesCost | null>(null);
  let loadError  = $state<string | null>(null);

  // ── Feedback ───────────────────────────────────────────────────────────────
  let saving    = $state(false);
  let saved     = $state<Transaction | null>(null);
  let saveError = $state<string | null>(null);

  let amount = $derived(parseInt(amountRaw.replace(/\D/g, ""), 10) || 0);

  // Categorías visibles en el dropdown (ocultar variantes individuales de Carrera en ingreso)
  let displayCategories = $derived(
    kind === "ingreso"
      ? categories.filter(c => c !== "Carrera mamá" && c !== "Carrera cuñada")
      : categories,
  );

  // Categoría efectiva enviada al backend
  let effectiveCategory = $derived(
    category === "Carrera" && carreraPersona === "mama"   ? "Carrera mamá"  :
    category === "Carrera" && carreraPersona === "cunada" ? "Carrera cuñada" :
    category
  );

  // Costo de gasolina calculado para Carrera otra persona
  let carreraOtraGasCost = $derived(
    routeCosts && carreraOtraKm > 0
      ? Math.round(carreraOtraKm / routeCosts.consumo_km_galon * routeCosts.precio_galon)
      : 0
  );

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
          const filtered = k === "ingreso"
            ? data.filter(c => c !== "Carrera mamá" && c !== "Carrera cuñada")
            : data;
          if (!filtered.includes(category)) category = filtered[0] ?? "";
        }
      } catch (e) {
        if (!cancelled) {
          console.error("[registrar] load categories error:", e);
          loadError = "Error cargando categorías. Recarga la app.";
        }
      }
    }

    loadCats();
    return () => { cancelled = true; };
  });

  // Resetear sub-selector al cambiar categoría o tipo
  $effect(() => {
    const k = kind;
    const c = category;
    if (k !== "ingreso" || c !== "Carrera") {
      carreraPersona   = null;
      carreraOtraKmRaw = "";
    }
  });

  // Cargar objetivos y costos de ruta una vez
  $effect(() => {
    invoke<Goal[]>("list_active_goals")
      .then((g) => { goals = g; })
      .catch(() => {});
    invoke<RoutesCost>("get_route_costs")
      .then((r) => { routeCosts = r; })
      .catch(() => {});
  });

  function selectGasPreset(km: number) {
    const str = km.toString();
    gasKmRaw = gasKmRaw === str ? "" : str;
  }

  function gasHintCost(): number {
    if (!routeCosts || gasKm <= 0) return 0;
    return Math.round(gasKm / routeCosts.consumo_km_galon * routeCosts.precio_galon);
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (amount <= 0) { saveError = "El monto debe ser mayor que 0."; return; }
    if (!category)   { saveError = "Selecciona una categoría."; return; }
    if (kind === "ingreso" && category === "Carrera" && !carreraPersona) {
      saveError = "Selecciona a quién fue la carrera."; return;
    }

    saving    = true;
    saveError = null;
    saved     = null;

    // gas_km: null para Carrera mamá/cuñada (el backend lo calcula desde config)
    let gasKmToSend: number | null = null;
    if (effectiveCategory !== "Carrera mamá" && effectiveCategory !== "Carrera cuñada") {
      if (category === "Carrera" && carreraPersona === "otra") {
        gasKmToSend = carreraOtraKm > 0 ? carreraOtraKm : null;
      } else {
        gasKmToSend = gasKm > 0 ? gasKm : null;
      }
    }

    try {
      const tx = await invoke<Transaction>("create_transaction", {
        input: {
          date,
          type: kind,
          category: effectiveCategory,
          amount,
          note: note.trim() || null,
          is_extraordinary: extraordinary,
          goal_id: goalId,
          gas_km: gasKmToSend,
        },
      });

      savedGasKm = (category === "Carrera" && carreraPersona === "otra") ? carreraOtraKm : gasKm;
      saved         = tx;
      amountRaw     = "";
      note          = "";
      extraordinary = false;
      goalId        = null;
      gasKmRaw      = "";
      carreraPersona   = null;
      carreraOtraKmRaw = "";
      date          = todayISO();
      setTimeout(() => { saved = null; savedGasKm = 0; }, 3000);
    } catch (e) {
      console.error("[registrar] save error:", e);
      saveError = "No se pudo guardar. Intenta de nuevo.";
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
      {#if saved.type === "ingreso" && (saved.category === "Carrera mamá" || saved.category === "Carrera cuñada") && routeCosts}
        <span class="auto-gas-note">
          + Gasolina: {formatCOP(saved.category === "Carrera mamá" ? routeCosts.carrera_mama : routeCosts.carrera_cunada)}
        </span>
      {:else if savedGasKm > 0 && routeCosts}
        <span class="auto-gas-note">
          + Gasolina: {formatCOP(Math.round(savedGasKm / routeCosts.consumo_km_galon * routeCosts.precio_galon))}
        </span>
      {/if}
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
        onclick={() => { kind = "ingreso"; goalId = null; }}
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
        {#each displayCategories as cat}
          <option value={cat}>{cat}</option>
        {/each}
      </select>
    </div>

    <!-- Sub-selector para Carrera ingreso -->
    {#if kind === "ingreso" && category === "Carrera"}
      <div class="field carrera-field">
        <label>¿A quién fue la carrera?</label>
        <div class="persona-row">
          <button
            type="button"
            class="persona-btn"
            class:active={carreraPersona === "mama"}
            onclick={() => carreraPersona = "mama"}
          >Mamá</button>
          <button
            type="button"
            class="persona-btn"
            class:active={carreraPersona === "cunada"}
            onclick={() => carreraPersona = "cunada"}
          >Cuñada</button>
          <button
            type="button"
            class="persona-btn"
            class:active={carreraPersona === "otra"}
            onclick={() => carreraPersona = "otra"}
          >Otra persona</button>
        </div>

        <!-- Gas auto (Mamá / Cuñada) — solo lectura -->
        {#if (carreraPersona === "mama" || carreraPersona === "cunada") && routeCosts}
          <div class="carrera-gas-info">
            <span class="gas-auto-label">Gasolina auto</span>
            <span class="gas-auto-value">
              {formatCOP(carreraPersona === "mama" ? routeCosts.carrera_mama : routeCosts.carrera_cunada)}
              <span class="gas-km-badge">
                {(carreraPersona === "mama" ? routeCosts.km_carrera_mama : routeCosts.km_carrera_cunada).toFixed(1)} km
              </span>
            </span>
          </div>
        {/if}

        <!-- Km para Otra persona -->
        {#if carreraPersona === "otra" && routeCosts}
          <div class="gas-row">
            <div class="gas-km-input">
              <input
                type="text"
                inputmode="decimal"
                bind:value={carreraOtraKmRaw}
                placeholder="km"
              />
              <span class="km-unit">km</span>
            </div>
            {#if carreraOtraKm > 0}
              <span class="gas-cost-hint">≈ {formatCOP(carreraOtraGasCost)}</span>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Gasolina adicional (para todo lo que no sea Carrera ingreso) -->
    {#if routeCosts && !(kind === "ingreso" && category === "Carrera")}
      <div class="field gas-field">
        <label>Gasolina <span class="optional">(opcional — se agrega como gasto separado)</span></label>
        <div class="gas-row">
          <button
            type="button"
            class="gas-preset-btn"
            class:active={gasKmRaw === routeCosts.km_universidad.toString()}
            onclick={() => selectGasPreset(routeCosts!.km_universidad)}
          >
            Universidad
          </button>
          <div class="gas-km-input">
            <input
              type="text"
              inputmode="decimal"
              bind:value={gasKmRaw}
              placeholder="km"
            />
            <span class="km-unit">km</span>
          </div>
          {#if gasKm > 0}
            <span class="gas-cost-hint">≈ {formatCOP(gasHintCost())}</span>
          {/if}
        </div>
      </div>
    {/if}

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
      <label>Fecha</label>
      <DatePicker bind:value={date} />
    </div>

    <!-- Nota -->
    <div class="field">
      <label for="note">Nota <span class="optional">(opcional)</span></label>
      <input id="note" type="text" bind:value={note} placeholder="Descripción breve…" maxlength="200" />
    </div>

    <!-- Extraordinario -->
    <label class="checkbox-row">
      <input type="checkbox" bind:checked={extraordinary} />
      <span>{kind === "gasto" ? "Gasto" : "Ingreso"} extraordinario</span>
      <span
        class="tooltip"
        data-tooltip="Evento único o no recurrente — no forma parte del presupuesto mensual habitual (ej. un regalo, una emergencia)"
      >?</span>
    </label>

    <!-- Objetivo (solo gastos) -->
    {#if kind === "gasto"}
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

  .auto-gas-note { font-size: 0.78rem; opacity: 0.85; }

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

  .toggle-btn.income.active  { background: color-mix(in srgb, var(--success) 20%, var(--bg-surface)); color: var(--success); }
  .toggle-btn.expense.active { background: color-mix(in srgb, var(--danger)  20%, var(--bg-surface)); color: var(--danger); }
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
  select {
    -webkit-appearance: none;
    appearance: none;
    background-color: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.9rem;
    padding: 0.55rem 2.2rem 0.55rem 0.75rem;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%238888aa' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.6rem center;
    background-size: 1rem;
  }

  input[type="text"] {
    background-image: none;
    padding-right: 0.75rem;
  }

  select option { background-color: #14141f; color: #e8e8f0; }

  input:focus, select:focus { border-color: var(--accent); }

  .field-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* ── Carrera sub-selector ── */
  .carrera-field {
    background: color-mix(in srgb, var(--success) 5%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--success) 20%, transparent);
    border-radius: var(--radius);
    padding: 0.65rem 0.875rem;
    gap: 0.6rem;
  }

  .persona-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .persona-btn {
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.82rem;
    font-weight: 500;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: all 0.15s;
  }

  .persona-btn.active {
    background: color-mix(in srgb, var(--success) 20%, var(--bg-elevated));
    color: var(--success);
    border-color: color-mix(in srgb, var(--success) 50%, transparent);
  }

  .persona-btn:hover:not(.active) { color: var(--text-primary); }

  .carrera-gas-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.15rem;
    padding: 0.35rem 0.5rem;
    background: color-mix(in srgb, var(--success) 8%, var(--bg-elevated));
    border-radius: 6px;
  }

  .gas-auto-label {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .gas-auto-value {
    font-size: 0.82rem;
    color: var(--success);
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .gas-km-badge {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-weight: 400;
  }

  /* ── Gasolina add-on ── */
  .gas-field {
    background: color-mix(in srgb, var(--accent) 6%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--accent) 20%, transparent);
    border-radius: var(--radius);
    padding: 0.65rem 0.875rem;
  }

  .gas-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .gas-preset-btn {
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.78rem;
    font-weight: 500;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: all 0.15s;
    white-space: nowrap;
  }

  .gas-preset-btn.active {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-elevated));
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .gas-preset-btn:hover:not(.active) { color: var(--text-primary); }

  .gas-km-input {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    background: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    padding: 0.35rem 0.6rem;
    transition: border-color 0.15s;
  }

  .gas-km-input:focus-within { border-color: var(--accent); }

  .gas-km-input input {
    width: 56px;
    background: transparent;
    border: none;
    color: #e8e8f0;
    font: inherit;
    font-size: 0.88rem;
    padding: 0;
    outline: none;
    background-image: none;
  }

  .km-unit {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .gas-cost-hint {
    font-size: 0.82rem;
    color: var(--accent);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
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

  /* ── Tooltip CSS (title no funciona en Tauri WebView) ── */
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
    position: relative;
  }

  .tooltip::after {
    content: attr(data-tooltip);
    position: absolute;
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-surface);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.45rem 0.65rem;
    font-size: 0.72rem;
    white-space: normal;
    width: 220px;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s;
    z-index: 100;
    text-align: left;
    line-height: 1.5;
    font-weight: 400;
  }

  .tooltip:hover::after { opacity: 1; }

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
