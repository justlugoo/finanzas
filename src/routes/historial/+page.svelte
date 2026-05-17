<script lang="ts">
  import { transactionApi } from "$lib/api";
  import { HISTORY_PAGE_SIZE } from "$lib/constants";
  import type { Transaction, TransactionInput, TransactionPage, CsvExport, ImportResult, PeriodSummary } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import { txState, bumpTxVersion } from "$lib/txState.svelte";
  import { MESES_CORTO, DIAS_SEMANA } from "$lib/constants";

  type PeriodKey = "Daily" | "Weekly" | "Monthly" | "Yearly";

  const PERIOD_LABELS: Record<PeriodKey, string> = {
    Daily: "Diario", Weekly: "Semanal", Monthly: "Mensual", Yearly: "Anual",
  };

  const PAGE_SIZE = HISTORY_PAGE_SIZE;

  // ── Filtros ────────────────────────────────────────────────────────────────
  let activePeriod  = $state<PeriodKey>("Monthly");
  let filterKind    = $state<"" | "ingreso" | "gasto">("");
  let filterCat     = $state("");
  let filterDebt    = $state(false);
  let filterSearch  = $state("");

  // ── Paginación ────────────────────────────────────────────────────────────
  let currentPage = $state(1);
  let totalCount  = $state(0);
  let totalPages  = $derived(Math.max(1, Math.ceil(totalCount / PAGE_SIZE)));

  let pageNumbers = $derived.by(() => {
    if (totalPages <= 7) return Array.from({ length: totalPages }, (_, i) => i + 1);
    const start = Math.max(1, Math.min(currentPage - 2, totalPages - 4));
    const end   = Math.min(totalPages, start + 4);
    return Array.from({ length: end - start + 1 }, (_, i) => start + i);
  });

  // ── Datos ─────────────────────────────────────────────────────────────────
  let txs           = $state<Transaction[]>([]);
  let categories    = $state<string[]>([]);
  let periodSummary = $state<PeriodSummary | null>(null);
  let loading       = $state(true);
  let error         = $state<string | null>(null);

  // ── Agrupamiento por fecha ─────────────────────────────────────────────────
  let grouped = $derived.by(() => {
    const map = new Map<string, Transaction[]>();
    for (const tx of txs) {
      if (!map.has(tx.date)) map.set(tx.date, []);
      map.get(tx.date)!.push(tx);
    }
    return [...map.entries()].map(([date, items]) => ({ date, items }));
  });

  let collapsedGroups = $state<Set<string>>(new Set());

  function toggleGroup(date: string) {
    const next = new Set(collapsedGroups);
    if (next.has(date)) next.delete(date); else next.add(date);
    collapsedGroups = next;
  }

  // ── Menú ⋯ ────────────────────────────────────────────────────────────────
  let menuOpen = $state(false);

  // ── Edición ───────────────────────────────────────────────────────────────
  let editingTx          = $state<Transaction | null>(null);
  let editAmount         = $state("");
  let editCategory       = $state("");
  let editDate           = $state("");
  let editNote           = $state("");
  let editKind           = $state<"ingreso" | "gasto">("gasto");
  let editExtraord       = $state(false);
  let editSaving         = $state(false);
  let editError          = $state<string | null>(null);

  // ── Confirmación eliminar ─────────────────────────────────────────────────
  let deletingId         = $state<number | null>(null);
  let deletingInProgress = $state<number | null>(null);

  // ── Selección múltiple ────────────────────────────────────────────────────
  let selectMode     = $state(false);
  let selectedIds    = $state<Set<number>>(new Set());
  let bulkConfirming = $state(false);
  let bulkDeleting   = $state(false);
  let bulkSuccessMsg = $state<string | null>(null);

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
    bulkDeleting   = true;
    bulkConfirming = false;
    const ids  = [...selectedIds];
    const prev = txs;
    txs = txs.filter(t => !selectedIds.has(t.id));
    exitSelectMode();
    try {
      const deleted = await transactionApi.removeBulk(ids);
      bumpTxVersion();
      bulkSuccessMsg = `${deleted} transacción${deleted !== 1 ? "es" : ""} eliminada${deleted !== 1 ? "s" : ""}.`;
      setTimeout(() => { bulkSuccessMsg = null; }, 3000);
      totalCount = Math.max(0, totalCount - ids.length);
    } catch (e) {
      txs = prev;
      console.error("[historial] bulk delete error:", e);
      error = "No se pudieron eliminar las transacciones. Intenta de nuevo.";
    } finally {
      bulkDeleting = false;
    }
  }

  // ── Exportar / Importar ───────────────────────────────────────────────────
  let exporting    = $state(false);
  let importing    = $state(false);
  let importResult = $state<ImportResult | null>(null);
  let reloadKey    = $state(0);
  let fileInputEl: HTMLInputElement | undefined = $state();

  // ── Utilidades ─────────────────────────────────────────────────────────────
  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function formatDate(d: string): string {
    const [, m, day] = d.split("-");
    return `${parseInt(day)} ${MESES_CORTO[parseInt(m) - 1]}`;
  }

  function formatDateLong(iso: string): string {
    const [y, m, d] = iso.split("-").map(Number);
    const dt = new Date(y, m - 1, d);
    return `${d} ${MESES_CORTO[m - 1]}, ${DIAS_SEMANA[dt.getDay()]}`;
  }

  function buildFilter() {
    return {
      period:      { type: activePeriod },
      kind:        filterKind   || null,
      category:    filterCat    || null,
      search_note: filterSearch || null,
      only_debt:   filterDebt   || null,
      page:        currentPage,
      page_size:   PAGE_SIZE,
    };
  }

  // ── Effects ────────────────────────────────────────────────────────────────

  let prevTxVersion = $state(txState.version);
  $effect(() => {
    if (txState.version !== prevTxVersion) {
      prevTxVersion = txState.version;
      currentPage = 1;
    }
  });

  $effect(() => {
    const _period = activePeriod;
    const _kind   = filterKind;
    const _cat    = filterCat;
    const _debt   = filterDebt;
    const _search = filterSearch;
    const _reload = reloadKey;
    const _v      = txState.version;
    const page    = currentPage;
    let cancelled = false;

    async function load() {
      loading = true;
      error   = null;
      try {
        const [result, cats, pSummary] = await Promise.all([
          transactionApi.list(buildFilter()),
          transactionApi.listCategories(),
          transactionApi.getPeriodSummary({ type: activePeriod }),
        ]);
        if (!cancelled) {
          txs           = result.transactions;
          totalCount    = result.total_count;
          categories    = cats;
          periodSummary = pSummary;
          loading       = false;
        }
      } catch (e) {
        if (!cancelled) {
          console.error("[historial] load error:", e);
          error   = "No se pudieron cargar las transacciones.";
          loading = false;
        }
      }
    }

    load();
    return () => { cancelled = true; };
  });

  // ── Edición ────────────────────────────────────────────────────────────────

  function startEdit(tx: Transaction) {
    editingTx    = tx;
    editKind     = tx.type as "ingreso" | "gasto";
    editDate     = tx.date;
    editNote     = tx.note ?? "";
    editExtraord = tx.is_extraordinary;
    editAmount   = tx.amount.toString();
    editError    = null;

    editCategory = tx.category;
  }

  function cancelEdit() { editingTx = null; }

  async function saveEdit() {
    if (!editingTx) return;
    const amt = parseInt(editAmount, 10);
    if (!amt || amt <= 0) { editError = "Monto inválido."; return; }
    editSaving = true;
    editError  = null;

    try {
      const input: TransactionInput = {
        date: editDate, type: editKind, category: editCategory,
        amount: amt, note: editNote.trim() || null,
        is_extraordinary: editExtraord, goal_id: editingTx.goal_id,
        gas_km: null, is_debt: editingTx.is_debt,
      };
      const updated = await transactionApi.update(editingTx.id, input);
      txs = txs.map(t => t.id === updated.id ? updated : t);
      editingTx = null;
      bumpTxVersion();
    } catch (e) {
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
    txs = txs.filter(t => t.id !== id);
    totalCount = Math.max(0, totalCount - 1);
    try {
      await transactionApi.remove(id);
      bumpTxVersion();
    } catch (e) {
      txs = prev;
      totalCount += 1;
      error = "No se pudo eliminar la transacción. Intenta de nuevo.";
    } finally {
      deletingInProgress = null;
    }
  }

  function triggerImport() { importResult = null; fileInputEl?.click(); }

  function handleImportFile(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    importing    = true;
    importResult = null;
    const reader = new FileReader();
    reader.onload = async (ev) => {
      const content = ev.target?.result as string;
      try {
        const result = await transactionApi.importCsv(content);
        importResult = result;
        if (result.imported > 0) { bumpTxVersion(); reloadKey += 1; currentPage = 1; }
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
      const result = await transactionApi.exportCsv({
        period: { type: activePeriod },
        kind: filterKind || null,
        category: filterCat || null,
        only_debt: filterDebt || null,
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

<div class="historial-shell">

  <!-- Toolbar -->
  <div class="toolbar">
    <h1>Historial</h1>
    <div class="toolbar-right">
      <div class="menu-wrap">
        <button
          class="action-btn icon-btn"
          onclick={() => { menuOpen = !menuOpen; }}
          aria-label="Menú acciones"
        >⋯</button>
        {#if menuOpen}
          <div
            class="menu-overlay"
            role="button"
            tabindex="-1"
            onclick={() => { menuOpen = false; }}
            onkeydown={() => {}}
          ></div>
          <div class="menu-dropdown">
            <button
              class="menu-item"
              onclick={() => { menuOpen = false; triggerImport(); }}
              disabled={importing}
            >{importing ? "Importando…" : "Importar CSV"}</button>
            <button
              class="menu-item"
              onclick={() => { menuOpen = false; exportCSV(); }}
              disabled={exporting || txs.length === 0}
            >{exporting ? "Exportando…" : "Exportar CSV"}</button>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <input
    type="file"
    accept=".csv,text/csv"
    bind:this={fileInputEl}
    onchange={handleImportFile}
    style="display:none"
  />

  <!-- Filtros -->
  {#if !selectMode}
  <div class="filters">
    <!-- Período -->
    <nav class="period-selector">
      {#each (Object.keys(PERIOD_LABELS) as PeriodKey[]) as key}
        <button
          class:active={activePeriod === key}
          onclick={() => { activePeriod = key; currentPage = 1; }}
        >{PERIOD_LABELS[key]}</button>
      {/each}
    </nav>

    <!-- Tipo: pills -->
    <div class="kind-pills">
      {#each [["", "Todos"], ["ingreso", "Ingresos"], ["gasto", "Gastos"]] as [val, label]}
        <button
          class="kind-pill"
          class:active={filterKind === val}
          class:income={val === "ingreso" && filterKind === val}
          class:expense={val === "gasto" && filterKind === val}
          onclick={() => { filterKind = val as typeof filterKind; currentPage = 1; }}
        >{label}</button>
      {/each}
    </div>

    <!-- Categoría -->
    <div class="filter-select-wrap">
      <CustomSelect
        value={filterCat}
        options={[
          { value: "", label: "Todas las categorías" },
          ...categories.map(c => ({ value: c, label: c })),
        ]}
        onchange={(v) => { filterCat = v; currentPage = 1; }}
      />
    </div>

    <!-- Deudas -->
    <button
      type="button"
      class="filter-pill"
      class:active={filterDebt}
      onclick={() => { filterDebt = !filterDebt; currentPage = 1; }}
    >Solo deudas</button>

    <!-- Búsqueda -->
    <div class="search-wrap">
      <input
        class="search-input"
        type="text"
        placeholder="Buscar en notas…"
        bind:value={filterSearch}
        oninput={() => { currentPage = 1; }}
      />
      {#if filterSearch}
        <button class="search-clear" onclick={() => { filterSearch = ""; currentPage = 1; }}>×</button>
      {/if}
    </div>
  </div>
  {/if}

  <!-- Stats bar -->
  {#if !loading && periodSummary}
    <div class="stats-bar">
      <span class="stat">
        <span class="stat-lbl">Ingresos</span>
        <span class="stat-val income">+{formatCOP(periodSummary.total_income)}</span>
      </span>
      <span class="stat-div">|</span>
      <span class="stat">
        <span class="stat-lbl">Gastos</span>
        <span class="stat-val expense">−{formatCOP(periodSummary.total_expenses)}</span>
      </span>
      <span class="stat-div">|</span>
      <span class="stat">
        <span class="stat-lbl">Balance</span>
        <span
          class="stat-val"
          class:income={periodSummary.balance >= 0}
          class:expense={periodSummary.balance < 0}
        >
          {periodSummary.balance >= 0 ? "+" : "−"}{formatCOP(Math.abs(periodSummary.balance))}
        </span>
      </span>
    </div>
  {/if}

  <!-- Banners -->
  {#if error}
    <div class="banner error"><strong>Error</strong><pre>{error}</pre></div>
  {/if}

  {#if bulkSuccessMsg}
    <div class="banner success">{bulkSuccessMsg}</div>
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

  <!-- Seleccionar -->
  {#if !selectMode && txs.length > 0}
    <div class="select-row">
      <button class="select-trigger" onclick={enterSelectMode}>Seleccionar</button>
    </div>
  {/if}

  <!-- Timeline / cuerpo -->
  {#if loading}
    <div class="timeline-wrap">
      <div class="placeholder-list">
        {#each [1,2,3,4,5,6,7,8] as _}
          <div class="placeholder-row"></div>
        {/each}
      </div>
    </div>
  {:else if grouped.length === 0}
    <div class="timeline-wrap empty-state">
      <p class="empty-msg">Sin transacciones en este período.</p>
    </div>
  {:else}
    <div class="timeline-wrap">
      {#each grouped as group (group.date)}
        {@const net = group.items.reduce((s, t) => s + (t.type === "ingreso" ? t.amount : -t.amount), 0)}
        <div class="date-group">
          <!-- Group header -->
          <button
            class="group-header"
            onclick={() => toggleGroup(group.date)}
          >
            <span class="group-chevron" class:collapsed={collapsedGroups.has(group.date)}>›</span>
            <span class="group-date">{formatDateLong(group.date)}</span>
            <span class="group-rule"></span>
            {#if collapsedGroups.has(group.date)}
              <span class="group-count">{group.items.length} transacción{group.items.length !== 1 ? "es" : ""}</span>
            {:else}
              <span class="group-net" class:income={net >= 0} class:expense={net < 0}>
                {net >= 0 ? "+" : "−"}{formatCOP(Math.abs(net))}
              </span>
            {/if}
          </button>

          <!-- Transaction rows -->
          {#if !collapsedGroups.has(group.date)}
            {#each group.items as tx (tx.id)}
              <div
                class="tx-row"
                class:selected={selectedIds.has(tx.id)}
                class:deleting={deletingInProgress === tx.id}
              >
                {#if selectMode}
                  <label class="tx-check-wrap">
                    <input
                      type="checkbox"
                      class="tx-check"
                      checked={selectedIds.has(tx.id)}
                      onchange={() => toggleSelect(tx.id)}
                    />
                  </label>
                {/if}

                <span
                  class="tx-badge"
                  class:badge-income={tx.type === "ingreso"}
                  class:badge-expense={tx.type === "gasto"}
                >
                  {tx.type === "ingreso" ? "↑" : "↓"}
                </span>

                <span class="tx-cat">{tx.category}</span>

                {#if tx.note}
                  <span class="tx-note">{tx.note}</span>
                {/if}

                {#if tx.is_debt}
                  <span class="tx-tag debt">deuda</span>
                {/if}

                {#if tx.is_extraordinary}
                  <span class="tx-tag extra" title="Extraordinario">✦</span>
                {/if}

                <span class="tx-gap"></span>

                <span
                  class="tx-amount"
                  class:income={tx.type === "ingreso"}
                  class:expense={tx.type === "gasto"}
                >
                  {tx.type === "ingreso" ? "+" : "−"}{formatCOP(tx.amount)}
                </span>

                {#if !selectMode}
                  <div class="tx-actions">
                    {#if deletingId === tx.id}
                      <span class="del-confirm-label">¿Eliminar?</span>
                      <button
                        class="act-btn danger"
                        onclick={() => confirmDelete(tx.id)}
                        disabled={deletingInProgress === tx.id}
                      >{deletingInProgress === tx.id ? "…" : "Sí"}</button>
                      <button
                        class="act-btn"
                        onclick={() => { deletingId = null; }}
                        disabled={deletingInProgress === tx.id}
                      >No</button>
                    {:else}
                      <button class="act-btn" onclick={() => startEdit(tx)}>Editar</button>
                      <button
                        class="act-btn danger"
                        onclick={() => { deletingId = tx.id; }}
                        disabled={deletingInProgress === tx.id}
                      >✕</button>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <!-- Selection bar (bottom, fixed when select mode active) -->
  {#if selectMode}
    <div class="selection-bar">
      <label class="sel-all-label">
        <input type="checkbox" class="sel-check" checked={allSelected} onchange={toggleSelectAll} />
        <span>Seleccionar todas</span>
      </label>
      <span class="sel-dot">·</span>
      <span class="sel-count">{selectedIds.size > 0 ? `${selectedIds.size} seleccionada${selectedIds.size !== 1 ? "s" : ""}` : "Ninguna"}</span>
      {#if bulkConfirming}
        <span class="sel-dot">·</span>
        <span class="sel-confirm-text">¿Eliminar {selectedIds.size}?</span>
        <button class="sel-btn danger" onclick={bulkDelete} disabled={bulkDeleting}>{bulkDeleting ? "Eliminando…" : "Confirmar"}</button>
        <button class="sel-btn" onclick={() => { bulkConfirming = false; }} disabled={bulkDeleting}>Atrás</button>
      {:else}
        <span class="sel-dot">·</span>
        <button class="sel-btn danger" onclick={() => { bulkConfirming = true; }} disabled={selectedIds.size === 0}>Eliminar seleccionadas</button>
      {/if}
      <span class="sel-spacer"></span>
      <button class="sel-btn secondary" onclick={exitSelectMode} disabled={bulkDeleting}>Cancelar</button>
    </div>
  {/if}

  <!-- Footer -->
  <div class="page-footer">
    <div class="footer-left">
      <span class="total-count">{totalCount} registros</span>
      {#if periodSummary !== null}
        {@const bal = periodSummary.balance}
        <span class="page-total" class:income={bal >= 0} class:expense={bal < 0}>
          Total período: {bal >= 0 ? "+" : "−"}{formatCOP(Math.abs(bal))}
        </span>
      {/if}
    </div>

    {#if totalPages > 1}
      <div class="pagination">
        <button class="page-btn" disabled={currentPage === 1} onclick={() => { currentPage -= 1; }}>←</button>

        {#if pageNumbers[0] > 1}
          <button class="page-btn" onclick={() => { currentPage = 1; }}>1</button>
          {#if pageNumbers[0] > 2}<span class="page-ellipsis">…</span>{/if}
        {/if}

        {#each pageNumbers as p}
          <button
            class="page-btn"
            class:active={p === currentPage}
            onclick={() => { currentPage = p; }}
          >{p}</button>
        {/each}

        {#if pageNumbers[pageNumbers.length - 1] < totalPages}
          {#if pageNumbers[pageNumbers.length - 1] < totalPages - 1}
            <span class="page-ellipsis">…</span>
          {/if}
          <button class="page-btn" onclick={() => { currentPage = totalPages; }}>{totalPages}</button>
        {/if}

        <button class="page-btn" disabled={currentPage === totalPages} onclick={() => { currentPage += 1; }}>→</button>
      </div>
    {/if}
  </div>
</div>

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
          <span class="field-label">Categoría</span>
          <CustomSelect
            bind:value={editCategory}
            options={categories.map(c => ({ value: c, label: c }))}
            placeholder="Selecciona categoría…"
          />
        </div>

        <div class="field">
          <label for="edit-amount">Monto</label>
          <input id="edit-amount" type="number" min="1" bind:value={editAmount} />
        </div>

        <div class="field">
          <span class="field-label">Fecha</span>
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
          disabled={editSaving}
        >
          {editSaving ? "Guardando…" : "Guardar"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .historial-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    padding: 0.875rem 1rem 0.75rem;
    gap: 0.55rem;
    box-sizing: border-box;
  }

  /* ── Toolbar ── */
  .toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-height: 32px;
  }

  h1 { font-size: 1.1rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.02em; }

  .toolbar-right { display: flex; align-items: center; gap: 0.4rem; }

  .action-btn {
    padding: 0.35rem 0.8rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: color 0.15s, background 0.15s;
  }
  .action-btn:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-surface); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .icon-btn { padding: 0.35rem 0.7rem; font-size: 1rem; letter-spacing: -0.1em; }

  /* ⋯ dropdown */
  .menu-wrap { position: relative; }

  .menu-overlay {
    position: fixed;
    inset: 0;
    z-index: 49;
    cursor: default;
  }

  .menu-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 50;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
    min-width: 148px;
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .menu-item {
    width: 100%;
    text-align: left;
    padding: 0.45rem 0.65rem;
    border-radius: 5px;
    font-size: 0.82rem;
    color: var(--text-secondary);
    transition: background 0.12s, color 0.12s;
  }
  .menu-item:hover:not(:disabled) { background: var(--bg-elevated); color: var(--text-primary); }
  .menu-item:disabled { opacity: 0.4; cursor: not-allowed; }


  /* ── Filtros ── */
  .filters {
    flex-shrink: 0;
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .period-selector {
    display: flex;
    gap: 3px;
    background: var(--bg-elevated);
    padding: 3px;
    border-radius: 7px;
  }

  .period-selector button {
    padding: 0.28rem 0.6rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }
  .period-selector button:hover { color: var(--text-primary); background: var(--bg-surface); }
  .period-selector button.active { background: var(--accent); color: #fff; }

  /* Kind pills */
  .kind-pills {
    display: flex;
    gap: 3px;
    background: var(--bg-elevated);
    padding: 3px;
    border-radius: 7px;
  }

  .kind-pill {
    padding: 0.28rem 0.65rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }
  .kind-pill:hover:not(.active) { color: var(--text-primary); background: var(--bg-surface); }
  .kind-pill.active { background: var(--bg-surface); color: var(--text-primary); }
  .kind-pill.active.income  { color: var(--success); }
  .kind-pill.active.expense { color: var(--danger); }

  .filter-select-wrap {
    font-size: 0.78rem;
    --cs-padding: 0.32rem 0.6rem;
  }

  .filter-pill {
    padding: 0.32rem 0.7rem;
    border-radius: var(--radius);
    font-size: 0.78rem;
    font-weight: 500;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: all 0.15s;
    white-space: nowrap;
  }
  .filter-pill:hover { color: var(--text-primary); }
  .filter-pill.active {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-elevated));
    border-color: color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }

  /* Search */
  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-input {
    -webkit-appearance: none;
    appearance: none;
    background: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.78rem;
    padding: 0.32rem 1.5rem 0.32rem 0.6rem;
    outline: none;
    width: 160px;
    transition: border-color 0.15s, width 0.2s;
  }
  .search-input:focus { border-color: var(--accent); width: 200px; }
  .search-input::placeholder { color: var(--text-muted); }

  .search-clear {
    position: absolute;
    right: 0.35rem;
    font-size: 0.9rem;
    color: var(--text-muted);
    padding: 0 0.15rem;
    line-height: 1;
    transition: color 0.15s;
  }
  .search-clear:hover { color: var(--text-primary); }

  /* ── Stats bar ── */
  .stats-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.78rem;
  }

  .stat { display: flex; align-items: center; gap: 0.4rem; }
  .stat-lbl { color: var(--text-muted); }
  .stat-val { font-weight: 600; font-variant-numeric: tabular-nums; color: var(--text-secondary); }
  .stat-val.income  { color: var(--success); }
  .stat-val.expense { color: var(--danger); }
  .stat-div { color: var(--border); font-size: 0.9rem; }

  /* ── Banners ── */
  .banner {
    flex-shrink: 0;
    border-radius: var(--radius);
    padding: 0.55rem 0.875rem;
    font-size: 0.82rem;
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
  .banner.warning {
    background: color-mix(in srgb, var(--warning) 12%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--warning) 40%, transparent);
    color: var(--warning);
  }
  .banner pre { font-size: 0.7rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }
  .import-errors { font-size: 0.72rem; opacity: 0.85; margin-top: 0.2rem; padding-left: 1.1rem; }

  /* ── Select row (above timeline) ── */
  .select-row {
    flex-shrink: 0;
    display: flex;
    justify-content: flex-end;
  }

  .select-trigger {
    font-size: 0.72rem;
    font-weight: 500;
    color: var(--text-muted);
    padding: 0.15rem 0.5rem;
    border-radius: var(--radius);
    border: 1px solid transparent;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .select-trigger:hover {
    color: var(--text-secondary);
    border-color: var(--border);
    background: var(--bg-elevated);
  }

  /* ── Selection bar (bottom) ── */
  .selection-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.875rem;
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-radius: var(--radius);
    font-size: 0.82rem;
  }

  .sel-all-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .sel-check {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border: 1.5px solid var(--border);
    border-radius: 3px;
    background: var(--bg-elevated);
    cursor: pointer;
    position: relative;
    flex-shrink: 0;
    transition: border-color 0.15s, background 0.15s;
  }
  .sel-check:checked { background: var(--accent); border-color: var(--accent); }
  .sel-check:checked::after {
    content: "";
    position: absolute;
    left: 3px; top: 0px;
    width: 4px; height: 8px;
    border: 2px solid #fff;
    border-top: none; border-left: none;
    transform: rotate(45deg);
  }

  .sel-dot { color: var(--text-muted); font-size: 0.9rem; flex-shrink: 0; }

  .sel-count {
    font-size: 0.78rem;
    color: var(--text-primary);
    font-weight: 600;
    white-space: nowrap;
  }

  .sel-confirm-text { font-size: 0.78rem; color: var(--text-secondary); white-space: nowrap; }

  .sel-spacer { flex: 1; }

  .sel-btn {
    padding: 0.28rem 0.65rem;
    border-radius: var(--radius);
    font-size: 0.75rem;
    font-weight: 500;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: color 0.15s, background 0.15s;
    white-space: nowrap;
  }
  .sel-btn:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-surface); }
  .sel-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .sel-btn.danger { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 40%, transparent); }
  .sel-btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 12%, var(--bg-elevated)); }
  .sel-btn.secondary { color: var(--text-muted); }

  /* ── Timeline ── */
  .timeline-wrap {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-msg { color: var(--text-muted); font-size: 0.85rem; }

  /* ── Date group ── */
  .date-group { display: flex; flex-direction: column; }

  .group-header {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.75rem;
    background: var(--bg-base);
    border-bottom: 1px solid var(--border);
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.12s;
    text-align: left;
    width: 100%;
  }
  .group-header:hover { background: var(--bg-elevated); }

  .group-chevron {
    display: inline-block;
    font-size: 0.85rem;
    color: var(--text-muted);
    transition: transform 0.15s;
    transform: rotate(90deg);
  }
  .group-chevron.collapsed { transform: rotate(0deg); }

  .group-date { white-space: nowrap; color: var(--text-secondary); font-weight: 600; }

  .group-rule {
    flex: 1;
    height: 1px;
    background: var(--border);
    opacity: 0.5;
  }

  .group-net {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    white-space: nowrap;
    font-size: 0.75rem;
  }
  .group-net.income  { color: var(--success); }
  .group-net.expense { color: var(--danger); }

  .group-count {
    color: var(--text-muted);
    font-weight: 500;
    white-space: nowrap;
  }

  /* ── Transaction row ── */
  .tx-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
    min-height: 38px;
  }
  .tx-row:last-child { border-bottom: none; }
  .tx-row:hover { background: var(--bg-elevated); }
  .tx-row.selected { background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .tx-row.deleting { opacity: 0.4; pointer-events: none; }

  /* Checkbox (select mode) */
  .tx-check-wrap { display: flex; align-items: center; flex-shrink: 0; }
  .tx-check {
    -webkit-appearance: none;
    appearance: none;
    width: 15px;
    height: 15px;
    border: 1.5px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elevated);
    cursor: pointer;
    position: relative;
    flex-shrink: 0;
    transition: border-color 0.15s, background 0.15s;
  }
  .tx-check:hover { border-color: var(--accent); }
  .tx-check:checked { background: var(--accent); border-color: var(--accent); }
  .tx-check:checked::after {
    content: "";
    position: absolute;
    left: 4px; top: 1px;
    width: 4px; height: 8px;
    border: 2px solid #fff;
    border-top: none; border-left: none;
    transform: rotate(45deg);
  }

  /* Badge (↑ / ↓) */
  .tx-badge {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    font-size: 0.7rem;
    font-weight: 700;
  }
  .badge-income  { background: color-mix(in srgb, var(--success) 18%, var(--bg-elevated)); color: var(--success); }
  .badge-expense { background: color-mix(in srgb, var(--danger)  18%, var(--bg-elevated)); color: var(--danger); }

  /* Category */
  .tx-cat {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* Note */
  .tx-note {
    font-size: 0.78rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex-shrink: 1;
    min-width: 0;
  }

  /* Tags (deuda, extraordinario) */
  .tx-tag {
    flex-shrink: 0;
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .tx-tag.debt  { background: color-mix(in srgb, var(--danger) 15%, transparent); color: var(--danger); border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); }
  .tx-tag.extra { background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); font-size: 0.58rem; }

  /* Gap / spacer */
  .tx-gap { flex: 1; min-width: 0.25rem; }

  /* Amount */
  .tx-amount {
    flex-shrink: 0;
    font-size: 0.85rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tx-amount.income  { color: var(--success); }
  .tx-amount.expense { color: var(--danger); }

  /* Hover actions */
  .tx-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.15rem;
    opacity: 0;
    transition: opacity 0.15s;
    min-width: 80px;
    justify-content: flex-end;
  }
  .tx-row:hover .tx-actions { opacity: 1; }

  .act-btn {
    font-size: 0.72rem;
    color: var(--text-muted);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
    white-space: nowrap;
  }
  .act-btn:hover { color: var(--text-primary); background: var(--bg-surface); }
  .act-btn.danger:hover { color: var(--danger); }
  .act-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .del-confirm-label { font-size: 0.72rem; color: var(--text-secondary); white-space: nowrap; }

  /* ── Placeholders ── */
  .placeholder-list { display: flex; flex-direction: column; gap: 0.4rem; padding: 0.6rem; }
  .placeholder-row  { height: 36px; border-radius: var(--radius); background: var(--bg-surface); animation: shimmer 1.4s ease-in-out infinite; }
  @keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.7; } }

  /* ── Footer ── */
  .page-footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border);
    min-height: 36px;
  }

  .footer-left { display: flex; align-items: center; gap: 1rem; font-size: 0.78rem; }
  .total-count { color: var(--text-muted); }
  .page-total  { font-weight: 600; font-variant-numeric: tabular-nums; color: var(--text-secondary); }
  .page-total.income  { color: var(--success); }
  .page-total.expense { color: var(--danger); }

  /* ── Paginación ── */
  .pagination { display: flex; align-items: center; gap: 2px; }

  .page-btn {
    min-width: 28px; height: 28px;
    border-radius: 5px; font-size: 0.78rem; font-weight: 500;
    padding: 0 0.4rem;
    background: var(--bg-elevated); border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
    display: flex; align-items: center; justify-content: center;
  }
  .page-btn:hover:not(:disabled) { background: var(--bg-surface); color: var(--text-primary); }
  .page-btn:disabled { opacity: 0.35; cursor: not-allowed; }
  .page-btn.active { background: var(--accent); border-color: var(--accent); color: #fff; }
  .page-ellipsis { color: var(--text-muted); font-size: 0.78rem; padding: 0 0.1rem; }

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
  input[type="number"] {
    -webkit-appearance: none; appearance: none;
    background-color: #1c1c2e; border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0; font: inherit; font-size: 0.875rem;
    padding: 0.5rem 0.65rem; outline: none; width: 100%;
  }

  input:focus { border-color: var(--accent); }

  .checkbox-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.875rem; color: var(--text-secondary); cursor: pointer; }
  .checkbox-row input { accent-color: var(--accent); }

  .type-toggle { display: grid; grid-template-columns: 1fr 1fr; background: var(--bg-elevated); border-radius: var(--radius); padding: 3px; gap: 3px; }
  .toggle-btn { padding: 0.45rem; border-radius: 5px; font-size: 0.85rem; font-weight: 600; color: var(--text-secondary); transition: background 0.15s, color 0.15s; }
  .toggle-btn.income.active  { background: color-mix(in srgb, var(--success) 20%, var(--bg-surface)); color: var(--success); }
  .toggle-btn.expense.active { background: color-mix(in srgb, var(--danger)  20%, var(--bg-surface)); color: var(--danger); }


  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; padding-top: 0.25rem; }

  .btn-cancel {
    padding: 0.5rem 1rem; background: var(--bg-elevated); border: 1px solid var(--border);
    border-radius: var(--radius); font-size: 0.85rem; color: var(--text-secondary);
  }
  .btn-cancel:hover { color: var(--text-primary); }

  .btn-save {
    padding: 0.5rem 1rem; background: var(--accent); border-radius: var(--radius);
    font-size: 0.85rem; font-weight: 600; color: #fff;
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-save:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-save:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
