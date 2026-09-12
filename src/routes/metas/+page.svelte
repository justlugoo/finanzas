<script lang="ts">
  import { metaApi, metaPaymentApi, loanApi, goalApi, categoryApi } from "$lib/api";
  import type { Category, MetaV2 } from "$lib/types";
  import type { StatEntry, PaymentItem } from "$lib/components/PaymentModal.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";
  import PaymentModal from "$lib/components/PaymentModal.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import TourPoint from "$lib/components/TourPoint.svelte";
  import { MESES_CORTO } from "$lib/constants";
  import { isActiveStep } from "$lib/tour.svelte";

  let metas     = $state<MetaV2[]>([]);
  let loading   = $state(true);
  let pageError = $state<string | null>(null);
  let detail    = $state<MetaV2 | null>(null);

  let filterTipo   = $state("todas");
  let filterEstado = $state("todos");

  let tipoFiltered = $derived(
    filterTipo === "todas" ? metas : metas.filter(m => m.tipo === filterTipo)
  );

  // Pendientes por sub-sección (orden fijo: Deudas → Préstamos → Ahorros).
  // Dentro de cada sub-sección, las más avanzadas van primero — ver el
  // progreso cercano a completarse arriba es, en sí mismo, un empujón para
  // terminarlas, sin necesidad de copy motivacional de más.
  let byProgressDesc = (a: MetaV2, b: MetaV2) => pct(b) - pct(a);
  let pendingDebts   = $derived(tipoFiltered.filter(m => m.tipo === "debo"          && m.estado === "pendiente").sort(byProgressDesc));
  let pendingLoans   = $derived(tipoFiltered.filter(m => m.tipo === "me_deben"      && m.estado === "pendiente").sort(byProgressDesc));
  let pendingSavings = $derived(tipoFiltered.filter(m => m.tipo === "quiero_juntar" && m.estado === "pendiente").sort(byProgressDesc));
  let doneMetas      = $derived(tipoFiltered.filter(m => m.estado === "completado"));

  let allPendingCount = $derived(pendingDebts.length + pendingLoans.length + pendingSavings.length);

  // ── Abonado este mes — único dato "de ánimo" del header, y solo aparece
  // cuando hay algo real que mostrar (silencio es mejor que un "$0" deprimente). ──
  let thisMonthPaid = $derived.by(() => {
    const now = new Date();
    const ym = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
    return metas.reduce((sum, m) => sum + m.abonos.filter(a => a.date.startsWith(ym)).reduce((s, a) => s + a.amount, 0), 0);
  });

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

  function pct(m: MetaV2): number {
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

  // Reutiliza solo la paleta funcional del sistema (éxito/peligro/acento) —
  // nada de colores decorativos extra por tipo de meta.
  function tipoAccent(tipo: string): string {
    if (tipo === "me_deben")      return "var(--success)"; /* por cobrar, vuelve a mí */
    if (tipo === "debo")          return "var(--danger)";  /* pasivo, lo debo */
    if (tipo === "quiero_juntar") return "var(--accent)";  /* meta positiva */
    return "var(--text-secondary)";
  }

  function buildStats(m: MetaV2): StatEntry[] {
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
    if (m.fecha) stats.push({ label: "Fecha", value: formatDateShort(m.fecha) });
    if (m.cuotas !== null && m.cuotas > 0 && m.tipo === "debo") {
      stats.push({ label: "Cuotas", value: `${m.cuotas} cuotas` });
    }
    const streak = monthStreak(m);
    if (!done && streak >= 2) {
      stats.push({ label: "Constancia", value: `${streak} meses seguidos`, colorClass: "accent" });
    }
    return stats;
  }

  function toPaymentItems(m: MetaV2): PaymentItem[] {
    return m.abonos.map(a => ({ id: a.id, date: a.date, amount: a.amount }));
  }

  function formatDateShort(iso: string): string {
    const [, m, d] = iso.split("-");
    return `${parseInt(d)} ${MESES_CORTO[parseInt(m) - 1]}`;
  }

  // Días hasta una fecha meta (solo aplica a "quiero_juntar" — para deudas
  // y préstamos `fecha` es el origen, no un objetivo a futuro).
  function daysUntil(iso: string): number {
    const target = new Date(iso + "T00:00:00");
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return Math.round((target.getTime() - now.getTime()) / 86_400_000);
  }

  // Meses consecutivos con al menos un abono, contando hacia atrás desde el
  // mes actual (o desde el último mes con abono, si este mes aún no tiene
  // uno — así un día 1 de mes sin abonar todavía no rompe la racha visible).
  function monthStreak(m: MetaV2): number {
    if (m.abonos.length === 0) return 0;
    const months = new Set(m.abonos.map(a => a.date.slice(0, 7)));
    const now = new Date();
    let y = now.getFullYear();
    let mo = now.getMonth() + 1;
    const key = () => `${y}-${String(mo).padStart(2, "0")}`;
    if (!months.has(key())) {
      mo -= 1;
      if (mo === 0) { mo = 12; y -= 1; }
    }
    let streak = 0;
    while (months.has(key())) {
      streak++;
      mo -= 1;
      if (mo === 0) { mo = 12; y -= 1; }
    }
    return streak;
  }

  // ── Crear ─────────────────────────────────────────────────────────────────
  const tipoOptions = [
    { value: "me_deben",      label: "Préstamos" },
    { value: "debo",          label: "Deudas"    },
    { value: "quiero_juntar", label: "Ahorros"   },
  ];

  let createOpen   = $state(false);
  let creating     = $state(false);
  let cTipo        = $state("");
  let cNombre      = $state("");
  let cAmountRaw   = $state("");
  let cDate        = $state("");
  let cTargetDate  = $state("");
  let cNota        = $state("");
  let cCategoryId  = $state("");
  let cCuotasRaw   = $state("");
  let cError       = $state<string | null>(null);
  let cAmount      = $derived(parseInt(cAmountRaw.replace(/\D/g, ""), 10) || 0);
  let cCuotas      = $derived(parseInt(cCuotasRaw.replace(/\D/g, ""), 10) || 0);

  // Categorías de gasto — una deuda ("comprar a crédito") necesita una
  // categoría real, igual que cualquier otro gasto (sección 4 de schema-v2.md).
  let expenseCategories = $state<Category[]>([]);
  let _catsLoaded = false;
  async function loadExpenseCategories() {
    if (_catsLoaded) return;
    _catsLoaded = true;
    try {
      expenseCategories = await categoryApi.list("expense");
    } catch (e) {
      console.error("[metas] load categories:", e);
    }
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

  function extractMsg(e: unknown): string {
    if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
    return "Error desconocido. Intenta de nuevo.";
  }

  function resetCreate() {
    cTipo = ""; cNombre = ""; cAmountRaw = ""; cDate = ""; cTargetDate = ""; cNota = "";
    cCategoryId = ""; cCuotasRaw = ""; cError = null;
  }

  async function handleCreate(ev: Event) {
    ev.preventDefault();
    if (!cTipo)          { cError = "Selecciona el tipo de meta."; return; }
    if (!cNombre.trim()) { cError = "El nombre no puede estar vacío."; return; }
    if (cAmount <= 0)    { cError = "El monto debe ser mayor que 0."; return; }
    if (cTipo === "me_deben" && !cDate) { cError = "La fecha del préstamo es requerida."; return; }
    if (cTipo === "debo") {
      if (!cDate)        { cError = "La fecha de la deuda es requerida."; return; }
      if (!cCategoryId)  { cError = "Selecciona una categoría."; return; }
    }
    creating = true; cError = null;
    try {
      if (cTipo === "me_deben") {
        await loanApi.create({
          person_name: cNombre.trim(),
          principal_cop: cAmount,
          lent_on: cDate,
          note: cNota.trim() || null,
        });
      } else if (cTipo === "debo") {
        await goalApi.createDebt({
          name: cNombre.trim(),
          target_cop: cAmount,
          occurred_on: cDate,
          category_id: cCategoryId,
          installments: cCuotas > 0 ? cCuotas : null,
          note: cNota.trim() || null,
        });
      } else {
        await goalApi.create({
          name: cNombre.trim(),
          target_cop: cAmount,
          target_date: cTargetDate || null,
          type: "saving",
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
  // Un solo camino para las tres clases de meta — el frontend ya no decide
  // si eso es un ingreso o un gasto (paso 6, sección 12 de schema-v2.md).
  async function handleAddPayment(amount: number, date: string) {
    if (!detail) return;
    const metaId = detail.id;
    await metaPaymentApi.addPayment({ meta_id: metaId, amount_cop: amount, occurred_on: date });
    await loadMetas();
    detail = metas.find(m => m.id === metaId) ?? null;
  }

  // ── Editar (todos los tipos) ──────────────────────────────────────────────
  let editOpen    = $state(false);
  let editTarget  = $state<MetaV2 | null>(null);
  let editing     = $state(false);
  let eName       = $state("");
  let eAmountRaw  = $state("");
  let eTargetDate = $state("");
  let eError      = $state<string | null>(null);
  let eAmount     = $derived(parseInt(eAmountRaw.replace(/\D/g, ""), 10) || 0);

  let editTitle = $derived(
    editTarget?.tipo === "me_deben"      ? "Editar préstamo" :
    editTarget?.tipo === "debo"          ? "Editar deuda"    :
                                           "Editar ahorro"
  );
  let editNameLabel = $derived(
    editTarget?.tipo === "me_deben" ? "Nombre del deudor"  :
    editTarget?.tipo === "debo"     ? "Nombre de la deuda" :
                                      "Nombre del objetivo"
  );
  let editAmountLabel = $derived(
    editTarget?.tipo === "me_deben" ? "Monto prestado"  :
    editTarget?.tipo === "debo"     ? "Monto total"     :
                                      "Monto objetivo"
  );

  function openEdit(m: MetaV2) {
    detail      = null;
    eName       = m.nombre;
    eAmountRaw  = String(m.total);
    eTargetDate = m.fecha ?? "";
    eError      = null;
    editTarget  = m;
    editOpen    = true;
  }

  async function handleEdit(ev: Event) {
    ev.preventDefault();
    if (!editTarget) return;
    if (!eName.trim()) { eError = "El nombre no puede estar vacío."; return; }
    if (eAmount <= 0)  { eError = "El monto debe ser mayor que 0."; return; }
    editing = true; eError = null;
    const [prefix, rawId] = editTarget.id.split(":");
    try {
      if (prefix === "loan") {
        await loanApi.update(rawId, eName.trim(), eAmount);
      } else {
        await goalApi.update(rawId, {
          name: eName.trim(),
          target_cop: eAmount,
          target_date: editTarget.tipo === "quiero_juntar" ? (eTargetDate || null) : null,
          type: editTarget.tipo === "debo" ? "debt" : "saving",
          installments: editTarget.cuotas,
        });
      }
      await loadMetas();
      editOpen = false;
      editTarget = null;
    } catch (e) {
      console.error("[metas] edit:", e);
      eError = extractMsg(e);
    } finally {
      editing = false;
    }
  }

  // ── Eliminar ──────────────────────────────────────────────────────────────
  let deleteOpen   = $state(false);
  let deleteTarget = $state<MetaV2 | null>(null);
  let deleting     = $state(false);
  let deleteError  = $state<string | null>(null);

  function openDelete(m: MetaV2) {
    detail       = null;
    deleteTarget = m;
    deleteError  = null;
    deleteOpen   = true;
  }

  async function handleDelete() {
    if (!deleteTarget) return;
    deleting = true; deleteError = null;
    const [prefix, rawId] = deleteTarget.id.split(":");
    try {
      if (prefix === "loan") {
        await loanApi.remove(rawId);
      } else {
        await goalApi.remove(rawId);
      }
      await loadMetas();
      deleteOpen = false;
      deleteTarget = null;
    } catch (e) {
      console.error("[metas] delete:", e);
      deleteError = extractMsg(e);
    } finally {
      deleting = false;
    }
  }
</script>

<main>
  <div class="header">
    <div class="header-title">
      <h1>Metas</h1>
      {#if thisMonthPaid > 0}
        <span class="header-hint">{formatCOP(thisMonthPaid)} abonados este mes</span>
      {/if}
    </div>
    <span class="tour-field-block">
      {#if isActiveStep("metas")}<TourPoint text="Crea un ahorro, préstamo o deuda — no desde Registros" />{/if}
      <button class="btn-primary" onclick={() => { createOpen = true; loadExpenseCategories(); }}>+ Nueva</button>
    </span>
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
              {@render metaCard(m)}
            {/each}
          </div>
        {/if}

        {#if pendingLoans.length > 0}
          <div class="subsection-label subsection-me_deben">Préstamos</div>
          <div class="meta-grid">
            {#each pendingLoans as m (m.id)}
              {@render metaCard(m)}
            {/each}
          </div>
        {/if}

        {#if pendingSavings.length > 0}
          <div class="subsection-label subsection-quiero_juntar">Ahorros</div>
          <div class="meta-grid">
            {#each pendingSavings as m (m.id)}
              {@render metaCard(m)}
            {/each}
          </div>
        {/if}
      {/if}

      <!-- ── Sección LOGROS ── -->
      {#if showDone && doneMetas.length > 0}
        {#if showPending && allPendingCount > 0}
          <div class="section-divider"></div>
        {/if}
        <div class="section-label secondary">Logros</div>
        <div class="meta-grid">
          {#each doneMetas as m (m.id)}
            {@render metaCard(m)}
          {/each}
        </div>
      {/if}

    </ScrollArea>
  {/if}
</main>

{#snippet metaCard(m: MetaV2)}
  {@const done = m.estado === "completado"}
  {@const streak = done ? 0 : monthStreak(m)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="meta-card" class:done onclick={() => { detail = m; }}>
    <div class="card-top">
      <span class="meta-name">{m.nombre}</span>
      {#if done}
        <span class="done-check" title="Completado">✓</span>
      {/if}
    </div>

    {#if done}
      <div class="done-summary">
        <span class="done-total">{formatCOP(m.total)}</span>
        <span class="done-label">{tipoLabel(m.tipo)}{m.fecha ? ` · ${formatDateShort(m.fecha)}` : ""}</span>
      </div>
    {:else}
      <div class="pending-amount">
        <span class="pending-value" style="color: {tipoAccent(m.tipo)}">{formatCOP(m.pendiente)}</span>
        <span class="pending-label">{pendingLabel(m.tipo)}</span>
      </div>

      <div class="progress-wrap">
        <div class="progress-bar">
          <div class="progress-fill" style="width: {pct(m)}%; background: {tipoAccent(m.tipo)}"></div>
        </div>
        <span class="pct-text">{pct(m).toFixed(0)}%</span>
      </div>

      <div class="amounts">
        <span class="paid-label">Abonado</span>
        <span class="paid-value">{formatCOP(m.abonado)}</span>
        <span class="sep">/</span>
        <span class="total-value">{formatCOP(m.total)}</span>
        {#if streak >= 2}
          <span class="streak-badge">{streak} meses seguidos</span>
        {/if}
      </div>

      {#if m.cuotas !== null && m.cuotas > 0 && m.tipo === "debo"}
        <div class="cuotas-hint">≈ {formatCOP(Math.ceil(m.total / m.cuotas))}/mes · {m.cuotas} cuotas</div>
      {/if}

      <div class="card-footer">
        {#if m.tipo === "quiero_juntar" && m.fecha}
          {@const d = daysUntil(m.fecha)}
          <span class="meta-date" class:overdue={d < 0}>
            {d > 0 ? `Faltan ${d} día${d !== 1 ? "s" : ""}` : d === 0 ? "Es hoy" : `Vencía hace ${-d} día${-d !== 1 ? "s" : ""}`}
          </span>
        {:else}
          <span class="meta-date">{m.fecha ? formatDateShort(m.fecha) : "Sin fecha"}</span>
        {/if}
      </div>
    {/if}
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

        {#if cTipo}
          <div class="field">
            <label for="c-nombre">
              {cTipo === "me_deben" ? "A quién prestaste" : cTipo === "debo" ? "Nombre de la deuda" : "Nombre del objetivo"}
            </label>
            <input
              id="c-nombre"
              type="text"
              bind:value={cNombre}
              placeholder={cTipo === "me_deben" ? "Ej: Juan, María…" : cTipo === "debo" ? "Ej: Compra a crédito…" : "Ej: Viaje, Laptop…"}
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

          {#if cTipo === "debo"}
            <div class="field">
              <span class="field-label">Categoría</span>
              <CustomSelect
                bind:value={cCategoryId}
                options={expenseCategories.map(c => ({ value: c.id, label: c.name }))}
                placeholder="Selecciona categoría…"
              />
            </div>
          {/if}

          {#if cTipo === "me_deben"}
            <div class="field">
              <span class="field-label">Fecha del préstamo</span>
              <DatePicker bind:value={cDate} />
            </div>
          {:else if cTipo === "debo"}
            <div class="field">
              <span class="field-label">Fecha de la deuda</span>
              <DatePicker bind:value={cDate} />
            </div>
            <div class="field">
              <label for="c-cuotas">Cuotas <span class="optional">(opcional)</span></label>
              <input
                id="c-cuotas"
                type="text"
                inputmode="numeric"
                placeholder="Ej: 12"
                maxlength="3"
                bind:value={cCuotasRaw}
              />
              {#if cCuotas > 0 && cAmount > 0}
                <span class="hint">≈ {formatCOP(Math.ceil(cAmount / cCuotas))}/mes</span>
              {/if}
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
            disabled={creating || !cTipo || !cNombre.trim() || cAmount <= 0 || (cTipo === "me_deben" && !cDate) || (cTipo === "debo" && (!cDate || !cCategoryId))}
          >
            {creating ? "Creando…" : "Crear"}
          </button>
        </div>

      </form>
  </div>
  </div>
{/if}

{#if detail}
  {@const dm = detail}
  <PaymentModal
    title={dm.nombre}
    subtitle={tipoLabel(dm.tipo)}
    subtitleClass="tipo-{dm.tipo}"
    note={dm.nota}
    stats={buildStats(dm)}
    paid={dm.abonado}
    total={dm.total}
    progressDone={dm.estado === "completado"}
    items={toPaymentItems(dm)}
    itemsLabel="Abonos"
    canPay={dm.estado === "pendiente"}
    onAddPayment={handleAddPayment}
    onEdit={() => openEdit(dm)}
    onDelete={() => openDelete(dm)}
    onClose={() => { detail = null; }}
  />
{/if}

{#if editOpen && editTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { editOpen = false; editTarget = null; }}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <h2>{editTitle}</h2>

    {#if eError}
      <div class="banner error small"><pre>{eError}</pre></div>
    {/if}

    <form onsubmit={handleEdit} class="modal-form">
      <div class="field">
        <label for="e-nombre">{editNameLabel}</label>
        <input id="e-nombre" type="text" bind:value={eName} maxlength="100" />
      </div>
      <div class="field">
        <label for="e-amount">{editAmountLabel}</label>
        <input
          id="e-amount"
          type="text"
          inputmode="numeric"
          placeholder="0"
          value={eAmountRaw ? new Intl.NumberFormat("es-CO").format(eAmount) : ""}
          oninput={(e) => handleAmountInput(e, (v) => { eAmountRaw = v; })}
        />
      </div>
      {#if editTarget?.tipo === "quiero_juntar"}
        <div class="field">
          <span class="field-label">Fecha meta <span class="optional">(opcional)</span></span>
          <DatePicker bind:value={eTargetDate} />
        </div>
      {/if}
      <div class="modal-actions">
        <button
          type="button"
          class="btn-secondary"
          onclick={() => { editOpen = false; editTarget = null; }}
        >Cancelar</button>
        <button
          type="submit"
          class="btn-primary"
          disabled={editing || !eName.trim() || eAmount <= 0}
        >{editing ? "Guardando…" : "Guardar"}</button>
      </div>
    </form>
  </div>
  </div>
{/if}

{#if deleteOpen && deleteTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { deleteOpen = false; deleteTarget = null; }}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal modal-sm" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <h2>Eliminar meta</h2>
    <p class="delete-msg">¿Eliminar <strong>{deleteTarget.nombre}</strong>? Esta acción no se puede deshacer.</p>
    {#if deleteError}
      <div class="banner error small"><pre>{deleteError}</pre></div>
    {/if}
    <div class="modal-actions">
      <button
        class="btn-secondary"
        onclick={() => { deleteOpen = false; deleteTarget = null; }}
      >Cancelar</button>
      <button class="btn-danger" onclick={handleDelete} disabled={deleting}>
        {deleting ? "Eliminando…" : "Eliminar"}
      </button>
    </div>
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
    gap: 1rem;
  }

  .header-title { display: flex; align-items: baseline; gap: 0.65rem; min-width: 0; }

  .tour-field-block { position: relative; display: block; }

  h1 { font-size: 1.1rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.02em; }

  .header-hint {
    font-size: 0.78rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

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
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid transparent;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .filter-btn.secondary {
    font-size: 0.66rem;
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

  /* Logros — misma tarjeta a plena opacidad (nada de gris "olvidado"), solo
     un check discreto y un resumen en vez de la barra de progreso ya vacía
     de sentido al 100%. */
  .meta-card.done { border-color: color-mix(in srgb, var(--success) 35%, var(--border)); }
  .meta-card.done:hover { border-color: var(--success); }

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

  /* ── Check de Logro — reemplaza el tipo-badge en tarjetas completadas ── */
  .done-check {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--success);
    background: color-mix(in srgb, var(--success) 16%, var(--bg-elevated));
    border-radius: var(--radius);
  }

  .done-summary { display: flex; flex-direction: column; gap: 0.15rem; }
  .done-total { font-size: 1.1rem; font-weight: 700; font-variant-numeric: tabular-nums; font-family: var(--font-mono); color: var(--text-primary); }
  .done-label { font-size: 0.75rem; color: var(--text-muted); }

  /* ── Racha de constancia — solo aparece con 2+ meses seguidos abonando;
     dato real, no decoración, por eso vive junto al monto abonado. ── */
  .streak-badge {
    font-size: 0.62rem;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: var(--radius);
    padding: 0.05rem 0.4rem;
    margin-left: auto;
    white-space: nowrap;
  }

  /* ── Pending amount ── */
  .pending-amount { display: flex; align-items: baseline; gap: 0.4rem; }
  .pending-value  { font-size: 1.1rem; font-weight: 700; font-variant-numeric: tabular-nums; font-family: var(--font-mono); }
  .pending-label  { font-size: 0.72rem; color: var(--text-muted); }

  /* ── Progress ── */
  .progress-wrap { display: flex; align-items: center; gap: 0.5rem; }
  .progress-bar  { flex: 1; height: 4px; background: var(--bg-elevated); overflow: hidden; }
  .progress-fill { height: 100%; transition: width 0.2s ease; }
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
    font-variant-numeric: tabular-nums; font-family: var(--font-mono);
  }

  /* ── Card footer ── */
  .card-footer { display: flex; align-items: center; justify-content: space-between; margin-top: auto; }
  .meta-date   { font-size: 0.72rem; color: var(--text-muted); }
  .meta-date.overdue { color: var(--text-secondary); }

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
    padding: 0.45rem 1rem; background: var(--accent); color: var(--bg-base);
    font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em;
    font-size: 0.78rem; font-weight: 700; border-radius: var(--radius);
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-secondary {
    padding: 0.45rem 1rem; background: transparent; color: var(--text-secondary);
    font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em;
    font-size: 0.78rem; font-weight: 600; border-radius: var(--radius);
    border: 1px solid var(--border); transition: border-color 0.15s, color 0.15s;
  }
  .btn-secondary:hover { border-color: var(--text-secondary); color: var(--text-primary); }

  /* ── Overlay / Modal ── */
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 20;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 1.5rem;
    width: min(440px, 92vw); max-height: 85vh; overflow-y: auto;
    display: flex; flex-direction: column;
  }
  .modal-sm { width: min(340px, 92vw); max-height: unset; }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); margin-bottom: 0.75rem; }

  /* ── Formulario ── */
  .modal-form    { display: flex; flex-direction: column; gap: 0.9rem; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

  .field { display: flex; flex-direction: column; gap: 0.3rem; }
  label, .field-label { font-size: 0.78rem; font-weight: 500; color: var(--text-secondary); }
  .optional { font-weight: 400; color: var(--text-muted); }

  input[type="text"] {
    -webkit-appearance: none; appearance: none;
    background-color: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius); color: var(--text-primary); font: inherit;
    font-size: 0.9rem; padding: 0.5rem 0.75rem; outline: none;
    transition: border-color 0.15s; width: 100%;
  }
  input:focus { border-color: var(--accent); }

  input[inputmode="numeric"] { font-family: var(--font-mono); }

  .hint {
    font-size: 0.72rem; color: var(--text-muted);
    line-height: 1.4; margin-top: 0.1rem;
  }

  /* ── Misc ── */
  .muted { color: var(--text-muted); font-size: 0.85rem; }
  .empty {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 0.5rem; padding: 2rem;
    color: var(--text-secondary); font-size: 0.9rem;
  }

  /* ── Progreso de ahorros ── */
  .atrasado-badge { font-size: 0.65rem; font-weight: 600; color: var(--accent); }
  .muted-hint     { font-size: 0.7rem; color: var(--text-muted); font-variant-numeric: tabular-nums; font-family: var(--font-mono); }

  /* ── Modal de confirmación ── */
  .delete-msg { font-size: 0.875rem; color: var(--text-secondary); margin-bottom: 0.75rem; line-height: 1.5; }
  .delete-msg strong { color: var(--text-primary); }
  .btn-danger {
    padding: 0.45rem 1rem; background: var(--danger); color: var(--bg-base);
    font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em;
    font-size: 0.78rem; font-weight: 700; border-radius: var(--radius);
    transition: opacity 0.15s;
  }
  .btn-danger:hover:not(:disabled) { opacity: 0.85; }
  .btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }
  .banner.small { font-size: 0.78rem; }
</style>
