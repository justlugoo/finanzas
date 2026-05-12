<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { GoalWithProgress, GoalDetail } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";

  let goals       = $state<GoalWithProgress[]>([]);
  let loading     = $state(true);
  let pageError   = $state<string | null>(null);

  // ── Filtro ────────────────────────────────────────────────────────────────
  let filterStatus = $state<string>("todos");

  let filtered = $derived(
    filterStatus === "todos"
      ? goals
      : goals.filter(g => g.goal.status === filterStatus)
  );

  // ── Crear ─────────────────────────────────────────────────────────────────
  let createOpen  = $state(false);
  let creating    = $state(false);
  let cName       = $state("");
  let cAmountRaw  = $state("");
  let cDate       = $state("");
  let cError      = $state<string | null>(null);

  let cAmount = $derived(parseInt(cAmountRaw.replace(/\D/g, ""), 10) || 0);

  // ── Editar ────────────────────────────────────────────────────────────────
  let editGoal    = $state<GoalWithProgress | null>(null);
  let editing     = $state(false);
  let eName       = $state("");
  let eAmountRaw  = $state("");
  let eDate       = $state("");
  let eStatus     = $state("activo");
  let eError      = $state<string | null>(null);

  let eAmount = $derived(parseInt(eAmountRaw.replace(/\D/g, ""), 10) || 0);

  // ── Detalle ───────────────────────────────────────────────────────────────
  let detail      = $state<GoalDetail | null>(null);
  let detailLoading = $state(false);

  // ── Eliminar ──────────────────────────────────────────────────────────────
  let deleteId    = $state<number | null>(null);
  let deleting    = $state(false);

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

  function barColor(g: GoalWithProgress): string {
    if (g.goal.status === "completado" || g.percentage >= 100) return "var(--success)";
    if (g.goal.status === "pausado") return "var(--text-muted)";
    if (!g.on_track && g.goal.target_date) return "#f59e0b";
    return "var(--accent)";
  }

  function statusLabel(s: string): string {
    return { activo: "Activo", completado: "Completado", pausado: "Pausado" }[s] ?? s;
  }

  // ── Carga ─────────────────────────────────────────────────────────────────
  async function loadGoals() {
    loading = true; pageError = null;
    try {
      goals = await invoke<GoalWithProgress[]>("list_goals", {});
    } catch (e) {
      console.error("[objetivos] load error:", e);
      pageError = "No se pudieron cargar los objetivos.";
    } finally {
      loading = false;
    }
  }

  $effect(() => { loadGoals(); });

  // ── Crear ─────────────────────────────────────────────────────────────────
  async function handleCreate(ev: Event) {
    ev.preventDefault();
    if (!cName.trim()) { cError = "El nombre no puede estar vacío."; return; }
    if (cAmount <= 0)  { cError = "El monto debe ser mayor que 0."; return; }
    creating = true; cError = null;
    try {
      const g = await invoke<GoalWithProgress>("create_goal", {
        input: { name: cName.trim(), target_amount: cAmount, target_date: cDate || null, status: null },
      });
      goals = [...goals, g].sort((a, b) => a.goal.name.localeCompare(b.goal.name));
      createOpen = false; cName = ""; cAmountRaw = ""; cDate = "";
    } catch (e) {
      console.error("[objetivos] create error:", e);
      cError = "No se pudo crear el objetivo. Intenta de nuevo.";
    } finally {
      creating = false;
    }
  }

  // ── Editar ────────────────────────────────────────────────────────────────
  function openEdit(g: GoalWithProgress, ev: Event) {
    ev.stopPropagation();
    editGoal = g;
    eName = g.goal.name;
    eAmountRaw = g.goal.target_amount.toString();
    eDate = g.goal.target_date ?? "";
    eStatus = g.goal.status;
    eError = null;
  }

  async function handleEdit(ev: Event) {
    ev.preventDefault();
    if (!editGoal) return;
    if (!eName.trim()) { eError = "El nombre no puede estar vacío."; return; }
    if (eAmount <= 0)  { eError = "El monto debe ser mayor que 0."; return; }
    editing = true; eError = null;
    try {
      const updated = await invoke<GoalWithProgress>("update_goal", {
        id: editGoal.goal.id,
        input: { name: eName.trim(), target_amount: eAmount, target_date: eDate || null, status: eStatus },
      });
      goals = goals.map(g => g.goal.id === updated.goal.id ? updated : g);
      editGoal = null;
    } catch (e) {
      console.error("[objetivos] edit error:", e);
      eError = "No se pudo guardar el objetivo. Intenta de nuevo.";
    } finally {
      editing = false;
    }
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
      await invoke("delete_goal", { id: deleteId });
      goals = goals.filter(g => g.goal.id !== deleteId);
      if (detail?.goal.goal.id === deleteId) detail = null;
      deleteId = null;
    } catch (e) {
      console.error("[objetivos] delete error:", e);
      pageError = "No se pudo eliminar el objetivo. Intenta de nuevo.";
    } finally {
      deleting = false;
    }
  }

  // ── Detalle ───────────────────────────────────────────────────────────────
  async function openDetail(id: number) {
    if (editGoal || deleteId !== null) return;
    detailLoading = true;
    try {
      detail = await invoke<GoalDetail>("get_goal_detail", { id });
    } catch (e) {
      console.error("[objetivos] detail error:", e);
      pageError = "No se pudo cargar el detalle. Intenta de nuevo.";
    } finally {
      detailLoading = false;
    }
  }
