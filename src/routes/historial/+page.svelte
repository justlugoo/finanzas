<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Transaction, TransactionInput, CsvExport, ImportResult } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import { txState } from "$lib/txState.svelte";

  type PeriodKey = "Daily" | "Weekly" | "Monthly" | "Yearly";

  const PERIOD_LABELS: Record<PeriodKey, string> = {
    Daily: "Diario", Weekly: "Semanal", Monthly: "Mensual", Yearly: "Anual",
  };

  // ── Filtros ────────────────────────────────────────────────────────────────
  let activePeriod = $state<PeriodKey>("Monthly");
  let filterKind   = $state<"" | "ingreso" | "gasto">("");
  let filterCat    = $state("");

  // ── Datos ─────────────────────────────────────────────────────────────────
  let txs          = $state<Transaction[]>([]);
  let categories   = $state<string[]>([]);
  let loading      = $state(true);
  let error        = $state<string | null>(null);

  // ── Edición ───────────────────────────────────────────────────────────────
  let editingTx          = $state<Transaction | null>(null);
  let editAmount         = $state("");
  let editCategory       = $state("");
  let editCarreraPersona = $state<"mama" | "cunada" | null>(null);
  let editDate           = $state("");
  let editNote           = $state("");
  let editKind           = $state<"ingreso" | "gasto">("gasto");
  let editExtraord       = $state(false);
  let editSaving         = $state(false);
  let editError          = $state<string | null>(null);

  // ── Filtro deudas ─────────────────────────────────────────────────────────
  let filterDebt = $state(false);

  // ── Confirmación eliminar ─────────────────────────────────────────────────
  let deletingId         = $state<number | null>(null);
  let deletingInProgress = $state<number | null>(null);

  // ── Selección múltiple ────────────────────────────────────────────────────
  let selectMode        = $state(false);
  let selectedIds       = $state<Set<number>>(new Set());
  let bulkConfirming    = $state(false);
  let bulkDeleting      = $state(false);
  let bulkSuccessMsg    = $state<string | null>(null);

  let allSelected = $derived(txs.length > 0 && selectedIds.size === txs.length);

  function enterSelectMode()  { selectMode = true; selectedIds = new Set(); bulkConfirming = false; }
  function exitSelectMode()   { selectMode = false; selectedIds = new Set(); bulkConfirming = false; }

  function toggleSelect(id: number) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAll() {
    selectedIds = allSelected ? new Set() : new Set(txs.map(t => t.id));
  }

  async function bulkDelete() {
    if (selectedIds.size === 0 || bulkDeleting) return;
    bulkDeleting = true;
    bulkConfirming = false;
    try {
      const ids = [...selectedIds];
      const deleted = await invoke<number>("delete_transactions_bulk", { ids });
      txs = txs.filter(t => !selectedIds.has(t.id));
      bulkSuccessMsg = `${deleted} transacción${deleted !== 1 ? "es" : ""} eliminada${deleted !== 1 ? "s" : ""}.`;
      exitSelectMode();
      setTimeout(() => { bulkSuccessMsg = null; }, 3000);
    } catch (e) {
      console.error("[historial] bulk delete error:", e);
      error = "No se pudieron eliminar las transacciones. Intenta de nuevo.";
      exitSelectMode();
    } finally {
      bulkDeleting = false;
    }
  }

  // ── Exportar / Importar ───────────────────────────────────────────────────
  let exporting      = $state(false);
  let importing      = $state(false);
  let importResult   = $state<ImportResult | null>(null);
  let reloadKey      = $state(0);
  let fileInputEl: HTMLInputElement | undefined = $state();

  let total = $derived(txs.reduce((sum, tx) => {
    return sum + (tx.type === "ingreso" ? tx.amount : -tx.amount);
  }, 0));

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

  function formatCategory(cat: string): string {
    if (cat === "Carrera mamá")   return "Carrera · mamá";
    if (cat === "Carrera cuñada") return "Carrera · cuñada";
    return cat;
  }

  function transformCategories(raw: string[]): string[] {
    const hasCarrera = raw.some(c => c === "Carrera mamá" || c === "Carrera cuñada");
    const filtered = raw.filter(c => c !== "Carrera mamá" && c !== "Carrera cuñada");
    return hasCarrera ? [...filtered, "Carrera"].sort() : filtered;
  }

  function buildFilter() {
    return {
      period:     { type: activePeriod },
      kind:       filterKind  || null,
      category:   filterCat   || null,
      only_debt:  filterDebt  || null,
    };
  }

  $effect(() => {
    const _period = activePeriod;
    const _kind   = filterKind;
    const _cat    = filterCat;
    const _debt   = filterDebt;
    const _reload = reloadKey;
    const _v      = txState.version;
    let cancelled = false;

    async function load() {
      loading = true;
      error   = null;
      try {
        const [data, cats] = await Promise.all([
          invoke<Transaction[]>("list_transactions", { filter: buildFilter() }),
          invoke<string[]>("list_categories"),
        ]);
        if (!cancelled) {
          txs        = data;
          categories = transformCategories(cats);
          loading    = false;
        }
      } catch (e) {
        if (!cancelled) { console.error("[historial] load error:", e); error = "No se pudieron cargar las transacciones."; loading = false; }
      }
    }

    load();
    return () => { cancelled = true; };
  });

  function startEdit(tx: Transaction) {
    editingTx    = tx;
    editKind     = tx.type as "ingreso" | "gasto";
    editDate     = tx.date;
    editNote     = tx.note ?? "";
    editExtraord = tx.is_extraordinary;
    editAmount   = tx.amount.toString();
    editError    = null;

    if (tx.category === "Carrera mamá") {
      editCategory = "Carrera";
      editCarreraPersona = "mama";
    } else if (tx.category === "Carrera cuñada") {
      editCategory = "Carrera";
      editCarreraPersona = "cunada";
    } else {
      editCategory = tx.category;
      editCarreraPersona = null;
    }
  }

  function cancelEdit() { editingTx = null; }

  async function saveEdit() {
    if (!editingTx) return;
    const amt = parseInt(editAmount, 10);
    if (!amt || amt <= 0) { editError = "Monto inválido."; return; }
    editSaving = true;
    editError  = null;

    const effectiveCat =
      editCategory === "Carrera" && editCarreraPersona === "mama"   ? "Carrera mamá"  :
      editCategory === "Carrera" && editCarreraPersona === "cunada" ? "Carrera cuñada" :
      editCategory;

    const input: TransactionInput = {
      date: editDate,
      type: editKind,
      category: effectiveCat,
      amount: amt,
      note: editNote.trim() || null,
      is_extraordinary: editExtraord,
      goal_id: editingTx.goal_id,
      gas_km: null,
      is_debt: editingTx.is_debt,
    };

    const prevTxs     = txs;
    const prevEditing = editingTx;
    const optimistic: Transaction = {
      ...editingTx!,
      date: editDate,
      type: editKind,
      category: effectiveCat,
      amount: amt,
      note: editNote.trim() || null,
      is_extraordinary: editExtraord,
    };
    txs       = txs.map((t) => t.id === editingTx!.id ? optimistic : t);
    editingTx = null;

    try {
      const updated = await invoke<Transaction>("update_transaction", {
        id: optimistic.id,
        input,
      });
      txs = txs.map((t) => t.id === updated.id ? updated : t);
    } catch (e) {
      txs       = prevTxs;
      editingTx = prevEditing;
      console.error("[historial] save edit error:", e);
      editError = "No se pudo guardar el cambio. Intenta de nuevo.";
    } finally {
      editSaving = false;
    }
  }

  async function confirmDelete(id: number) {
    deletingInProgress = id;
    deletingId = null;
    const prev = txs;
    txs = txs.filter((t) => t.id !== id);
    try {
      await invoke("delete_transaction", { id });
    } catch (e) {
      txs = prev;
      error = "No se pudo eliminar la transacción. Intenta de nuevo.";
    } finally {
      deletingInProgress = null;
    }
  }

  function triggerImport() {
    importResult = null;
    fileInputEl?.click();
  }

  function handleImportFile(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    importing = true;
    importResult = null;
    const reader = new FileReader();
    reader.onload = async (ev) => {
      const content = ev.target?.result as string;
      try {
        const result = await invoke<ImportResult>("import_transactions_csv", { csvContent: content });
        importResult = result;
        if (result.imported > 0) reloadKey += 1;
      } catch (err) {
        error = typeof err === "string" ? err : "Error al importar el archivo.";
      } finally {
        importing = false;
        if (fileInputEl) fileInputEl.value = "";
      }
    };
    reader.readAsText(file);
  }

  async function exportCSV() {
    exporting = true;
    try {
      const result = await invoke<CsvExport>("export_transactions_csv", {
        filter: buildFilter(),
      });
      const blob = new Blob([result.content], { type: "text/csv;charset=utf-8;" });
      const url  = URL.createObjectURL(blob);
      const a    = document.createElement("a");
      a.href     = url;
      a.download = result.suggested_filename;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("[historial] export error:", e);
      error = "Error al exportar. Intenta de nuevo.";
    } finally {
      exporting = false;
    }
  }
