<script lang="ts">
  let { value = $bindable("") }: { value: string } = $props();

  const MESES = [
    "enero","febrero","marzo","abril","mayo","junio",
    "julio","agosto","septiembre","octubre","noviembre","diciembre",
  ];
  const MESES_CORTO = ["ene","feb","mar","abr","may","jun","jul","ago","sep","oct","nov","dic"];

  let open    = $state(false);
  let tempDay   = $state(1);
  let tempMonth = $state(1);
  let tempYear  = $state(2026);

  function parseISO(iso: string) {
    const p = iso.split("-");
    return { y: parseInt(p[0], 10), m: parseInt(p[1], 10), d: parseInt(p[2], 10) };
  }

  function pad(n: number): string { return String(n).padStart(2, "0"); }

  function formatDisplay(iso: string): string {
    try {
      const { y, m, d } = parseISO(iso);
      return `${d} ${MESES_CORTO[m - 1]} ${y}`;
    } catch { return iso; }
  }

  function daysIn(year: number, month: number): number {
    return new Date(year, month, 0).getDate();
  }

  function openPicker() {
    if (value) {
      const { y, m, d } = parseISO(value);
      tempYear = y; tempMonth = m; tempDay = d;
    } else {
      const now = new Date();
      tempYear = now.getFullYear();
      tempMonth = now.getMonth() + 1;
      tempDay = now.getDate();
    }
    open = true;
  }

  function apply() {
    const d = Math.min(tempDay, daysIn(tempYear, tempMonth));
    value = `${tempYear}-${pad(tempMonth)}-${pad(d)}`;
    open = false;
  }

  function cancel() { open = false; }

  const NOW_YEAR = new Date().getFullYear();
  const YEARS = Array.from({ length: 16 }, (_, i) => 2020 + i);

  let days = $derived(Array.from({ length: daysIn(tempYear, tempMonth) }, (_, i) => i + 1));
</script>

{#if open}
  <div class="dp-row">
    <select class="dp-sel dp-day" bind:value={tempDay}>
      {#each days as d}
        <option value={d}>{d}</option>
      {/each}
    </select>
    <select class="dp-sel dp-month" bind:value={tempMonth}>
      {#each MESES as mes, i}
        <option value={i + 1}>{mes}</option>
      {/each}
    </select>
    <select class="dp-sel dp-year" bind:value={tempYear}>
      {#each YEARS as y}
        <option value={y}>{y}</option>
      {/each}
    </select>
    <button type="button" class="dp-btn dp-ok" onclick={apply} title="Aplicar">✓</button>
    <button type="button" class="dp-btn dp-x"  onclick={cancel} title="Cancelar">✕</button>
  </div>
{:else}
  <button type="button" class="dp-closed" onclick={openPicker}>
    <svg class="dp-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
      <rect x="1" y="3" width="14" height="12" rx="2"/>
      <path d="M1 7h14M5 1v4M11 1v4"/>
    </svg>
    {value ? formatDisplay(value) : "Seleccionar fecha…"}
  </button>
{/if}

<style>
  /* ── Closed ── */
  .dp-closed {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    text-align: left;
    padding: 0.5rem 0.75rem;
    background: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.9rem;
    width: 100%;
    transition: border-color 0.15s, color 0.15s;
  }
  .dp-closed:hover { border-color: var(--accent); color: var(--accent); }

  .dp-icon {
    width: 14px;
    height: 14px;
    opacity: 0.55;
    flex-shrink: 0;
  }
  .dp-closed:hover .dp-icon { opacity: 1; }

  /* ── Open ── */
  .dp-row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .dp-sel {
    -webkit-appearance: none;
    appearance: none;
    background-color: #14141f;
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.82rem;
    padding: 0.4rem 0.35rem;
    outline: none;
    cursor: pointer;
  }
  .dp-sel option { background-color: #14141f; }

  .dp-day   { width: 50px; text-align: center; }
  .dp-month { flex: 1; min-width: 0; }
  .dp-year  { width: 68px; text-align: center; }

  /* ── Botones ── */
  .dp-btn {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
    font-weight: 600;
  }

  .dp-ok {
    background: color-mix(in srgb, var(--success) 18%, var(--bg-elevated));
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
  }
  .dp-ok:hover { background: color-mix(in srgb, var(--success) 30%, var(--bg-elevated)); }

  .dp-x {
    background: var(--bg-elevated);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .dp-x:hover { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 35%, transparent); }
</style>
