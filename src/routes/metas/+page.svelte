<script lang="ts">
  import { metaApi, loanApi, goalApi } from "$lib/api";
  import type { Meta } from "$lib/types";
  import type { StatEntry, PaymentItem } from "$lib/components/PaymentModal.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";
  import PaymentModal from "$lib/components/PaymentModal.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import DatePicker from "$lib/components/DatePicker.svelte";

  let metas     = $state<Meta[]>([]);
  let loading   = $state(true);
  let pageError = $state<string | null>(null);
  let detail    = $state<Meta | null>(null);

  let filterTipo   = $state("todas");
  let filterEstado = $state("todos");

  let tipoFiltered = $derived(
    filterTipo === "todas" ? metas : metas.filter(m => m.tipo === filterTipo)
  );

  // Pendientes por sub-sección (orden fijo: Deudas → Préstamos → Ahorros)
  let pendingDebts   = $derived(tipoFiltered.filter(m => m.tipo === "debo"          && m.estado === "pendiente"));
  let pendingLoans   = $derived(tipoFiltered.filter(m => m.tipo === "me_deben"      && m.estado === "pendiente"));
  let pendingSavings = $derived(tipoFiltered.filter(m => m.tipo === "quiero_juntar" && m.estado === "pendiente"));
  let doneMetas      = $derived(tipoFiltered.filter(m => m.estado === "completado"));

  let allPendingCount = $derived(pendingDebts.length + pendingLoans.length + pendingSavings.length);

  // Visibilidad de secciones principales según filtro de estado
  let showPending = $derived(filterEstado !== "completado");
  let showDone    = $derived(filterEstado !== "pendiente");

  let visibleCount = $derived(
    (showPending ? allPendingCount : 0) + (showDone ? doneMetas.length : 0)
  );

  // ── Helpers ───────────────────────────────────────────────────────────────
  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function pct(m: Meta): number {
    return m.total > 0 ? Math.min((m.abonado / m.total) * 100, 100) : 0;
  }

  function tipoLabel(tipo: string): string {
    if (tipo === "me_deben")      return "Préstamos";
    if (tipo === "debo")          return "Deudas";
    if (tipo === "quiero_juntar") return "Ahorros";
    return tipo;
  }

  function pendingLabel(tipo: string): string {
    if (tipo === "me_deben") return "por cobrar";
    if (tipo === "debo")     return "por pagar";
    return "por juntar";
  }

  function tipoAccent(tipo: string): string {
    if (tipo === "me_deben")      return "#8e8abd";   /* lavanda apagada  */
    if (tipo === "debo")          return "#a99060";   /* ámbar cálido     */
    if (tipo === "quiero_juntar") return "#5fa386";   /* salvia/teal suave */
    return "#8e8abd";
  }

  function buildStats(m: Meta): StatEntry[] {
    const done  = m.estado === "completado";
    const stats: StatEntry[] = [];
    if (m.tipo === "me_deben") {
      stats.push({ label: "Prestado",   value: formatCOP(m.total) });
      stats.push({ label: "Cobrado",    value: formatCOP(m.abonado),   colorClass: "success" });
      stats.push({ label: "Por cobrar", value: formatCOP(m.pendiente), colorClass: done ? undefined : "accent" });
    } else if (m.tipo === "debo") {
      stats.push({ label: "Deuda",     value: formatCOP(m.total) });
      stats.push({ label: "Pagado",    value: formatCOP(m.abonado),   colorClass: "success" });
      stats.push({ label: "Por pagar", value: formatCOP(m.pendiente), colorClass: done ? undefined : "accent" });
    } else {
      stats.push({ label: "Objetivo", value: formatCOP(m.total) });
      stats.push({ label: "Ahorrado", value: formatCOP(m.abonado),   colorClass: "success" });
      stats.push({ label: "Restante", value: formatCOP(m.pendiente), colorClass: done ? undefined : "accent" });
    }
    if (m.fecha) stats.push({ label: "Fecha", value: m.fecha });
    if (m.cuotas !== null && m.cuotas > 0 && m.tipo === "debo") {
      stats.push({ label: "Cuotas", value: `${m.cuotas} cuotas` });
    }
    return stats;
  }

  function toPaymentItems(m: Meta): PaymentItem[] {
    return m.abonos.map(a => ({ id: a.id, date: a.date, amount: a.amount }));
  }

  // ── Crear ─────────────────────────────────────────────────────────────────
  const tipoOptions = [
    { value: "me_deben",      label: "Préstamos" },
    { value: "debo",          label: "Deudas"    },
    { value: "quiero_juntar", label: "Ahorros"   },
  ];

  let createOpen  = $state(false);
  let creating    = $state(false);
  let cTipo       = $state("");
  let cNombre     = $state("");
  let cAmountRaw  = $state("");
  let cDate       = $state("");
  let cTargetDate = $state("");
  let cNota       = $state("");
  let cError      = $state<string | null>(null);
  let cAmount     = $derived(parseInt(cAmountRaw.replace(/\D/g, ""), 10) || 0);

  function handleAmountInput(
    e: Event & { currentTarget: HTMLInputElement },
    setter: (v: string) => void
  ) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    setter(digits);
    e.currentTarget.value = digits
      ? new Intl.NumberFormat("es-CO").format(parseInt(digits, 10))
      : "";
  }

  function extractMsg(e: unknown): string {
    if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
    return "Error desconocido. Intenta de nuevo.";
  }

  function resetCreate() {
    cTipo = ""; cNombre = ""; cAmountRaw = ""; cDate = ""; cTargetDate = ""; cNota = ""; cError = null;
  }

  async function handleCreate(ev: Event) {
    ev.preventDefault();
    if (!cTipo)          { cError = "Selecciona el tipo de meta."; return; }
    if (!cNombre.trim()) { cError = "El nombre no puede estar vacío."; return; }
    if (cAmount <= 0)    { cError = "El monto debe ser mayor que 0."; return; }
    if (cTipo === "me_deben" && !cDate) { cError = "La fecha del préstamo es requerida."; return; }
    creating = true; cError = null;
    try {
      if (cTipo === "me_deben") {
        await loanApi.create({
          person_name: cNombre.trim(),
          amount: cAmount,
          date: cDate,
          note: cNota.trim() || null,
        });
      } else {
        await goalApi.create({
          name: cNombre.trim(),
          target_amount: cAmount,
          target_date: cTargetDate || null,
          status: "activo",
        });
      }
      await loadMetas();
      createOpen = false;
      resetCreate();
    } catch (e) {
      console.error("[metas] create:", e);
      cError = extractMsg(e);
    } finally {
      creating = false;
    }
  }

  // ── Carga ─────────────────────────────────────────────────────────────────
  async function loadMetas() {
    loading = true; pageError = null;
    try {
      metas = await metaApi.list();
    } catch (e) {
      console.error("[metas] load:", e);
      pageError = "No se pudieron cargar las metas.";
    } finally {
      loading = false;
    }
  }

  $effect(() => { loadMetas(); });

  // ── Abono ─────────────────────────────────────────────────────────────────
  async function handleAddPayment(amount: number, date: string) {
    if (!detail) return;
    const metaId = detail.id;
    const [prefix, rawId] = metaId.split(":");
    const numId = parseInt(rawId, 10);
    if (prefix === "loan") {
      await loanApi.addPayment({ loan_id: numId, amount, date });
    } else {
      await goalApi.addContribution(numId, amount, date);
    }
    await loadMetas();
    detail = metas.find(m => m.id === metaId) ?? null;
  }
