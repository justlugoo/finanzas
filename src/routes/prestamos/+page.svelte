<script lang="ts">
  import { loanApi } from "$lib/api";
  import type { LoanWithBalance } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";
  import PaymentModal from "$lib/components/PaymentModal.svelte";

  let loans        = $state<LoanWithBalance[]>([]);
  let loading      = $state(true);
  let pageError    = $state<string | null>(null);

  // ── Filtro ────────────────────────────────────────────────────────────────
  let filterStatus = $state<string>("todos");

  let pendingLoans = $derived(loans.filter(l => l.loan.status === "pendiente"));
  let paidLoans    = $derived(loans.filter(l => l.loan.status === "pagado"));
  let filtered     = $derived(
    filterStatus === "pendiente" ? pendingLoans :
    filterStatus === "pagado"    ? paidLoans    :
    loans
  );

  // ── Crear ─────────────────────────────────────────────────────────────────
  let createOpen  = $state(false);
  let creating    = $state(false);
  let cName       = $state("");
  let cAmountRaw  = $state("");
  let cDate       = $state("");
  let cNote       = $state("");
  let cError      = $state<string | null>(null);
  let cAmount     = $derived(parseInt(cAmountRaw.replace(/\D/g, ""), 10) || 0);

  // ── Detalle ───────────────────────────────────────────────────────────────
  let detail        = $state<LoanWithBalance | null>(null);
  let detailLoading = $state(false);

  // ── Abono ─────────────────────────────────────────────────────────────────

  // ── Eliminar ──────────────────────────────────────────────────────────────
  let deleteId  = $state<number | null>(null);
  let deleting  = $state(false);

  // ── Helpers ───────────────────────────────────────────────────────────────
  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

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

  function loanPct(l: LoanWithBalance): number {
    return l.loan.amount > 0 ? Math.min((l.paid / l.loan.amount) * 100, 100) : 0;
  }

  function extractMsg(e: unknown): string {
    if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
    return "Error desconocido. Intenta de nuevo.";
  }

  // ── Carga ─────────────────────────────────────────────────────────────────
  async function loadLoans() {
    loading = true; pageError = null;
    try {
      loans = await loanApi.list();
    } catch (e) {
      console.error("[prestamos] load:", e);
      pageError = "No se pudieron cargar los préstamos.";
    } finally {
      loading = false;
    }
  }

  $effect(() => { loadLoans(); });

  // ── Crear ─────────────────────────────────────────────────────────────────
  async function handleCreate(ev: Event) {
    ev.preventDefault();
    if (!cName.trim()) { cError = "El nombre no puede estar vacío."; return; }
    if (cAmount <= 0)  { cError = "El monto debe ser mayor que 0."; return; }
    if (!cDate)        { cError = "La fecha es requerida."; return; }
    creating = true; cError = null;
    try {
      const loan = await loanApi.create({
        person_name: cName.trim(),
        amount: cAmount,
        date: cDate,
        note: cNote.trim() || null,
      });
      loans = [loan, ...loans];
      createOpen = false; cName = ""; cAmountRaw = ""; cDate = ""; cNote = "";
    } catch (e) {
      console.error("[prestamos] create:", e);
      cError = extractMsg(e);
    } finally {
      creating = false;
    }
  }

  // ── Detalle ───────────────────────────────────────────────────────────────
  async function openDetail(id: number) {
    if (deleteId !== null) return;
    detailLoading = true;
    try {
      detail = await loanApi.get(id);
    } catch (e) {
      console.error("[prestamos] detail:", e);
      pageError = "No se pudo cargar el detalle.";
    } finally {
      detailLoading = false;
    }
  }

  // ── Abono ─────────────────────────────────────────────────────────────────
  async function handleAddPayment(amount: number, date: string) {
    if (!detail) return;
    const updated = await loanApi.addPayment({ loan_id: detail.loan.id, amount, date });
    detail = updated;
    loans = loans.map(l => l.loan.id === updated.loan.id ? updated : l);
  }

  // ── Eliminar ──────────────────────────────────────────────────────────────
  function confirmDelete(id: number, ev: Event) {
    ev.stopPropagation();
    deleteId = id;
  }

  async function handleDelete() {
    if (deleteId === null) return;
    deleting = true;
    try {
      await loanApi.remove(deleteId);
      loans = loans.filter(l => l.loan.id !== deleteId);
      if (detail?.loan.id === deleteId) detail = null;
      deleteId = null;
    } catch (e) {
      console.error("[prestamos] delete:", e);
      pageError = "No se pudo eliminar el préstamo.";
    } finally {
      deleting = false;
    }
  }
