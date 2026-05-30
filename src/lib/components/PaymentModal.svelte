<script lang="ts">
  import DatePicker from "$lib/components/DatePicker.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";

  export interface PaymentItem {
    id: number;
    date: string;
    amount: number;
    note?: string | null;
    category?: string | null;
  }

  export interface StatEntry {
    label: string;
    value: string;
    colorClass?: string;
  }

  let {
    title,
    subtitle        = null,
    subtitleClass   = "",
    note            = null,
    stats,
    paid,
    total,
    progressDone    = false,
    items,
    itemsLabel      = "Abonos",
    showCategory    = false,
    showNote        = false,
    canPay          = true,
    onAddPayment,
    onClose,
    onEdit          = null,
    onDelete        = null,
  }: {
    title:          string;
    subtitle?:      string | null;
    subtitleClass?: string;
    note?:          string | null;
    stats:          StatEntry[];
    paid:           number;
    total:          number;
    progressDone?:  boolean;
    items:          PaymentItem[];
    itemsLabel?:    string;
    showCategory?:  boolean;
    showNote?:      boolean;
    canPay?:        boolean;
    onAddPayment:   (amount: number, date: string) => Promise<void>;
    onClose:        () => void;
    onEdit?:        (() => void) | null;
    onDelete?:      (() => void) | null;
  } = $props();

  let showPaymentForm = $state(false);
  let pAmountRaw      = $state("");
  let pDate           = $state("");
  let pError          = $state<string | null>(null);
  let paying          = $state(false);
  let pAmount = $derived(parseInt(pAmountRaw.replace(/\D/g, ""), 10) || 0);
  let pct     = $derived(total > 0 ? Math.min((paid / total) * 100, 100) : 0);

  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function handleAmountInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    pAmountRaw = digits;
    e.currentTarget.value = digits
      ? new Intl.NumberFormat("es-CO").format(parseInt(digits, 10))
      : "";
  }

  function extractMsg(e: unknown): string {
    if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
    return "Error desconocido. Intenta de nuevo.";
  }

  function resetForm() {
    showPaymentForm = false;
    pAmountRaw = ""; pDate = ""; pError = null;
  }

  async function handleSubmit(ev: Event) {
    ev.preventDefault();
    if (pAmount <= 0) { pError = "El monto debe ser mayor que 0."; return; }
    if (!pDate)       { pError = "La fecha es requerida."; return; }
    paying = true; pError = null;
    try {
      await onAddPayment(pAmount, pDate);
      resetForm();
    } catch (e) {
      pError = extractMsg(e);
    } finally {
      paying = false;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onClose}></div>
<div class="modal modal-wide" role="dialog" aria-modal="true" tabindex="-1">
  <ScrollArea class="modal-scroll" scrollbar="thin">
    <div class="detail-header">
      <h2>{title}</h2>
      {#if subtitle}
        <span class="status-badge {subtitleClass}">{subtitle}</span>
      {/if}
    </div>

    {#if note}
      <p class="detail-note">{note}</p>
    {/if}

    <div class="detail-stats">
      {#each stats as s}
        <div class="stat">
          <span class="stat-label">{s.label}</span>
          <span class="stat-value {s.colorClass ?? ''}">{s.value}</span>
        </div>
      {/each}
    </div>

    <div class="progress-wrap detail-progress">
      <div class="progress-bar">
        <div
          class="progress-fill"
          class:fill-done={progressDone}
          style="width: {pct}%"
        ></div>
      </div>
      <span class="pct">{pct.toFixed(0)}%</span>
    </div>

    <h3>{itemsLabel} ({items.length})</h3>

    {#if items.length === 0}
      <p class="muted">Sin {itemsLabel.toLowerCase()} registrados aún.</p>
    {:else}
      <ScrollArea orientation="horizontal" scrollbar="thin">
        <table class="items-table">
          <thead>
            <tr>
              <th>Fecha</th>
              {#if showCategory}<th>Categoría</th>{/if}
              <th class="right">Monto</th>
              {#if showNote}<th>Nota</th>{/if}
            </tr>
          </thead>
          <tbody>
            {#each items as item (item.id)}
              <tr>
                <td>{item.date}</td>
                {#if showCategory}<td>{item.category ?? "—"}</td>{/if}
                <td class="right amount-cell">{formatCOP(item.amount)}</td>
                {#if showNote}<td class="note-cell">{item.note ?? "—"}</td>{/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </ScrollArea>
    {/if}

    {#if showPaymentForm}
      <div class="payment-form-wrap">
        <h3>Registrar abono</h3>
        {#if pError}
          <div class="banner error small"><pre>{pError}</pre></div>
        {/if}
        <form onsubmit={handleSubmit} class="modal-form">
          <div class="field">
            <label for="pm-amount">Monto del abono</label>
            <input
              id="pm-amount"
              type="text"
              inputmode="numeric"
              placeholder="0"
              value={pAmountRaw ? new Intl.NumberFormat("es-CO").format(pAmount) : ""}
              oninput={handleAmountInput}
            />
          </div>
          <div class="field">
            <span class="field-label">Fecha del abono</span>
            <DatePicker bind:value={pDate} />
          </div>
          <div class="modal-actions">
            <button type="button" class="btn-secondary" onclick={resetForm}>Cancelar</button>
            <button type="submit" class="btn-primary" disabled={paying || pAmount <= 0 || !pDate}>
              {paying ? "Guardando…" : "Registrar"}
            </button>
          </div>
        </form>
      </div>
    {:else}
      <div class="modal-actions detail-actions">
        <button class="btn-secondary" onclick={onClose}>Cerrar</button>
        <div class="action-group">
          {#if canPay}
            <button class="btn-primary" onclick={() => { showPaymentForm = true; }}>+ Abonar</button>
          {/if}
          {#if onEdit}
            <button class="btn-secondary" onclick={onEdit}>Editar</button>
          {/if}
          {#if onDelete}
            <button class="btn-danger" onclick={onDelete}>Eliminar</button>
          {/if}
        </div>
      </div>
    {/if}
  </ScrollArea>
</div>

<style>
  :global(.modal-scroll) { flex: 1; min-height: 0; }

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
  .modal-wide { width: min(560px, 96vw); }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); margin-bottom: 0; }
  h3 { font-size: 0.85rem; font-weight: 600; color: var(--text-secondary); margin: 1rem 0 0.5rem; }

  .detail-header { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
  .detail-note   { font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0.5rem; }

  .detail-stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.75rem; margin-bottom: 0.5rem;
  }
  .stat { background: var(--bg-elevated); border-radius: var(--radius); padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.2rem; }
  .stat-label { font-size: 0.7rem; color: var(--text-muted); }
  .stat-value { font-size: 0.9rem; font-weight: 600; color: var(--text-primary); }
  .stat-value.success { color: var(--success); }
  .stat-value.accent  { color: var(--accent);  }

  .detail-progress { margin-bottom: 0.25rem; }
  .progress-wrap { display: flex; align-items: center; gap: 0.5rem; }
  .progress-bar  { flex: 1; height: 6px; background: var(--bg-elevated); border-radius: 999px; overflow: hidden; }
  .progress-fill { height: 100%; border-radius: 999px; background: var(--accent); transition: width 0.3s ease; }
  .progress-fill.fill-done { background: var(--success); }
  .pct { font-size: 0.72rem; color: var(--text-muted); min-width: 2.5rem; text-align: right; }

  .status-badge {
    font-size: 0.65rem; font-weight: 600;
    padding: 0.15rem 0.5rem; border-radius: 999px; white-space: nowrap;
  }

  .items-table { width: 100%; font-size: 0.8rem; border-collapse: collapse; }
  .items-table th,
  .items-table td { padding: 0.4rem 0.5rem; text-align: left; border-bottom: 1px solid var(--border); }
  .items-table th { color: var(--text-muted); font-weight: 500; font-size: 0.72rem; }
  .items-table td { color: var(--text-secondary); }
  .right       { text-align: right; }
  .amount-cell { color: var(--text-primary); font-weight: 500; }
  .note-cell   { color: var(--text-muted); max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .payment-form-wrap { border-top: 1px solid var(--border); padding-top: 0.75rem; margin-top: 0.5rem; }
  .payment-form-wrap h3 { margin-top: 0; }

  .modal-form    { display: flex; flex-direction: column; gap: 0.9rem; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .detail-actions { justify-content: space-between; align-items: center; }
  .action-group   { display: flex; gap: 0.5rem; }
  .btn-danger {
    padding: 0.45rem 1rem; background: transparent; color: var(--danger);
    font-size: 0.85rem; font-weight: 600; border-radius: var(--radius);
    border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
    transition: all 0.15s;
  }
  .btn-danger:hover { background: color-mix(in srgb, var(--danger) 10%, transparent); }

  .field { display: flex; flex-direction: column; gap: 0.3rem; }
  label, .field-label { font-size: 0.78rem; font-weight: 500; color: var(--text-secondary); }

  input[type="text"] {
    -webkit-appearance: none; appearance: none;
    background-color: #14141f; border: 1px solid #2a2a40;
    border-radius: var(--radius); color: #e8e8f0; font: inherit;
    font-size: 0.9rem; padding: 0.5rem 0.75rem; outline: none;
    transition: border-color 0.15s; width: 100%;
  }
  input:focus { border-color: var(--accent); }

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

  .banner { border-radius: var(--radius); padding: 0.65rem 1rem; font-size: 0.85rem; }
  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }
  .banner.small { padding: 0.4rem 0.75rem; margin-bottom: 0.5rem; }
  .banner pre { font-size: 0.72rem; white-space: pre-wrap; word-break: break-all; }

  .muted { color: var(--text-muted); font-size: 0.85rem; }
</style>