</script>

<main>
  <div class="header">
    <h1>Metas</h1>
    <button class="btn-primary" onclick={() => { createOpen = true; }}>+ Nueva</button>
  </div>

  {#if pageError}
    <div class="banner error"><strong>Error</strong><pre>{pageError}</pre></div>
  {/if}

  <div class="filter-row">
    {#each [["todas", "Todas"], ["me_deben", "Préstamos"], ["debo", "Deudas"], ["quiero_juntar", "Ahorros"]] as [val, lbl]}
      <button
        class="filter-btn"
        class:active={filterTipo === val}
        onclick={() => { filterTipo = val; }}
      >{lbl}</button>
    {/each}
    <div class="filter-sep"></div>
    {#each [["todos", "Todos"], ["pendiente", "Pendientes"], ["completado", "Completadas"]] as [val, lbl]}
      <button
        class="filter-btn secondary"
        class:active={filterEstado === val}
        onclick={() => { filterEstado = val; }}
      >{lbl}</button>
    {/each}
  </div>

  {#if loading}
    <p class="muted">Cargando…</p>
  {:else if metas.length === 0}
    <div class="empty">
      <p>Sin metas registradas.</p>
      <p class="muted">Crea préstamos u objetivos para verlos aquí.</p>
    </div>
  {:else if visibleCount === 0}
    <p class="muted">Sin metas con ese filtro.</p>
  {:else}
    <ScrollArea class="metas-scroll" scrollbar="thin">

      <!-- ── Sección PENDIENTES ── -->
      {#if showPending && allPendingCount > 0}
        <div class="section-label">Pendientes</div>

        {#if pendingDebts.length > 0}
          <div class="subsection-label subsection-debo">Deudas</div>
          <div class="meta-grid">
            {#each pendingDebts as m (m.id)}
              {@render metaCard(m, false, false)}
            {/each}
          </div>
        {/if}

        {#if pendingLoans.length > 0}
          <div class="subsection-label subsection-me_deben">Préstamos</div>
          <div class="meta-grid">
            {#each pendingLoans as m (m.id)}
              {@render metaCard(m, false, false)}
            {/each}
          </div>
        {/if}

        {#if pendingSavings.length > 0}
          <div class="subsection-label subsection-quiero_juntar">Ahorros</div>
          <div class="meta-grid">
            {#each pendingSavings as m (m.id)}
              {@render metaCard(m, false, false)}
            {/each}
          </div>
        {/if}
      {/if}

      <!-- ── Sección COMPLETADAS ── -->
      {#if showDone && doneMetas.length > 0}
        {#if showPending && allPendingCount > 0}
          <div class="section-divider"></div>
        {/if}
        <div class="section-label secondary">Completadas</div>
        <div class="meta-grid dimmed">
          {#each doneMetas as m (m.id)}
            {@render metaCard(m, true, true)}
          {/each}
        </div>
      {/if}

    </ScrollArea>
  {/if}
</main>

{#snippet metaCard(m: Meta, dimmed: boolean, showBadge: boolean)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="meta-card" class:dimmed onclick={() => { detail = m; }}>
    <div class="card-top">
      <span class="meta-name">{m.nombre}</span>
      {#if showBadge}
        <span class="tipo-badge tipo-{m.tipo}">{tipoLabel(m.tipo)}</span>
      {/if}
    </div>

    <div class="pending-amount">
      <span class="pending-value" style="color: {tipoAccent(m.tipo)}">{formatCOP(m.pendiente)}</span>
      <span class="pending-label">{pendingLabel(m.tipo)}</span>
    </div>

    <div class="progress-wrap">
      <div class="progress-bar">
        <div
          class="progress-fill"
          style="width: {pct(m)}%; background: {m.estado === 'completado' ? 'var(--success)' : tipoAccent(m.tipo)}"
        ></div>
      </div>
      <span class="pct-text">{pct(m).toFixed(0)}%</span>
    </div>

    <div class="amounts">
      <span class="paid-label">Abonado</span>
      <span class="paid-value">{formatCOP(m.abonado)}</span>
      <span class="sep">/</span>
      <span class="total-value">{formatCOP(m.total)}</span>
    </div>

    {#if m.cuotas !== null && m.cuotas > 0 && m.tipo === "debo"}
      <div class="cuotas-hint">≈ {formatCOP(Math.ceil(m.total / m.cuotas))}/mes · {m.cuotas} cuotas</div>
    {/if}

    <div class="card-footer">
      <span class="meta-date">{m.fecha ?? "Sin fecha"}</span>
    </div>
  </div>
{/snippet}

<!-- ── Modal: Nueva Meta ─────────────────────────────────────────────────── -->
{#if createOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { createOpen = false; resetCreate(); }}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <ScrollArea class="modal-scroll" scrollbar="thin">
      <h2>Nueva meta</h2>

      {#if cError}
        <div class="banner error small"><pre>{cError}</pre></div>
      {/if}

      <form onsubmit={handleCreate} class="modal-form">

        <div class="field">
          <span class="field-label">Tipo</span>
          <CustomSelect
            bind:value={cTipo}
            options={tipoOptions}
            placeholder="Selecciona el tipo…"
            onchange={() => { cError = null; }}
          />
        </div>

        {#if cTipo === "debo"}
          <div class="deuda-info">
            <p>Las deudas se registran automáticamente al agregar un gasto en la pestaña <strong>Registrar</strong>. Activa la opción "¿Es deuda?" en el formulario de gasto.</p>
          </div>
        {:else if cTipo}
          <div class="field">
            <label for="c-nombre">
              {cTipo === "me_deben" ? "A quién prestaste" : "Nombre del objetivo"}
            </label>
            <input
              id="c-nombre"
              type="text"
              bind:value={cNombre}
              placeholder={cTipo === "me_deben" ? "Ej: Juan, María…" : "Ej: Viaje, Laptop…"}
              maxlength="100"
            />
          </div>

          <div class="field">
            <label for="c-amount">Monto</label>
            <input
              id="c-amount"
              type="text"
              inputmode="numeric"
              placeholder="0"
              value={cAmountRaw ? new Intl.NumberFormat("es-CO").format(cAmount) : ""}
              oninput={(e) => handleAmountInput(e, (v) => { cAmountRaw = v; })}
            />
          </div>

          {#if cTipo === "me_deben"}
            <div class="field">
              <span class="field-label">Fecha del préstamo</span>
              <DatePicker bind:value={cDate} />
            </div>
          {:else}
            <div class="field">
              <span class="field-label">Fecha meta <span class="optional">(opcional)</span></span>
              <DatePicker bind:value={cTargetDate} />
            </div>
          {/if}

          <div class="field">
            <label for="c-nota">Nota <span class="optional">(opcional)</span></label>
            <input
              id="c-nota"
              type="text"
              bind:value={cNota}
              placeholder="Para qué es…"
              maxlength="200"
            />
          </div>
        {/if}

        <div class="modal-actions">
          <button
            type="button"
            class="btn-secondary"
            onclick={() => { createOpen = false; resetCreate(); }}
          >Cancelar</button>
          <button
            type="submit"
            class="btn-primary"
            disabled={creating || !cTipo || cTipo === "debo" || !cNombre.trim() || cAmount <= 0 || (cTipo === "me_deben" && !cDate)}
          >
            {creating ? "Creando…" : "Crear"}
          </button>
        </div>

      </form>
    </ScrollArea>
  </div>
  </div>
{/if}

{#if detail}
  <PaymentModal
    title={detail.nombre}
    subtitle={tipoLabel(detail.tipo)}
    subtitleClass="tipo-badge tipo-{detail.tipo}"
    note={detail.nota}
    stats={buildStats(detail)}
    paid={detail.abonado}
    total={detail.total}
    progressDone={detail.estado === "completado"}
    items={toPaymentItems(detail)}
    itemsLabel="Abonos"
    canPay={detail.estado === "pendiente"}
    onAddPayment={handleAddPayment}
    onClose={() => { detail = null; }}
  />
{/if}

<style>
  main {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 1rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h1 { font-size: 1.1rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.02em; }

  /* ── Filtros ── */
  .filter-row {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .filter-sep {
    width: 1px;
    height: 16px;
    background: var(--border);
    margin: 0 0.2rem;
  }

  .filter-btn {
    padding: 0.3rem 0.75rem;
    border-radius: 999px;
    font-size: 0.78rem;
    font-weight: 500;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid transparent;
    transition: all 0.15s;
  }
  .filter-btn.secondary {
    font-size: 0.73rem;
    padding: 0.25rem 0.65rem;
  }
  .filter-btn.active {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-elevated));
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  /* ── Sección principal ── */
  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    padding: 0.5rem 0.25rem 0.25rem;
  }
  .section-label.secondary { color: var(--text-muted); }

  /* ── Sub-sección ── */
  .subsection-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 0.5rem 0.25rem 0.3rem 0.75rem;
    border-left: 2px solid var(--border);
    margin-left: 0.1rem;
    margin-top: 0.25rem;
  }

  /* ── Divisor entre PENDIENTES y COMPLETADAS ── */
  .section-divider {
    height: 1px;
    background: var(--border);
    margin: 1.25rem 0.25rem 0;
  }

  :global(.metas-scroll) { flex: 1; min-height: 0; }

  /* ── Grid ── */
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 1rem;
    align-content: start;
    padding: 0.25rem;
  }
  .meta-grid.dimmed { opacity: 0.55; }

  /* ── Card ── */
  .meta-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1rem;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    transition: border-color 0.15s;
  }
  .meta-card:hover { border-color: var(--accent); }
  .meta-card.dimmed:hover { border-color: var(--border); }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .meta-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  /* ── Tipo badge (solo en tarjetas de Completadas) ── */
  .tipo-badge {
    font-size: 0.65rem;
    font-weight: 600;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tipo-me_deben      { background: rgba(142, 138, 189, 0.12); color: #8e8abd; }
  .tipo-debo          { background: rgba(169, 144,  96, 0.12); color: #a99060; }
  .tipo-quiero_juntar { background: rgba( 95, 163, 134, 0.12); color: #5fa386; }

  /* ── Pending amount ── */
  .pending-amount { display: flex; align-items: baseline; gap: 0.4rem; }
  .pending-value  { font-size: 1.1rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  .pending-label  { font-size: 0.72rem; color: var(--text-muted); }

  /* ── Progress ── */
  .progress-wrap { display: flex; align-items: center; gap: 0.5rem; }
  .progress-bar  { flex: 1; height: 6px; background: var(--bg-elevated); border-radius: 999px; overflow: hidden; }
  .progress-fill { height: 100%; border-radius: 999px; transition: width 0.3s ease; }
  .pct-text      { font-size: 0.72rem; color: var(--text-muted); min-width: 2.5rem; text-align: right; }

  /* ── Amounts row ── */
  .amounts { display: flex; align-items: baseline; gap: 0.3rem; font-size: 0.8rem; }
  .paid-label  { color: var(--text-muted); }
  .paid-value  { color: var(--text-secondary); font-weight: 500; }
  .sep         { color: var(--border); }
  .total-value { color: var(--text-muted); }

  /* ── Cuotas hint ── */
  .cuotas-hint {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  /* ── Card footer ── */
  .card-footer { display: flex; align-items: center; justify-content: space-between; }
  .meta-date   { font-size: 0.72rem; color: var(--text-muted); }

  /* ── Banner ── */
  .banner { border-radius: var(--radius); padding: 0.65rem 1rem; font-size: 0.85rem; }
  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }
  .banner pre { font-size: 0.72rem; white-space: pre-wrap; word-break: break-all; }

  /* ── Botones ── */
  .btn-primary {
    padding: 0.45rem 1rem; background: var(--accent); color: #fff;
    font-size: 0.85rem; font-weight: 600; border-radius: var(--radius);
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-secondary {
    padding: 0.45rem 1rem; background: var(--bg-elevated); color: var(--text-secondary);
    font-size: 0.85rem; font-weight: 500; border-radius: var(--radius);
    border: 1px solid var(--border); transition: background 0.15s;
  }
  .btn-secondary:hover { background: var(--bg-surface); }

  /* ── Overlay / Modal ── */
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 20;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 1.5rem;
    width: min(440px, 92vw); max-height: 85vh; overflow: hidden;
    display: flex; flex-direction: column;
  }
  :global(.modal-scroll) { flex: 1; min-height: 0; }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); margin-bottom: 0.75rem; }

  /* ── Formulario ── */
  .modal-form    { display: flex; flex-direction: column; gap: 0.9rem; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

  .field { display: flex; flex-direction: column; gap: 0.3rem; }
  label, .field-label { font-size: 0.78rem; font-weight: 500; color: var(--text-secondary); }
  .optional { font-weight: 400; color: var(--text-muted); }

  input[type="text"] {
    -webkit-appearance: none; appearance: none;
    background-color: #14141f; border: 1px solid #2a2a40;
    border-radius: var(--radius); color: #e8e8f0; font: inherit;
    font-size: 0.9rem; padding: 0.5rem 0.75rem; outline: none;
    transition: border-color 0.15s; width: 100%;
  }
  input:focus { border-color: var(--accent); }

  .hint {
    font-size: 0.72rem; color: var(--text-muted);
    line-height: 1.4; margin-top: 0.1rem;
  }

  .deuda-info {
    background: color-mix(in srgb, var(--bg-elevated) 80%, var(--warning) 20%);
    border: 1px solid color-mix(in srgb, var(--warning) 25%, transparent);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
  }
  .deuda-info p {
    font-size: 0.8rem; color: var(--text-secondary); line-height: 1.5; margin: 0;
  }
  .deuda-info strong { color: var(--text-primary); font-weight: 600; }

  /* ── Misc ── */
  .muted { color: var(--text-muted); font-size: 0.85rem; }
  .empty {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 0.5rem; padding: 2rem;
    color: var(--text-secondary); font-size: 0.9rem;
  }
</style>