</script>

<main>
  <div class="toolbar">
    <h1>Historial</h1>
    <div class="toolbar-actions">
      {#if !selectMode}
        <button class="action-btn" onclick={triggerImport} disabled={importing}>
          {importing ? "Importando…" : "Importar CSV"}
        </button>
        <button class="action-btn" onclick={exportCSV} disabled={exporting || txs.length === 0}>
          {exporting ? "Exportando…" : "Exportar CSV"}
        </button>
        <button class="action-btn" onclick={enterSelectMode} disabled={txs.length === 0}>
          Seleccionar
        </button>
      {:else}
        <button class="action-btn" onclick={exitSelectMode} disabled={bulkDeleting}>Cancelar</button>
      {/if}
    </div>
  </div>

  <input
    type="file"
    accept=".csv,text/csv"
    bind:this={fileInputEl}
    onchange={handleImportFile}
    style="display:none"
  />

  {#if error}
    <div class="banner error"><strong>Error</strong><pre>{error}</pre></div>
  {/if}

  {#if bulkSuccessMsg}
    <div class="banner success">{bulkSuccessMsg}</div>
  {/if}

  {#if selectMode}
    <div class="bulk-bar">
      <span class="bulk-count">{selectedIds.size} seleccionada{selectedIds.size !== 1 ? "s" : ""}</span>
      <div class="bulk-actions">
        {#if bulkConfirming}
          <span class="bulk-confirm-text">¿Eliminar {selectedIds.size} transacción{selectedIds.size !== 1 ? "es" : ""}? No se puede deshacer.</span>
          <button class="action-btn danger" onclick={bulkDelete} disabled={bulkDeleting}>
            {bulkDeleting ? "Eliminando…" : "Confirmar"}
          </button>
          <button class="action-btn" onclick={() => { bulkConfirming = false; }} disabled={bulkDeleting}>Cancelar</button>
        {:else}
          <button
            class="action-btn danger"
            onclick={() => { bulkConfirming = true; }}
            disabled={selectedIds.size === 0}
          >
            Eliminar seleccionadas
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if importResult}
    <div class="banner" class:success={importResult.skipped === 0} class:warning={importResult.skipped > 0}>
      <strong>Importación completada</strong>
      <span>{importResult.imported} importadas, {importResult.skipped} omitidas</span>
      {#if importResult.errors.length > 0}
        <ul class="import-errors">
          {#each importResult.errors.slice(0, 5) as err}
            <li>{err}</li>
          {/each}
          {#if importResult.errors.length > 5}
            <li>… y {importResult.errors.length - 5} más</li>
          {/if}
        </ul>
      {/if}
    </div>
  {/if}

  <!-- Filtros -->
  <div class="filters">
    <nav class="period-selector">
      {#each (Object.keys(PERIOD_LABELS) as PeriodKey[]) as key}
        <button class:active={activePeriod === key} onclick={() => { activePeriod = key; }}>
          {PERIOD_LABELS[key]}
        </button>
      {/each}
    </nav>

    <select bind:value={filterKind} class="filter-select">
      <option value="">Todos los tipos</option>
      <option value="ingreso">Ingresos</option>
      <option value="gasto">Gastos</option>
    </select>

    <select bind:value={filterCat} class="filter-select">
      <option value="">Todas las categorías</option>
      {#each categories as cat}
        <option value={cat}>{cat}</option>
      {/each}
    </select>

    <button
      type="button"
      class="filter-debt-btn"
      class:active={filterDebt}
      onclick={() => { filterDebt = !filterDebt; }}
    >
      Solo deudas
    </button>
  </div>

  <!-- Tabla -->
  {#if loading}
    <div class="placeholder-list">
      {#each [1,2,3,4,5] as _}<div class="placeholder-row"></div>{/each}
    </div>
  {:else if txs.length === 0}
    <p class="empty">Sin transacciones en este período.</p>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            {#if selectMode}
              <th class="check-col">
                <input type="checkbox" checked={allSelected} onchange={toggleSelectAll} />
              </th>
            {/if}
            <th>Fecha</th>
            <th>Tipo</th>
            <th>Categoría</th>
            <th class="right">Monto</th>
            <th>Nota</th>
            {#if !selectMode}<th></th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each txs as tx (tx.id)}
            <tr class:row-selected={selectedIds.has(tx.id)}>
              {#if selectMode}
                <td class="check-col">
                  <input type="checkbox" checked={selectedIds.has(tx.id)} onchange={() => toggleSelect(tx.id)} />
                </td>
              {/if}
              <td class="date-cell">{formatDate(tx.date)}</td>
              <td class="type-cell">
                <span class="badge" class:badge-income={tx.type === "ingreso"} class:badge-expense={tx.type === "gasto"}>
                  {tx.type === "ingreso" ? "Ingreso" : "Gasto"}
                </span>
                {#if tx.is_debt}
                  <span class="badge badge-debt">deuda</span>
                {/if}
              </td>
              <td>{formatCategory(tx.category)}</td>
              <td class="right amount-cell" class:income={tx.type === "ingreso"} class:expense={tx.type === "gasto"}>
                {tx.type === "ingreso" ? "+" : "−"}{formatCOP(tx.amount)}
              </td>
              <td class="note-cell">{tx.note ?? ""}</td>
              {#if !selectMode}
                <td class="actions-cell">
                  {#if deletingId === tx.id}
                    <span class="confirm-del">
                      ¿Eliminar?
                      <button
                        class="action-link danger"
                        onclick={() => confirmDelete(tx.id)}
                        disabled={deletingInProgress === tx.id}
                      >{deletingInProgress === tx.id ? "…" : "Sí"}</button>
                      <button class="action-link" onclick={() => { deletingId = null; }} disabled={deletingInProgress === tx.id}>No</button>
                    </span>
                  {:else}
                    <button class="action-link" onclick={() => startEdit(tx)}>Editar</button>
                    <button class="action-link danger" onclick={() => { deletingId = tx.id; }}>Eliminar</button>
                  {/if}
                </td>
              {/if}
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <td colspan={selectMode ? 4 : 3} class="total-label">Total período</td>
            <td class="right total-value" class:income={total >= 0} class:expense={total < 0}>
              {total >= 0 ? "+" : "−"}{formatCOP(Math.abs(total))}
            </td>
            <td colspan={selectMode ? 1 : 2}></td>
          </tr>
        </tfoot>
      </table>
    </div>
  {/if}
</main>

<!-- Modal edición -->
{#if editingTx}
  <div
    class="modal-overlay"
    role="button"
    tabindex="-1"
    onclick={cancelEdit}
    onkeydown={(e) => { if (e.key === "Escape") cancelEdit(); }}
  >
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <h2>Editar transacción #{editingTx.id}</h2>

      {#if editError}
        <div class="banner error"><pre>{editError}</pre></div>
      {/if}

      <div class="modal-form">
        <div class="type-toggle">
          <button type="button" class="toggle-btn income" class:active={editKind === "ingreso"} onclick={() => { editKind = "ingreso"; }}>Ingreso</button>
          <button type="button" class="toggle-btn expense" class:active={editKind === "gasto"} onclick={() => { editKind = "gasto"; }}>Gasto</button>
        </div>

        <div class="field">
          <label for="edit-cat">Categoría</label>
          <select id="edit-cat" bind:value={editCategory} onchange={() => { editCarreraPersona = null; }}>
            {#each categories as cat}
              <option value={cat}>{cat}</option>
            {/each}
          </select>
        </div>

        {#if editCategory === "Carrera" && editKind === "ingreso"}
          <div class="field">
            <span class="field-label">Persona</span>
            <div class="carrera-toggle">
              <button type="button" class:active={editCarreraPersona === "mama"}   onclick={() => { editCarreraPersona = "mama"; }}>Mamá</button>
              <button type="button" class:active={editCarreraPersona === "cunada"} onclick={() => { editCarreraPersona = "cunada"; }}>Cuñada</button>
            </div>
          </div>
        {/if}

        <div class="field">
          <label for="edit-amount">Monto</label>
          <input id="edit-amount" type="number" min="1" bind:value={editAmount} />
        </div>

        <div class="field">
          <label>Fecha</label>
          <DatePicker bind:value={editDate} />
        </div>

        <div class="field">
          <label for="edit-note">Nota</label>
          <input id="edit-note" type="text" bind:value={editNote} />
        </div>

        <label class="checkbox-row">
          <input type="checkbox" bind:checked={editExtraord} />
          <span>Extraordinario</span>
        </label>
      </div>

      <div class="modal-actions">
        <button class="btn-cancel" onclick={cancelEdit}>Cancelar</button>
        <button
          class="btn-save"
          onclick={saveEdit}
          disabled={editSaving || (editCategory === "Carrera" && editKind === "ingreso" && !editCarreraPersona)}
        >
          {editSaving ? "Guardando…" : "Guardar"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    width: 100%;
    box-sizing: border-box;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  h1 { font-size: 1.25rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.02em; }

  .toolbar-actions { display: flex; gap: 0.4rem; }

  .action-btn {
    padding: 0.4rem 0.9rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: color 0.15s, background 0.15s;
  }

  .action-btn:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-surface); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

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
  }
  .banner.warning {
    background: color-mix(in srgb, var(--warning) 12%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--warning) 40%, transparent);
    color: var(--warning);
  }
  .banner pre { font-size: 0.72rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }
  .import-errors { font-size: 0.75rem; opacity: 0.85; margin-top: 0.25rem; padding-left: 1.1rem; display: flex; flex-direction: column; gap: 0.1rem; }

  /* ── Filtros ── */
  .filters {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .period-selector {
    display: flex;
    gap: 4px;
    background: var(--bg-elevated);
    padding: 4px;
    border-radius: 8px;
  }

  .period-selector button {
    padding: 0.3rem 0.65rem;
    border-radius: 5px;
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .period-selector button:hover { color: var(--text-primary); background: var(--bg-surface); }
  .period-selector button.active { background: var(--accent); color: #fff; }

  .filter-select {
    -webkit-appearance: none;
    appearance: none;
    background-color: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.35rem 1.8rem 0.35rem 0.65rem;
    outline: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%238888aa' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.45rem center;
    background-size: 0.875rem;
  }

  .filter-select:focus { border-color: var(--accent); }
  .filter-select option { background-color: #14141f; color: #e8e8f0; }

  .filter-debt-btn {
    padding: 0.35rem 0.75rem;
    border-radius: var(--radius);
    font-size: 0.8rem;
    font-weight: 500;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: all 0.15s;
    white-space: nowrap;
  }
  .filter-debt-btn:hover { color: var(--text-primary); }
  .filter-debt-btn.active {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-elevated));
    border-color: color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }

  /* ── Tabla ── */
  .table-wrap {
    overflow-x: auto;
    border-radius: var(--radius);
    border: 1px solid var(--border);
  }

  table { width: 100%; border-collapse: collapse; }

  th {
    text-align: left;
    padding: 0.6rem 0.875rem;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  td {
    padding: 0.55rem 0.875rem;
    font-size: 0.85rem;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  tr:last-child td { border-bottom: none; }
  tr:hover td { background: var(--bg-elevated); }

  .right { text-align: right; }

  .date-cell { color: var(--text-muted); font-variant-numeric: tabular-nums; white-space: nowrap; }

  .amount-cell { font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .amount-cell.income  { color: var(--success); }
  .amount-cell.expense { color: var(--danger); }

  .note-cell { color: var(--text-secondary); font-size: 0.8rem; max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* ── Badge tipo ── */
  .badge {
    display: inline-block;
    padding: 0.18rem 0.5rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .badge-income  { background: color-mix(in srgb, var(--success) 20%, var(--bg-elevated)); color: var(--success); }
  .badge-expense { background: color-mix(in srgb, var(--danger)  20%, var(--bg-elevated)); color: var(--danger); }
  .badge-debt    { background: color-mix(in srgb, var(--danger)  12%, transparent); color: var(--danger); border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); text-transform: uppercase; letter-spacing: 0.04em; font-size: 0.65rem; }

  .type-cell { display: flex; align-items: center; gap: 0.3rem; flex-wrap: wrap; }

  /* ── Acciones ── */
  .actions-cell { white-space: nowrap; }

  .action-link {
    font-size: 0.78rem;
    color: var(--text-muted);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    transition: color 0.15s;
  }

  .action-link:hover { color: var(--text-primary); }
  .action-link.danger:hover { color: var(--danger); }

  .confirm-del { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.78rem; color: var(--text-secondary); }

  /* ── Totales ── */
  tfoot td {
    border-top: 1px solid var(--border);
    border-bottom: none;
    background: var(--bg-elevated);
    padding: 0.6rem 0.875rem;
  }

  .total-label { font-size: 0.78rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); }

  .total-value { font-size: 0.95rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  .total-value.income  { color: var(--success); }
  .total-value.expense { color: var(--danger); }

  /* ── Placeholders ── */
  .placeholder-list { display: flex; flex-direction: column; gap: 0.5rem; }
  .placeholder-row { height: 42px; border-radius: var(--radius); background: var(--bg-surface); animation: shimmer 1.4s ease-in-out infinite; }
  @keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.7; } }

  .empty { color: var(--text-muted); font-size: 0.85rem; padding: 0.5rem 0; }

  /* ── Modal ── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) + 4px);
    padding: 1.5rem;
    width: 100%;
    max-width: 400px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .modal h2 { font-size: 1rem; font-weight: 600; color: var(--text-primary); }

  .modal-form { display: flex; flex-direction: column; gap: 0.75rem; }

  .field { display: flex; flex-direction: column; gap: 0.3rem; }

  label, .field-label { font-size: 0.8rem; font-weight: 500; color: var(--text-secondary); }

  input[type="text"],
  input[type="number"],
  input[type="date"],
  select {
    -webkit-appearance: none;
    appearance: none;
    background-color: #1c1c2e;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.875rem;
    padding: 0.5rem 2rem 0.5rem 0.65rem;
    outline: none;
    width: 100%;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%238888aa' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.5rem center;
    background-size: 0.875rem;
  }

  input[type="text"],
  input[type="number"],
  input[type="date"] {
    background-image: none;
    padding-right: 0.65rem;
  }

  select option { background-color: #1c1c2e; color: #e8e8f0; }

  input:focus, select:focus { border-color: var(--accent); }

  .checkbox-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.875rem; color: var(--text-secondary); cursor: pointer; }
  .checkbox-row input { accent-color: var(--accent); }

  .type-toggle { display: grid; grid-template-columns: 1fr 1fr; background: var(--bg-elevated); border-radius: var(--radius); padding: 3px; gap: 3px; }

  .toggle-btn { padding: 0.45rem; border-radius: 5px; font-size: 0.85rem; font-weight: 600; color: var(--text-secondary); transition: background 0.15s, color 0.15s; }
  .toggle-btn.income.active  { background: color-mix(in srgb, var(--success) 20%, var(--bg-surface)); color: var(--success); }
  .toggle-btn.expense.active { background: color-mix(in srgb, var(--danger)  20%, var(--bg-surface)); color: var(--danger); }

  /* Carrera sub-selector en modal */
  .carrera-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 3px;
    gap: 3px;
  }

  .carrera-toggle button {
    padding: 0.4rem;
    border-radius: 5px;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .carrera-toggle button.active {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-surface));
    color: var(--accent);
  }

  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; padding-top: 0.25rem; }

  .btn-cancel {
    padding: 0.5rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .btn-cancel:hover { color: var(--text-primary); }

  .btn-save {
    padding: 0.5rem 1rem;
    background: var(--accent);
    border-radius: var(--radius);
    font-size: 0.85rem;
    font-weight: 600;
    color: #fff;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn-save:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-save:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Selección múltiple ── */
  .bulk-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.6rem 0.875rem;
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .bulk-count { color: var(--text-primary); font-weight: 600; }
  .bulk-actions { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .bulk-confirm-text { font-size: 0.82rem; color: var(--text-secondary); }
  .action-btn.danger {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 40%, transparent);
  }
  .action-btn.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, var(--bg-elevated));
    color: var(--danger);
  }
  .check-col { width: 36px; text-align: center; padding: 0 0.25rem; }
  .check-col input[type="checkbox"] {
    -webkit-appearance: none;
    appearance: none;
    width: 15px;
    height: 15px;
    border: 1.5px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elevated);
    cursor: pointer;
    position: relative;
    vertical-align: middle;
    flex-shrink: 0;
    transition: border-color 0.15s, background 0.15s;
  }
  .check-col input[type="checkbox"]:hover {
    border-color: var(--accent);
  }
  .check-col input[type="checkbox"]:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .check-col input[type="checkbox"]:checked::after {
    content: "";
    position: absolute;
    left: 4px;
    top: 1px;
    width: 4px;
    height: 8px;
    border: 2px solid #fff;
    border-top: none;
    border-left: none;
    transform: rotate(45deg);
  }
  .row-selected td { background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .banner.success {
    background: color-mix(in srgb, var(--success) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--success) 40%, transparent);
    color: var(--success);
    font-weight: 500;
  }
</style>