</script>

<main>
  <div class="header">
    <h1>Objetivos</h1>
    <button class="btn-primary" onclick={() => { createOpen = true; }}>+ Nuevo</button>
  </div>

  {#if pageError}
    <div class="banner error"><strong>Error</strong><pre>{pageError}</pre></div>
  {/if}

  <!-- Filtro de estado -->
  <div class="filter-row">
    {#each ["todos", "activo", "completado", "pausado"] as s}
      <button
        class="filter-btn"
        class:active={filterStatus === s}
        onclick={() => { filterStatus = s; }}
      >
        {s === "todos" ? "Todos" : statusLabel(s)}
      </button>
    {/each}
  </div>

  {#if loading}
    <p class="muted">Cargando…</p>
  {:else if filtered.length === 0}
    <div class="empty">
      <p>Sin objetivos{filterStatus !== "todos" ? ` con estado "${statusLabel(filterStatus)}"` : ""}.</p>
      {#if filterStatus === "todos"}
        <button class="btn-primary" onclick={() => { createOpen = true; }}>Crear el primero</button>
      {/if}
    </div>
  {:else}
    <div class="goal-grid">
      {#each filtered as g (g.goal.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="goal-card" onclick={() => openDetail(g.goal.id)}>
          <div class="card-top">
            <div class="card-title-row">
              <span class="goal-name">{g.goal.name}</span>
              {#if g.goal.is_debt_goal}
                <span class="debt-badge">DEUDA</span>
              {/if}
            </div>
            {#if g.goal.status === "activo" && !g.on_track}
              <span class="status-badge status-off-track">Atrasado</span>
            {:else}
              <span class="status-badge status-{g.goal.status}">{statusLabel(g.goal.status)}</span>
            {/if}
          </div>

          <div class="progress-wrap">
            <div class="progress-bar">
              <div
                class="progress-fill"
                style="width: {Math.min(g.percentage, 100)}%; background: {barColor(g)}"
              ></div>
            </div>
            <span class="pct">{g.percentage.toFixed(0)}%</span>
          </div>

          <div class="amounts">
            <span class="current">{formatCOP(g.current_amount)}</span>
            <span class="sep">/</span>
            <span class="target">{formatCOP(g.goal.target_amount)}</span>
            {#if g.goal.is_debt_goal}
              <span class="debt-label">abonado</span>
            {/if}
          </div>

          {#if g.goal.target_date}
            <div class="meta-row">
              <span class="meta-label">Meta:</span>
              <span class="meta-value">{g.goal.target_date}</span>
              {#if g.monthly_required}
                <span class="meta-sep">·</span>
                <span class="meta-label">Mensual:</span>
                <span class="meta-value">{formatCOP(g.monthly_required)}</span>
              {/if}
            </div>
          {/if}

          {#if !g.on_track && g.goal.status === "activo" && g.projected_completion_date}
            <div class="track-row">
              <span class="proj">Proyección: {g.projected_completion_date}</span>
            </div>
          {/if}

          <div class="card-actions">
            <button class="btn-icon" onclick={(ev) => openEdit(g, ev)} title="Editar">✎</button>
            <button class="btn-icon danger" onclick={(ev) => confirmDelete(g.goal.id, ev)} title="Eliminar">✕</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</main>

<!-- ── Modal: Crear ─────────────────────────────────────────────────────── -->
{#if createOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { createOpen = false; }}></div>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>Nuevo objetivo</h2>
    {#if cError}
      <div class="banner error small"><pre>{cError}</pre></div>
    {/if}
    <form onsubmit={handleCreate} class="modal-form">
      <div class="field">
        <label for="c-name">Nombre</label>
        <input id="c-name" type="text" bind:value={cName} placeholder="Ej: Laptop, Viaje…" maxlength="100" />
      </div>
      <div class="field">
        <label for="c-amount">Monto objetivo</label>
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
        <label>Fecha límite <span class="optional">(opcional)</span></label>
        <DatePicker bind:value={cDate} />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn-secondary" onclick={() => { createOpen = false; }}>Cancelar</button>
        <button type="submit" class="btn-primary" disabled={creating || cAmount <= 0 || !cName.trim()}>
          {creating ? "Creando…" : "Crear"}
        </button>
      </div>
    </form>
  </div>
{/if}

<!-- ── Modal: Editar ────────────────────────────────────────────────────── -->
{#if editGoal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { editGoal = null; }}></div>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>Editar objetivo</h2>
    {#if eError}
      <div class="banner error small"><pre>{eError}</pre></div>
    {/if}
    <form onsubmit={handleEdit} class="modal-form">
      <div class="field">
        <label for="e-name">Nombre</label>
        <input id="e-name" type="text" bind:value={eName} maxlength="100" />
      </div>
      <div class="field">
        <label for="e-amount">Monto objetivo</label>
        <input
          id="e-amount"
          type="text"
          inputmode="numeric"
          value={eAmountRaw ? new Intl.NumberFormat("es-CO").format(eAmount) : ""}
          oninput={(e) => handleAmountInput(e, (v) => { eAmountRaw = v; })}
        />
      </div>
      <div class="field">
        <label>Fecha límite <span class="optional">(opcional)</span></label>
        <DatePicker bind:value={eDate} />
      </div>
      <div class="field">
        <label>Estado</label>
        <CustomSelect
          bind:value={eStatus}
          options={[
            { value: "activo",     label: "Activo" },
            { value: "pausado",    label: "Pausado" },
            { value: "completado", label: "Completado" },
          ]}
        />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn-secondary" onclick={() => { editGoal = null; }}>Cancelar</button>
        <button type="submit" class="btn-primary" disabled={editing || eAmount <= 0 || !eName.trim()}>
          {editing ? "Guardando…" : "Guardar"}
        </button>
      </div>
    </form>
  </div>
{/if}

<!-- ── Modal: Detalle ───────────────────────────────────────────────────── -->
{#if detail || detailLoading}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { detail = null; }}></div>
  <div class="modal modal-wide" role="dialog" aria-modal="true" tabindex="-1">
    {#if detailLoading}
      <p class="muted">Cargando…</p>
    {:else if detail}
      <h2>{detail.goal.goal.name}</h2>
      <div class="detail-stats">
        <div class="stat">
          <span class="stat-label">Acumulado</span>
          <span class="stat-value">{formatCOP(detail.goal.current_amount)}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Meta</span>
          <span class="stat-value">{formatCOP(detail.goal.goal.target_amount)}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Progreso</span>
          <span class="stat-value">{detail.goal.percentage.toFixed(1)}%</span>
        </div>
        {#if detail.goal.monthly_required}
          <div class="stat">
            <span class="stat-label">Mensual req.</span>
            <span class="stat-value">{formatCOP(detail.goal.monthly_required)}</span>
          </div>
        {/if}
      </div>

      <h3>Contribuciones ({detail.contributions.length})</h3>
      {#if detail.contributions.length === 0}
        <p class="muted">Sin transacciones asociadas aún.</p>
      {:else}
        <div class="contrib-table-wrap">
          <table class="contrib-table">
            <thead>
              <tr>
                <th>Fecha</th>
                <th>Categoría</th>
                <th class="right">Monto</th>
                <th>Nota</th>
              </tr>
            </thead>
            <tbody>
              {#each detail.contributions as tx (tx.id)}
                <tr>
                  <td>{tx.date}</td>
                  <td>{tx.category}</td>
                  <td class="right amount-cell">{formatCOP(tx.amount)}</td>
                  <td class="note-cell">{tx.note ?? "—"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => { detail = null; }}>Cerrar</button>
      </div>
    {/if}
  </div>
{/if}

<!-- ── Confirm: Eliminar ─────────────────────────────────────────────────── -->
{#if deleteId !== null}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { deleteId = null; }}></div>
  <div class="modal modal-sm" role="dialog" aria-modal="true" tabindex="-1">
    <h2>¿Eliminar objetivo?</h2>
    <p class="muted">Las transacciones asociadas quedarán sin objetivo.</p>
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

  h1 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); margin-bottom: 1rem; }
  h3 { font-size: 0.85rem; font-weight: 600; color: var(--text-secondary); margin: 1rem 0 0.5rem; }

  /* ── Filtro ── */
  .filter-row {
    flex-shrink: 0;
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
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

  .filter-btn.active {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-elevated));
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  /* ── Cards ── */
  .goal-grid {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1rem;
    align-content: start;
    padding: 0.25rem;
  }

  .goal-card {
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

  .goal-card:hover { border-color: var(--accent); }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    flex: 1;
  }

  .goal-name { font-size: 0.95rem; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .debt-badge {
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--danger) 15%, transparent);
    color: var(--danger);
    border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .debt-label {
    font-size: 0.72rem;
    color: var(--danger);
    font-weight: 500;
  }

  .status-badge {
    font-size: 0.65rem;
    font-weight: 600;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    white-space: nowrap;
  }
  .status-activo     { background: color-mix(in srgb, var(--accent) 20%, transparent);  color: var(--accent);  }
  .status-completado { background: color-mix(in srgb, var(--success) 20%, transparent); color: var(--success); }
  .status-pausado    { background: color-mix(in srgb, var(--text-muted) 20%, transparent); color: var(--text-muted); }
  .status-off-track  { background: color-mix(in srgb, #f59e0b 20%, transparent); color: #f59e0b; }

  /* ── Progreso ── */
  .progress-wrap { display: flex; align-items: center; gap: 0.5rem; }

  .progress-bar {
    flex: 1;
    height: 6px;
    background: var(--bg-elevated);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: 999px;
    transition: width 0.3s ease;
  }

  .pct { font-size: 0.72rem; color: var(--text-muted); min-width: 2.5rem; text-align: right; }

  /* ── Montos ── */
  .amounts { display: flex; align-items: baseline; gap: 0.3rem; font-size: 0.85rem; }
  .current { color: var(--text-primary); font-weight: 600; }
  .sep, .target { color: var(--text-muted); }

  /* ── Meta row ── */
  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    font-size: 0.72rem;
  }
  .meta-label { color: var(--text-muted); }
  .meta-value { color: var(--text-secondary); }
  .meta-sep   { color: var(--border); }

  /* ── Track ── */
  .track-row { font-size: 0.72rem; }
  .on-track  { color: var(--success); font-weight: 500; }
  .off-track { color: #f59e0b; font-weight: 500; }
  .proj      { color: var(--text-muted); }

  /* ── Card actions ── */
  .card-actions {
    display: flex;
    gap: 0.4rem;
    justify-content: flex-end;
    margin-top: 0.25rem;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    font-size: 0.8rem;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }
  .btn-icon:hover { background: color-mix(in srgb, var(--accent) 20%, var(--bg-elevated)); color: var(--accent); }
  .btn-icon.danger:hover { background: color-mix(in srgb, var(--danger) 20%, var(--bg-elevated)); color: var(--danger); }

  /* ── Botones globales ── */
  .btn-primary {
    padding: 0.45rem 1rem;
    background: var(--accent);
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    border-radius: var(--radius);
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-secondary {
    padding: 0.45rem 1rem;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    transition: background 0.15s;
  }
  .btn-secondary:hover { background: var(--bg-surface); }

  .btn-danger {
    padding: 0.45rem 1rem;
    background: color-mix(in srgb, var(--danger) 80%, transparent);
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    border-radius: var(--radius);
    transition: opacity 0.15s;
  }
  .btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Banners ── */
  .banner {
    border-radius: var(--radius);
    padding: 0.65rem 1rem;
    font-size: 0.85rem;
  }
  .banner.error {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    color: var(--danger);
  }
  .banner.small { padding: 0.4rem 0.75rem; margin-bottom: 0.5rem; }
  .banner pre { font-size: 0.72rem; white-space: pre-wrap; word-break: break-all; }

  /* ── Modal ── */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    z-index: 20;
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.5rem;
    z-index: 21;
    width: min(440px, 92vw);
    max-height: 85vh;
    overflow-y: auto;
  }

  .modal-wide { width: min(600px, 96vw); }
  .modal-sm   { width: min(340px, 92vw); }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 0.5rem;
  }

  /* ── Campos ── */
  .field { display: flex; flex-direction: column; gap: 0.3rem; }

  label {
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .optional { font-weight: 400; color: var(--text-muted); }

  input[type="text"] {
    -webkit-appearance: none;
    appearance: none;
    background-color: #14141f;
    border: 1px solid #2a2a40;
    border-radius: var(--radius);
    color: #e8e8f0;
    font: inherit;
    font-size: 0.9rem;
    padding: 0.5rem 0.75rem;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
  }

  input:focus { border-color: var(--accent); }

  /* ── Detalle ── */
  .detail-stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .stat {
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 0.6rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .stat-label { font-size: 0.7rem; color: var(--text-muted); }
  .stat-value { font-size: 0.9rem; font-weight: 600; color: var(--text-primary); }

  .contrib-table-wrap { overflow-x: auto; }

  .contrib-table {
    width: 100%;
    font-size: 0.8rem;
    border-collapse: collapse;
  }

  .contrib-table th,
  .contrib-table td {
    padding: 0.4rem 0.5rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .contrib-table th { color: var(--text-muted); font-weight: 500; font-size: 0.72rem; }
  .contrib-table td { color: var(--text-secondary); }

  .right       { text-align: right; }
  .amount-cell { color: var(--text-primary); font-weight: 500; }
  .note-cell   { color: var(--text-muted); max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* ── Misc ── */
  .muted { color: var(--text-muted); font-size: 0.85rem; }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 2rem;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
</style>