</script>

<main>
  <div class="header">
    <h1>Préstamos</h1>
    <button class="btn-primary" onclick={() => { createOpen = true; }}>+ Nuevo</button>
  </div>

  {#if pageError}
    <div class="banner error"><strong>Error</strong><pre>{pageError}</pre></div>
  {/if}

  <div class="filter-row">
    {#each [["todos", "Todos"], ["pendiente", "Pendientes"], ["pagado", "Pagados"]] as [val, lbl]}
      <button
        class="filter-btn"
        class:active={filterStatus === val}
        onclick={() => { filterStatus = val; }}
      >{lbl}</button>
    {/each}
  </div>

  {#if loading}
    <p class="muted">Cargando…</p>
  {:else if loans.length === 0}
    <div class="empty">
      <p>Sin préstamos registrados.</p>
      <button class="btn-primary" onclick={() => { createOpen = true; }}>Registrar el primero</button>
    </div>
  {:else if filtered.length === 0}
    <p class="muted">Sin préstamos con estado "{filterStatus}".</p>
  {:else}
    <ScrollArea class="loans-scroll" scrollbar="thin">
      {#if filterStatus === "todos"}
        {#if pendingLoans.length > 0}
          <div class="section-label">Pendientes</div>
          <div class="loan-grid">
            {#each pendingLoans as l (l.loan.id)}
              {@render loanCard(l, false)}
            {/each}
          </div>
        {/if}
        {#if paidLoans.length > 0}
          <div class="section-label secondary">Pagados</div>
          <div class="loan-grid dimmed">
            {#each paidLoans as l (l.loan.id)}
              {@render loanCard(l, true)}
            {/each}
          </div>
        {/if}
      {:else}
        <div class="loan-grid">
          {#each filtered as l (l.loan.id)}
            {@render loanCard(l, false)}
          {/each}
        </div>
      {/if}
    </ScrollArea>
  {/if}
</main>

{#snippet loanCard(l: LoanWithBalance, dimmed: boolean)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="loan-card" class:dimmed onclick={() => openDetail(l.loan.id)}>
    <div class="card-top">
      <span class="person-name">{l.loan.person_name}</span>
      <span class="status-badge status-{l.loan.status}">
        {l.loan.status === "pendiente" ? "Pendiente" : "Pagado"}
      </span>
    </div>

    <div class="pending-amount">
      <span class="pending-value">{formatCOP(l.pending)}</span>
      <span class="pending-label">por cobrar</span>
    </div>

    <div class="progress-wrap">
      <div class="progress-bar">
        <div
          class="progress-fill"
          class:fill-done={l.loan.status === "pagado"}
          style="width: {loanPct(l)}%"
        ></div>
      </div>
      <span class="pct">{loanPct(l).toFixed(0)}%</span>
    </div>

    <div class="amounts">
      <span class="paid-label">Pagado</span>
      <span class="paid-value">{formatCOP(l.paid)}</span>
      <span class="sep">/</span>
      <span class="total-value">{formatCOP(l.loan.amount)}</span>
    </div>

    <div class="card-footer">
      <span class="loan-date">{l.loan.date}</span>
      <button
        class="btn-icon danger"
        onclick={(ev) => confirmDelete(l.loan.id, ev)}
        title="Eliminar"
      >✕</button>
    </div>
  </div>
{/snippet}

<!-- ── Modal: Crear ────────────────────────────────────────────────────────── -->
{#if createOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { createOpen = false; }}></div>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <ScrollArea class="modal-scroll" scrollbar="thin">
      <h2>Nuevo préstamo</h2>
      {#if cError}
        <div class="banner error small"><pre>{cError}</pre></div>
      {/if}
      <form onsubmit={handleCreate} class="modal-form">
        <div class="field">
          <label for="c-name">Nombre del deudor</label>
          <input id="c-name" type="text" bind:value={cName} placeholder="Ej: Juan, María…" maxlength="100" />
        </div>
        <div class="field">
          <label for="c-amount">Monto prestado</label>
          <input
            id="c-amount"
            type="text"
            inputmode="numeric"
            placeholder="0"
            value={cAmountRaw ? new Intl.NumberFormat("es-CO").format(cAmount) : ""}
            oninput={(e) => handleAmountInput(e, (v) => { cAmountRaw = v; })}
          />
        </div>
        <div class="field">
          <span class="field-label">Fecha del préstamo</span>
          <DatePicker bind:value={cDate} />
        </div>
        <div class="field">
          <label for="c-note">Nota <span class="optional">(opcional)</span></label>
          <input id="c-note" type="text" bind:value={cNote} placeholder="Para qué fue…" maxlength="200" />
        </div>
        <div class="modal-actions">
          <button type="button" class="btn-secondary" onclick={() => { createOpen = false; }}>Cancelar</button>
          <button type="submit" class="btn-primary" disabled={creating || cAmount <= 0 || !cName.trim() || !cDate}>
            {creating ? "Creando…" : "Crear"}
          </button>
        </div>
      </form>
    </ScrollArea>
  </div>
{/if}

<!-- ── Modal: Detalle ─────────────────────────────────────────────────────── -->
{#if detailLoading}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { detailLoading = false; }}></div>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <p class="muted">Cargando…</p>
  </div>
{:else if detail}
  <PaymentModal
    title={detail.loan.person_name}
    subtitle={detail.loan.status === "pendiente" ? "Pendiente" : "Pagado"}
    subtitleClass="status-{detail.loan.status}"
    note={detail.loan.note}
    stats={[
      { label: "Monto original", value: formatCOP(detail.loan.amount) },
      { label: "Pagado",         value: formatCOP(detail.paid),    colorClass: "success" },
      { label: "Pendiente",      value: formatCOP(detail.pending), colorClass: detail.loan.status === "pendiente" ? "accent" : undefined },
      { label: "Fecha",          value: detail.loan.date },
    ]}
    paid={detail.paid}
    total={detail.loan.amount}
    progressDone={detail.loan.status === "pagado"}
    items={detail.payments}
    itemsLabel="Abonos"
    canPay={detail.loan.status === "pendiente"}
    onAddPayment={handleAddPayment}
    onClose={() => { detail = null; }}
  />
{/if}

<!-- ── Confirm: Eliminar ───────────────────────────────────────────────────── -->
{#if deleteId !== null}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { deleteId = null; }}></div>
  <div class="modal modal-sm" role="dialog" aria-modal="true" tabindex="-1">
    <h2>¿Eliminar préstamo?</h2>
    <p class="muted">Se eliminarán también todos los abonos registrados.</p>
    <div class="modal-actions">
      <button class="btn-secondary" onclick={() => { deleteId = null; }}>Cancelar</button>
      <button class="btn-danger" onclick={handleDelete} disabled={deleting}>
        {deleting ? "Eliminando…" : "Eliminar"}
      </button>
    </div>
  </div>
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
  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); margin-bottom: 0.75rem; }

  /* ── Filtro ── */
  .filter-row { flex-shrink: 0; display: flex; gap: 0.4rem; }

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
  .filter-btn.active {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-elevated));
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  /* ── Secciones ── */
  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    padding: 0.5rem 0.25rem 0.25rem;
  }
  .section-label.secondary { color: var(--text-muted); margin-top: 0.75rem; }

  :global(.loans-scroll) { flex: 1; min-height: 0; }
  :global(.modal-scroll)  { flex: 1; min-height: 0; }

  /* ── Grid ── */
  .loan-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 1rem;
    align-content: start;
    padding: 0.25rem;
  }
  .loan-grid.dimmed { opacity: 0.55; }

  /* ── Card ── */
  .loan-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1rem;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    transition: border-color 0.15s;
    position: relative;
  }
  .loan-card:hover { border-color: var(--accent); }
  .loan-card.dimmed { cursor: default; }
  .loan-card.dimmed:hover { border-color: var(--border); }

  .card-top { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }

  .person-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  /* ── Pending amount ── */
  .pending-amount { display: flex; align-items: baseline; gap: 0.4rem; }
  .pending-value  { font-size: 1.1rem; font-weight: 700; color: var(--accent); font-variant-numeric: tabular-nums; }
  .pending-label  { font-size: 0.72rem; color: var(--text-muted); }

  /* ── Progress ── */
  .progress-wrap { display: flex; align-items: center; gap: 0.5rem; }
  .progress-bar  { flex: 1; height: 6px; background: var(--bg-elevated); border-radius: 999px; overflow: hidden; }
  .progress-fill { height: 100%; border-radius: 999px; background: var(--accent); transition: width 0.3s ease; }
  .progress-fill.fill-done { background: var(--success); }
  .pct { font-size: 0.72rem; color: var(--text-muted); min-width: 2.5rem; text-align: right; }

  /* ── Amounts row ── */
  .amounts { display: flex; align-items: baseline; gap: 0.3rem; font-size: 0.8rem; }
  .paid-label  { color: var(--text-muted); }
  .paid-value  { color: var(--text-secondary); font-weight: 500; }
  .sep         { color: var(--border); }
  .total-value { color: var(--text-muted); }

  /* ── Card footer ── */
  .card-footer { display: flex; align-items: center; justify-content: space-between; margin-top: 0.1rem; }
  .loan-date   { font-size: 0.72rem; color: var(--text-muted); }

  .btn-icon {
    width: 28px; height: 28px; border-radius: 6px; font-size: 0.8rem;
    background: var(--bg-elevated); color: var(--text-secondary);
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s, color 0.15s;
  }
  .btn-icon.danger:hover { background: color-mix(in srgb, var(--danger) 20%, var(--bg-elevated)); color: var(--danger); }

  /* ── Status badge ── */
  .status-badge {
    font-size: 0.65rem; font-weight: 600;
    padding: 0.15rem 0.5rem; border-radius: 999px; white-space: nowrap;
  }
  .status-pendiente { background: color-mix(in srgb, var(--accent) 20%, transparent);  color: var(--accent);  }
  .status-pagado    { background: color-mix(in srgb, var(--success) 20%, transparent); color: var(--success); }

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

  .btn-danger {
    padding: 0.45rem 1rem; background: color-mix(in srgb, var(--danger) 80%, transparent);
    color: #fff; font-size: 0.85rem; font-weight: 600;
    border-radius: var(--radius); transition: opacity 0.15s;
  }
  .btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Banner ── */
  .banner { border-radius: var(--radius); padding: 0.65rem 1rem; font-size: 0.85rem; }
  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }
  .banner.small { padding: 0.4rem 0.75rem; margin-bottom: 0.5rem; }
  .banner pre { font-size: 0.72rem; white-space: pre-wrap; word-break: break-all; }

  /* ── Overlay / Modal (para modales propios de esta página) ── */
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 20;
  }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 1.5rem; z-index: 21;
    width: min(440px, 92vw); max-height: 85vh; overflow: hidden;
    display: flex; flex-direction: column;
  }
  .modal-sm { width: min(340px, 92vw); }

  .modal-form { display: flex; flex-direction: column; gap: 0.9rem; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

  /* ── Campos ── */
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

  /* ── Misc ── */
  .muted { color: var(--text-muted); font-size: 0.85rem; }
  .empty {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 1rem; padding: 2rem;
    color: var(--text-muted); font-size: 0.9rem;
  }
</style>
