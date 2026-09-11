<script lang="ts">
  import { entryApi, categoryApi, accountApi, goalApi, gasApi, vehicleApi, routeApi, fillupApi } from "$lib/api";
  import type {
    Category, GoalWithProgressV2, Entry, RoutesCost, AccountBalances,
    PeriodSummaryV2, CategoryProgressV2, RouteV2, VehicleV2, Account, FillupWithExpenseResult,
  } from "$lib/types";
  import DatePicker from "$lib/components/DatePicker.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";
  import { bumpTxVersion } from "$lib/txState.svelte";
  import { kmToM } from "$lib/constants";

  // ── Estado del formulario ──────────────────────────────────────────────────
  let kind = $state<"ingreso" | "gasto" | "tanqueo">(
    (localStorage.getItem("registrar_kind") as "ingreso" | "gasto" | "tanqueo") ?? "gasto"
  );
  let categoryId    = $state("");
  let amountRaw     = $state("");
  let date          = $state(todayISO());
  let note          = $state("");
  let extraordinary = $state(false);

  // ── Viaje (para el tanque, sin costo estimado — sección 7 de schema-v2.md) ──
  let gasKmRaw   = $state("");
  let gasKm      = $derived(parseFloat(gasKmRaw) || 0);
  let savedGasKm = $state(0);
  let vehicles   = $state<VehicleV2[]>([]);
  let vehicleId  = $state<string | null>(null);
  let viajeOpen  = $state(false);

  function toggleViaje() {
    viajeOpen = !viajeOpen;
    if (!viajeOpen) gasKmRaw = "";
  }

  // ── Hint flotante de "Extraordinario" — position:fixed con coordenadas
  // calculadas en JS, igual que el menú de CustomSelect: así escapa de
  // cualquier ancestro con overflow:hidden/scroll y nunca queda tapado. ──
  let extraHintOpen  = $state(false);
  let extraHintStyle = $state("");
  let extraHintEl    = $state<HTMLElement | undefined>(undefined);

  function showExtraHint() {
    if (extraHintEl) {
      const rect = extraHintEl.getBoundingClientRect();
      extraHintStyle = `left:${rect.left + rect.width / 2}px; bottom:${window.innerHeight - rect.top + 8}px;`;
    }
    extraHintOpen = true;
  }
  function hideExtraHint() { extraHintOpen = false; }

  // ── Tanqueo ───────────────────────────────────────────────────────────────
  let fillupVehicleId = $state<string | null>(null);
  let fillupAmountRaw = $state("");
  let fillupPriceRaw  = $state("");
  let fillupDate      = $state(todayISO());
  let fillupNote      = $state("");
  let fillupSaved     = $state<FillupWithExpenseResult | null>(null);
  let fillupAmount    = $derived(parseInt(fillupAmountRaw.replace(/\D/g, ""), 10) || 0);
  let fillupPrice     = $derived(parseInt(fillupPriceRaw.replace(/\D/g, ""), 10) || 0);

  // ── Datos cargados ─────────────────────────────────────────────────────────
  let allCategories = $state<Category[]>([]);
  let categories    = $state<Category[]>([]);
  let accounts      = $state<Account[]>([]);
  let routeCosts    = $state<RoutesCost | null>(null);
  let customRoutes  = $state<RouteV2[]>([]);
  let loadError     = $state<string | null>(null);

  function accountId(code: string): string | null {
    return accounts.find(a => a.code === code)?.id ?? null;
  }

  async function findOrCreateCategory(name: string, kind: "income" | "expense"): Promise<string> {
    const existing = allCategories.find(c => c.name === name && c.kind === kind);
    if (existing) return existing.id;
    const created = await categoryApi.create({ name, kind, is_fixed: false, route_id: null });
    allCategories = [...allCategories, created];
    return created.id;
  }

  let _catsLoaded  = false;
  let _catsLoading = false;

  async function loadCategories() {
    if (_catsLoaded || _catsLoading) return;
    _catsLoading = true;
    try {
      allCategories = await categoryApi.list();
      _catsLoaded = true;
    } catch (e) {
      console.error("[registrar] load categories error:", e);
      loadError = "Error cargando categorías. Recarga la app.";
    } finally {
      _catsLoading = false;
    }
  }

  // ── Feedback ───────────────────────────────────────────────────────────────
  let saving       = $state(false);
  let saved        = $state<Entry | null>(null);
  let saveError    = $state<string | null>(null);

  let amount = $derived(parseInt(amountRaw.replace(/\D/g, ""), 10) || 0);

  let mappedKind = $derived<"income" | "expense">(kind === "ingreso" ? "income" : "expense");

  let displayCategories = $derived(
    allCategories.filter(c => c.kind === mappedKind && (mappedKind === "income" || c.name !== "Gasolina"))
  );

  // ── Panel contextual (derecha) ─────────────────────────────────────────────
  let statsRevision  = $state(0);
  let statsLoading   = $state(true);
  let lastTx         = $state<Entry | null>(null);
  let monthSummary   = $state<PeriodSummaryV2 | null>(null);
  let balanceData    = $state<AccountBalances | null>(null);
  let activeGoals    = $state<GoalWithProgressV2[]>([]);
  let categoryMap    = $derived(new Map(allCategories.map(c => [c.id, c.name])));

  function categoryName(e: Entry): string {
    if (e.type === "transfer") return "Transferencia";
    return (e.category_id && categoryMap.get(e.category_id)) ?? "Sin categoría";
  }

  let catLoading   = $state(false);
  let catBudget    = $state<CategoryProgressV2 | null>(null);
  let catRecentTxs = $state<Entry[]>([]);
  let catAvg3m     = $state<number | null>(null);

  let showCatPanel = $derived(
    !catLoading && (catBudget !== null || catRecentTxs.length > 0)
  );
  let nextGoal = $derived(activeGoals.find(g => g.pending > 0) ?? activeGoals[0] ?? null);

  // ── Utilidades ─────────────────────────────────────────────────────────────
  function todayISO(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function formatDate(iso: string): string {
    const [, m, d] = iso.split("-");
    return `${d}/${m}`;
  }

  function handleAmountInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    if (!digits) { amountRaw = ""; e.currentTarget.value = ""; return; }
    const num = parseInt(digits, 10);
    amountRaw = digits;
    e.currentTarget.value = new Intl.NumberFormat("es-CO").format(num);
  }

  function handleFillupAmountInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    if (!digits) { fillupAmountRaw = ""; e.currentTarget.value = ""; return; }
    const num = parseInt(digits, 10);
    fillupAmountRaw = digits;
    e.currentTarget.value = new Intl.NumberFormat("es-CO").format(num);
  }

  function handleFillupPriceInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    if (!digits) { fillupPriceRaw = ""; e.currentTarget.value = ""; return; }
    const num = parseInt(digits, 10);
    fillupPriceRaw = digits;
    e.currentTarget.value = new Intl.NumberFormat("es-CO").format(num);
  }

  // ── Effects ────────────────────────────────────────────────────────────────

  // Persist kind
  $effect(() => { localStorage.setItem("registrar_kind", kind); });

  // Aplicar categorías del cache cuando cambia el tipo
  $effect(() => {
    const k = mappedKind;
    let cancelled = false;
    async function apply() {
      await loadCategories();
      if (cancelled) return;
      if (kind === "tanqueo") return; // tanqueo usa categoría fija "Gasolina", pero necesita allCategories cargado
      const filtered = allCategories.filter(c => c.kind === k && (k === "income" || c.name !== "Gasolina"));
      categories = filtered;
      if (!filtered.some(c => c.id === categoryId)) categoryId = filtered[0]?.id ?? "";
    }
    apply();
    return () => { cancelled = true; };
  });

  // Pre-fill km del viaje cuando la categoría tiene ruta asociada
  $effect(() => {
    const cat = allCategories.find(c => c.id === categoryId);
    if (cat?.route_id) {
      const route = customRoutes.find(r => r.id === cat.route_id);
      if (route) { gasKmRaw = (route.distance_m / 1000).toString(); viajeOpen = true; }
    } else {
      gasKmRaw = "";
    }
  });

  // Cargar costos de ruta, rutas, cuentas y vehículos una vez
  $effect(() => {
    gasApi.getRouteCosts().then(r => { routeCosts = r; }).catch(() => {});
    routeApi.list().then(r => { customRoutes = r; }).catch(() => {});
    accountApi.list().then(a => { accounts = a; }).catch(() => {});
    gasApi.getCurrent().then(p => { if (p && !fillupPriceRaw) fillupPriceRaw = String(p.price_per_gallon); }).catch(() => {});
    vehicleApi.list().then(vs => {
      vehicles = vs;
      if (vehicleId === null && vs.length > 0) vehicleId = vs[0].id;
      if (fillupVehicleId === null && vs.length > 0) fillupVehicleId = vs[0].id;
    }).catch(() => {});
  });

  // Cargar estadísticas generales del panel derecho
  $effect(() => {
    const _ = statsRevision;
    let cancelled = false;
    statsLoading = true;
    const now = new Date();
    Promise.all([
      entryApi.list({ page: 1, page_size: 1 }),
      entryApi.getPeriodSummary({ type: "Month", value: { year: now.getFullYear(), month: now.getMonth() + 1 } }),
      entryApi.getAccountBalances(),
      goalApi.list("saving"),
    ]).then(([recent, summary, bal, gls]) => {
      if (cancelled) return;
      lastTx       = recent.entries[0] ?? null;
      monthSummary = summary;
      balanceData  = bal;
      activeGoals  = gls;
      statsLoading = false;
    }).catch(e => {
      if (!cancelled) { console.error("[registrar] stats:", e); statsLoading = false; }
    });
    return () => { cancelled = true; };
  });

  // Cargar estadísticas de la categoría seleccionada
  $effect(() => {
    const cat = categoryId;
    catBudget = null; catRecentTxs = []; catAvg3m = null;
    if (!cat) { catLoading = false; return; }

    let cancelled = false;
    catLoading = true;

    const today = new Date();
    const y = today.getFullYear();
    const m = today.getMonth();
    const d3m = new Date(y, m - 2, 1);
    const s3 = `${d3m.getFullYear()}-${String(d3m.getMonth() + 1).padStart(2, "0")}-01`;
    const eToday = `${y}-${String(m + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;

    Promise.all([
      entryApi.getCategoryProgress({ type: "Month", value: { year: y, month: m + 1 } }),
      entryApi.list({ category_id: cat, page: 1, page_size: 5 }),
      entryApi.list({ category_id: cat, start: s3, end: eToday, page_size: 500 }),
    ]).then(([progress, recent, hist]) => {
      if (cancelled) return;
      catBudget    = progress.find(p => p.category_id === cat) ?? null;
      catRecentTxs = recent.entries;
      if (hist.entries.length > 0) {
        catAvg3m = Math.round(hist.entries.reduce((s, t) => s + t.amount_cop, 0) / 3);
      }
      catLoading = false;
    }).catch(e => {
      if (!cancelled) { console.error("[registrar] cat stats:", e); catLoading = false; }
    });
    return () => { cancelled = true; };
  });

  // ── Formulario ─────────────────────────────────────────────────────────────

  function selectGasPreset(km: number) {
    const str = km.toString();
    gasKmRaw = gasKmRaw === str ? "" : str;
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();

    if (kind === "tanqueo") {
      await doFillupSave();
      return;
    }

    if (amount <= 0)   { saveError = "El monto debe ser mayor que 0."; return; }
    if (!categoryId)   { saveError = "Selecciona una categoría."; return; }

    if (kind === "gasto") {
      try {
        const bal = await entryApi.getAccountBalances();
        if (bal.disponible < amount) {
          saveError = "Saldo no disponible. Si vas a financiar este gasto, regístralo como deuda desde Metas.";
          return;
        }
      } catch {
        // Si falla la consulta de saldo, se procede sin verificar
      }
    }

    await doSave();
  }

  async function doSave() {
    saving    = true;
    saveError = null;
    saved     = null;

    try {
      const cash = accountId("cash");
      if (!cash) throw new Error("No se encontró la cuenta 'cash'.");
      const tx: Entry = await entryApi.create({
        occurred_on: date, type: mappedKind, amount_cop: amount,
        account_from: mappedKind === "expense" ? cash : null,
        account_to:   mappedKind === "income"  ? cash : null,
        category_id: categoryId, goal_id: null, loan_id: null,
        note: note.trim() || null, is_extraordinary: extraordinary,
      });

      // Viaje asociado (sin costo — solo consumo de tanque, sección 7)
      if (viajeOpen && gasKm > 0 && vehicleId !== null) {
        try {
          await fillupApi.registerTrip({ vehicle_id: vehicleId, occurred_on: date, distance_m: kmToM(gasKm) });
        } catch (e) {
          console.error("[registrar] trip register error:", e);
        }
      }

      bumpTxVersion();
      savedGasKm   = gasKm;
      saved        = tx;
      lastTx       = tx;
      statsRevision++;

      amountRaw       = "";
      note            = "";
      extraordinary   = false;
      date            = todayISO();
      const cat = allCategories.find(c => c.id === categoryId);
      if (cat?.route_id) {
        const route = customRoutes.find(r => r.id === cat.route_id);
        gasKmRaw = route ? (route.distance_m / 1000).toString() : "";
      } else {
        gasKmRaw = "";
      }
      setTimeout(() => { saved = null; savedGasKm = 0; }, 6000);
    } catch (e: any) {
      console.error("[registrar] save error:", e);
      saveError = e?.kind === "ValidationError" ? e.message : "No se pudo guardar. Intenta de nuevo.";
    } finally {
      saving = false;
    }
  }

  async function doFillupSave() {
    if (fillupAmount <= 0) { saveError = "El monto debe ser mayor que 0."; return; }
    if (fillupPrice  <= 0) { saveError = "El precio del galón debe ser mayor que 0."; return; }
    if (fillupVehicleId === null) { saveError = "Selecciona un vehículo."; return; }

    saving    = true;
    saveError = null;
    fillupSaved = null;

    try {
      const gasolinaCat = await findOrCreateCategory("Gasolina", "expense");
      const result = await fillupApi.createWithExpense({
        vehicle_id: fillupVehicleId,
        occurred_on: fillupDate,
        total_cop: fillupAmount,
        price_cop_per_gallon: fillupPrice,
        category_id: gasolinaCat,
        note: fillupNote.trim() || null,
      });

      fillupSaved     = result;
      fillupAmountRaw = "";
      fillupNote      = "";
      fillupDate      = todayISO();
      bumpTxVersion();
      statsRevision++;
      setTimeout(() => { fillupSaved = null; }, 6000);
    } catch (e: any) {
      console.error("[registrar] fillup save error:", e);
      saveError = e?.message ?? "No se pudo registrar el tanqueo. Intenta de nuevo.";
    } finally {
      saving = false;
    }
  }

  let kindLabel = $derived(kind === "ingreso" ? "ingreso" : kind === "gasto" ? "gasto" : "tanqueo");
</script>

<div class="registrar-shell">

  <!-- ═══════════════════════ ENTRADA RÁPIDA ═══════════════════════ -->
  <div class="entry-col">
    <ScrollArea class="entry-scroll" scrollbar="thin">
    <div class="entry-wrap">

      {#if loadError}
        <div class="banner error"><strong>Error cargando datos</strong><pre>{loadError}</pre></div>
      {/if}

      {#if fillupSaved}
        <div class="banner success">
          <div class="banner-body">
            ✓ Tanqueo guardado · {formatCOP(fillupSaved.fillup.total_cop)}
            <span class="auto-gas-note">{(fillupSaved.fillup.volume_ml / 3785.411784).toFixed(2)} gal cargados</span>
            {#if fillupSaved.warning}
              <span class="auto-gas-note fillup-warn">⚠ {fillupSaved.warning}</span>
            {/if}
          </div>
          <button class="banner-close" onclick={() => { fillupSaved = null; }}>×</button>
        </div>
      {/if}

      {#if saved}
        <div class="banner success">
          <div class="banner-body">
            ✓ {saved.type === "income" ? "Ingreso" : "Gasto"} de {formatCOP(saved.amount_cop)} guardado
            {#if savedGasKm > 0}
              <span class="auto-gas-note">+ viaje de {savedGasKm} km registrado</span>
            {/if}
          </div>
          <button class="banner-close" onclick={() => { saved = null; savedGasKm = 0; }}>×</button>
        </div>
      {/if}

      {#if saveError}
        <div class="banner error"><strong>Error al guardar</strong><pre>{saveError}</pre></div>
      {/if}

      <form id="tx-form" onsubmit={handleSubmit} class="entry-form">

        <!-- Pestañas de tipo -->
        <div class="kind-tabs">
          <button
            type="button"
            class="kind-tab tab-income"
            class:active={kind === "ingreso"}
            onclick={() => { kind = "ingreso"; saveError = null; }}
          >Ingreso</button>
          <button
            type="button"
            class="kind-tab tab-expense"
            class:active={kind === "gasto"}
            onclick={() => { kind = "gasto"; saveError = null; }}
          >Gasto</button>
          <button
            type="button"
            class="kind-tab tab-fuel"
            class:active={kind === "tanqueo"}
            onclick={() => { kind = "tanqueo"; saveError = null; }}
          >Tanqueo</button>
        </div>

        {#if kind === "tanqueo"}

          <!-- ── Tanqueo ── -->
          <div class="entry-card">
          <div class="amount-display tone-fuel">
            <span class="amount-currency">$</span>
            <input
              class="amount-input"
              type="text"
              inputmode="numeric"
              placeholder="0"
              value={fillupAmountRaw ? new Intl.NumberFormat("es-CO").format(fillupAmount) : ""}
              oninput={handleFillupAmountInput}
            />
          </div>
          <span class="amount-caption">monto pagado</span>

          <div class="chip-section">
            <span class="chip-section-label">Vehículo</span>
            {#if vehicles.length === 0}
              <p class="empty-hint">Configura un vehículo en <a href="/config">Configuración</a> para registrar tanqueos.</p>
            {:else}
              <div class="chip-grid">
                {#each vehicles as v (v.id)}
                  <button
                    type="button"
                    class="chip"
                    class:active={fillupVehicleId === v.id}
                    onclick={() => { fillupVehicleId = v.id; }}
                  >{v.name}</button>
                {/each}
              </div>
            {/if}
          </div>

          <div class="meta-row">
            <div class="meta-field">
              <span class="meta-label">Precio del galón <span class="optional">editable</span></span>
              <input
                class="meta-input"
                type="text"
                inputmode="numeric"
                placeholder="0"
                value={fillupPriceRaw ? new Intl.NumberFormat("es-CO").format(fillupPrice) : ""}
                oninput={handleFillupPriceInput}
              />
            </div>
            <div class="meta-field">
              <span class="meta-label">Fecha</span>
              <DatePicker bind:value={fillupDate} />
            </div>
          </div>

          <div class="meta-field">
            <span class="meta-label">Nota <span class="optional">opcional</span></span>
            <input class="meta-input" type="text" bind:value={fillupNote} placeholder="Descripción breve…" maxlength="200" />
          </div>

          <div class="cat-fixed-note">Se registra en categoría <strong>Gasolina</strong></div>
          </div>

        {:else}

          <!-- ── Ingreso / Gasto ── -->
          <div class="entry-card">
          <div class="amount-display" class:tone-income={kind === "ingreso"} class:tone-expense={kind === "gasto"}>
            <span class="amount-currency">$</span>
            <input
              class="amount-input"
              type="text"
              inputmode="numeric"
              placeholder="0"
              value={amountRaw ? new Intl.NumberFormat("es-CO").format(amount) : ""}
              oninput={handleAmountInput}
            />
          </div>
          <span class="amount-caption">monto del {kindLabel}</span>

          <div class="chip-section">
            <span class="chip-section-label">Categoría</span>
            {#if displayCategories.length === 0}
              <p class="empty-hint">Sin categorías todavía.</p>
            {:else}
              <div class="chip-grid">
                {#each displayCategories as c (c.id)}
                  <button
                    type="button"
                    class="chip"
                    class:active={categoryId === c.id}
                    onclick={() => { categoryId = c.id; }}
                  >{c.name}</button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Kilometraje del viaje (para el tanque, sin costo estimado) -->
          {#if routeCosts}
            <div class="viaje-section">
              <button type="button" class="viaje-toggle" onclick={toggleViaje}>
                <span class="chip-section-label">Kilometraje <span class="optional">opcional</span></span>
                <span class="switch" class:on={viajeOpen}></span>
              </button>

              {#if viajeOpen}
                <div class="viaje-body">
                  {#if vehicles.length === 0}
                    <p class="empty-hint">Configura un vehículo en <a href="/config">Configuración</a> para registrar viajes.</p>
                  {:else}
                    {#if customRoutes.length > 0}
                      <div class="viaje-field">
                        <span class="viaje-field-label">Destino</span>
                        <div class="chip-grid chip-grid-sm">
                          {#each customRoutes as route (route.id)}
                            <button
                              type="button"
                              class="chip chip-sm"
                              class:active={gasKmRaw === (route.distance_m / 1000).toString()}
                              onclick={() => selectGasPreset(route.distance_m / 1000)}
                              title={route.description ?? route.name}
                            >{route.name}</button>
                          {/each}
                        </div>
                      </div>
                    {/if}

                    <div class="viaje-field">
                      <span class="viaje-field-label">Distancia</span>
                      <div class="km-field">
                        <input type="text" inputmode="decimal" class="km-value" bind:value={gasKmRaw} placeholder="0" />
                        <span class="km-unit">km</span>
                      </div>
                    </div>

                    {#if gasKm > 0}
                      <div class="viaje-field">
                        <span class="viaje-field-label">Vehículo</span>
                        <div class="chip-grid chip-grid-sm">
                          {#each vehicles as v (v.id)}
                            <button
                              type="button"
                              class="chip chip-sm"
                              class:active={vehicleId === v.id}
                              onclick={() => { vehicleId = v.id; }}
                            >{v.name}</button>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <div class="meta-row">
            <div class="meta-field">
              <span class="meta-label">Fecha</span>
              <DatePicker bind:value={date} />
            </div>
            <div class="meta-field meta-field-check">
              <span class="meta-label">&nbsp;</span>
              <label class="check-row">
                <input type="checkbox" class="check-box" bind:checked={extraordinary} />
                <span>Extraordinario</span>
                <span
                  class="hint-mark"
                  role="tooltip"
                  bind:this={extraHintEl}
                  onmouseenter={showExtraHint}
                  onmouseleave={hideExtraHint}
                >?</span>
              </label>
            </div>
          </div>

          <div class="meta-field">
            <span class="meta-label">Nota <span class="optional">opcional</span></span>
            <input class="meta-input" type="text" bind:value={note} placeholder="Descripción breve…" maxlength="200" />
          </div>

          <p class="metas-hint">
            ¿Vas a apartar plata para un ahorro, prestar dinero o registrar una deuda? Eso se hace desde <a href="/metas">Metas</a>, no aquí.
          </p>
          </div>

        {/if}

      </form>
    </div>
    </ScrollArea>

    <div class="save-bar-wrap">
      <button
        type="submit"
        form="tx-form"
        class="save-bar"
        class:save-income={kind === "ingreso"}
        class:save-expense={kind === "gasto"}
        class:save-fuel={kind === "tanqueo"}
        disabled={saving || (kind === "tanqueo" ? fillupAmount <= 0 || fillupVehicleId === null : amount <= 0)}
      >
        {saving ? "Guardando…" : `Guardar ${kindLabel}`}
      </button>
    </div>
  </div>

  <!-- ═══════════════════════ CONTEXTO ═══════════════════════ -->
  <div class="context-col">
    <ScrollArea class="ctx-scroll" scrollbar="thin">
    <div class="ctx-panel">
    {#if catLoading && !catBudget && catRecentTxs.length === 0}
      <div class="ctx-loading">
        <span class="ctx-loading-dot"></span>
        <span class="ctx-loading-label">Cargando {categories.find(c => c.id === categoryId)?.name ?? ""}…</span>
      </div>
    {:else if showCatPanel}

      <div class="ctx-panel-title">{categories.find(c => c.id === categoryId)?.name ?? ""}</div>

      {#if catBudget}
        <div class="ctx-row ctx-row-col">
          <span class="ctx-row-label">Presupuesto mensual</span>
          <div class="budget-bar-track">
            <div
              class="budget-bar-fill"
              class:over={catBudget.is_over}
              style="width: {Math.min(catBudget.percentage, 100)}%"
            ></div>
          </div>
          <div class="budget-nums">
            <span class="budget-current" class:over={catBudget.is_over}>
              {formatCOP(catBudget.current_amount)}
            </span>
            <span class="budget-sep">/</span>
            <span class="budget-target">{formatCOP(catBudget.monthly_target)}</span>
            <span class="budget-pct" class:over={catBudget.is_over}>
              {catBudget.percentage.toFixed(0)}%
            </span>
          </div>
        </div>
        <div class="ctx-div"></div>
      {/if}

      {#if catAvg3m !== null}
        <div class="ctx-row">
          <span class="ctx-row-label">Promedio mensual (3 meses)</span>
          <span class="ctx-row-value">{formatCOP(catAvg3m)}</span>
        </div>
        <div class="ctx-div"></div>
      {/if}

      {#if catRecentTxs.length > 0}
        <div class="ctx-row ctx-row-col">
          <span class="ctx-row-label">Últimas transacciones</span>
          <ul class="ctx-tx-list">
            {#each catRecentTxs as tx}
              <li class="ctx-tx-item">
                <span class="ctx-tx-date">{formatDate(tx.occurred_on)}</span>
                <span class="ctx-tx-note">{tx.note ?? "—"}</span>
                <span class="ctx-tx-amount" class:income={tx.type === "income"}>
                  {tx.type === "income" ? "+" : "−"}{formatCOP(tx.amount_cop)}
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

    {:else}

      {#if statsLoading && !monthSummary}
        <div class="ctx-loading">
          <span class="ctx-loading-dot"></span>
          <span class="ctx-loading-label">Cargando…</span>
        </div>
      {:else}

        {#if lastTx}
          <div class="ctx-row ctx-row-col">
            <span class="ctx-row-label">Último registro</span>
            <div class="ctx-last-row">
              <span class="ctx-last-cat">{categoryName(lastTx)}</span>
              <span class="ctx-last-amount" class:income={lastTx.type === "income"}>
                {lastTx.type === "income" ? "+" : lastTx.type === "expense" ? "−" : "→"}{formatCOP(lastTx.amount_cop)}
              </span>
            </div>
            <div class="ctx-last-meta">
              {formatDate(lastTx.occurred_on)}{lastTx.note ? ` · ${lastTx.note}` : ""}
            </div>
          </div>
          <div class="ctx-div"></div>
        {/if}

        {#if monthSummary}
          <div class="ctx-row ctx-row-col">
            <span class="ctx-row-label">Este mes</span>
            <div class="ctx-two-col">
              <div class="ctx-stat">
                <span class="ctx-stat-label">Ingresos</span>
                <span class="ctx-stat-val income">+{formatCOP(monthSummary.total_income)}</span>
              </div>
              <div class="ctx-stat">
                <span class="ctx-stat-label">Gastos</span>
                <span class="ctx-stat-val expense">−{formatCOP(monthSummary.total_expense)}</span>
              </div>
            </div>
          </div>
          <div class="ctx-div"></div>
        {/if}

        {#if balanceData}
          <div class="ctx-row">
            <span class="ctx-row-label">Disponible</span>
            <span
              class="ctx-row-value"
              class:income={balanceData.disponible >= 0}
              class:expense={balanceData.disponible < 0}
            >
              {formatCOP(balanceData.disponible)}
            </span>
          </div>
          <div class="ctx-div"></div>
        {/if}

        {#if nextGoal}
          <div class="ctx-row ctx-row-col">
            <span class="ctx-row-label">Próximo objetivo</span>
            <div class="ctx-goal-name">{nextGoal.goal.name}</div>
            <div class="ctx-progress-bar">
              <div
                class="ctx-progress-fill"
                style="width: {Math.min(nextGoal.percentage, 100)}%"
              ></div>
            </div>
            <div class="ctx-goal-row">
              <span class="ctx-goal-curr">{formatCOP(nextGoal.current_amount)}</span>
              <span class="ctx-goal-pct">{nextGoal.percentage.toFixed(0)}%</span>
              <span class="ctx-goal-target">{formatCOP(nextGoal.goal.target_cop)}</span>
            </div>
          </div>
        {/if}

      {/if}
    {/if}
    </div>
    </ScrollArea>
  </div>

</div>

{#if extraHintOpen}
  <div class="hint-popup" style={extraHintStyle}>
    Evento único o no recurrente — no forma parte del presupuesto mensual habitual (ej. un regalo, una emergencia)
  </div>
{/if}

<style>
  /* ── Layout shell ── */
  .registrar-shell {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
  }

  /* ── Entrada: columna principal ── */
  .entry-col {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
  }

  :global(.entry-scroll) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .entry-wrap {
    max-width: 880px;
    width: 100%;
    padding: 1.25rem 1.5rem 1rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .entry-form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* ── Pestañas de tipo (controlan qué formulario se ve, viven fuera de la tarjeta) ── */
  .kind-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    gap: 1.75rem;
  }

  .kind-tab {
    padding: 0.5rem 0.1rem 0.65rem;
    font-size: 0.78rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }
  .kind-tab:hover { color: var(--text-secondary); }
  .kind-tab.tab-income.active  { color: var(--success); border-color: var(--success); }
  .kind-tab.tab-expense.active { color: var(--danger);  border-color: var(--danger); }
  .kind-tab.tab-fuel.active    { color: var(--accent);  border-color: var(--accent); }

  /* ── Tarjeta: todo el formulario del tipo activo vive dentro, con el mismo
     chrome (borde + fondo) que los paneles del resto de la app ── */
  .entry-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  /* ── Monto — es el dato principal de toda la pantalla, tiene que notarse
     de inmediato antes que cualquier otro campo. ── */
  .amount-display {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding-bottom: 0.85rem;
    border-bottom: 2px solid var(--border);
  }

  /* Los <input> no exponen su línea base de texto de forma confiable en
     flexbox (varía por navegador/motor) — en vez de pelear con "baseline"
     o "flex-end", se centran verticalmente por la altura de caja completa,
     que es predecible en cualquier motor y se ve bien con números sin
     descendentes. */
  .amount-currency {
    font-size: 1.4rem;
    line-height: 1;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .amount-input {
    -webkit-appearance: none;
    appearance: none;
    background: transparent;
    border: none;
    outline: none;
    width: 100%;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 2.75rem;
    line-height: 1;
    font-weight: 700;
    color: var(--text-primary);
    padding: 0;
  }
  .amount-input::placeholder { color: var(--text-muted); opacity: 0.5; }

  .amount-display.tone-income  { border-color: var(--success); }
  .amount-display.tone-income  .amount-input,
  .amount-display.tone-income  .amount-currency { color: var(--success); }

  .amount-display.tone-expense { border-color: var(--danger); }
  .amount-display.tone-expense .amount-input,
  .amount-display.tone-expense .amount-currency { color: var(--danger); }

  .amount-display.tone-fuel    { border-color: var(--accent); }
  .amount-display.tone-fuel    .amount-input,
  .amount-display.tone-fuel    .amount-currency { color: var(--accent); }

  .amount-caption {
    display: block;
    margin-top: -0.75rem;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  /* ── Selección por chips — ancho uniforme sin importar el largo del texto:
     una grilla reparte 1fr por columna, todas las celdas quedan iguales ── */
  .chip-section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .chip-section-label {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .chip-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.5rem;
  }

  .chip-grid-sm { grid-template-columns: repeat(auto-fill, minmax(88px, 1fr)); gap: 0.4rem; }

  .chip {
    width: 100%;
    min-height: 2.3rem;
    padding: 0.5rem 0.6rem;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.82rem;
    font-weight: 500;
    text-align: center;
    line-height: 1.25;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    white-space: normal;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
  .chip:hover:not(.active) { color: var(--text-primary); border-color: var(--text-secondary); }
  .chip.active {
    background: var(--accent);
    color: var(--bg-base);
    border-color: var(--accent);
    font-weight: 700;
  }

  .chip-sm { min-height: 2rem; padding: 0.4rem 0.5rem; font-size: 0.74rem; }

  /* ── Kilometraje — separado del resto por una línea arriba, igual que
     las demás divisiones de sección en la app (nada de acento lateral). ── */
  .viaje-section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border-top: 1px solid var(--border);
    padding-top: 0.85rem;
  }

  .viaje-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
  }

  .switch {
    width: 32px;
    height: 17px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    position: relative;
    transition: background 0.15s, border-color 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 11px;
    height: 11px;
    background: var(--text-muted);
    transition: transform 0.15s, background 0.15s;
  }
  .switch.on { background: color-mix(in srgb, var(--accent) 22%, var(--bg-elevated)); border-color: var(--accent); }
  .switch.on::after { transform: translateX(13px); background: var(--accent); }

  .viaje-body {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .viaje-field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .viaje-field-label {
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  /* Campo de distancia — el <div> solo da contexto de posicionamiento;
     el borde/fondo se definen más abajo directamente en `input.km-value`
     para ganarle en especificidad al `input[type="text"]` genérico
     (mismo peso, pero declarado después → gana la cascada). El sufijo
     "km" es una etiqueta superpuesta dentro del mismo campo, no un
     segundo elemento con su propia caja. */
  .km-field {
    position: relative;
    width: fit-content;
  }

  .km-unit {
    position: absolute;
    top: 50%;
    right: 0.65rem;
    transform: translateY(-50%);
    font-size: 0.75rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .empty-hint { font-size: 0.78rem; color: var(--text-muted); margin: 0; }
  .empty-hint a { color: var(--accent); text-decoration: none; }
  .empty-hint a:hover { text-decoration: underline; }

  /* ── Fila de metadatos (fecha / nota / extraordinario) ── */
  .meta-row {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
  }
  .meta-row > * { flex: 1; min-width: 0; }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .meta-label {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .optional { font-weight: 400; text-transform: none; letter-spacing: 0; color: var(--text-muted); }

  .meta-input,
  input[type="text"] {
    -webkit-appearance: none;
    appearance: none;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.85rem;
    padding: 0.5rem 0.65rem;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
  }
  input[inputmode="numeric"] { font-family: var(--font-mono); }
  input:focus { border-color: var(--accent); }

  /* Misma especificidad que input[type="text"] de arriba (elemento+atributo
     vs. elemento+clase) — al declararse después, gana la cascada y puede
     pisar el padding/width genéricos para dejar espacio al sufijo "km". */
  input.km-value {
    width: 120px;
    font-size: 0.9rem;
    font-weight: 600;
    text-align: left;
    padding: 0.5rem 2.5rem 0.5rem 0.65rem;
  }

  .meta-field-check { flex: 0 0 auto !important; }

  .check-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.65rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.82rem;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.15s;
  }
  .check-row:hover { border-color: var(--text-secondary); }

  /* Checkbox propio (mismo patrón que .tx-check en Historial) — nada del
     checkbox nativo del navegador. */
  .check-box {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    background: var(--bg-surface);
    border: 1.5px solid var(--border);
    cursor: pointer;
    position: relative;
    transition: border-color 0.15s, background 0.15s;
  }
  .check-box:hover { border-color: var(--accent); }
  .check-box:checked { background: var(--accent); border-color: var(--accent); }
  .check-box:checked::after {
    content: "";
    position: absolute;
    left: 4px; top: 1px;
    width: 4px; height: 8px;
    border: 2px solid var(--bg-base);
    border-top: none; border-left: none;
    transform: rotate(45deg);
  }

  .hint-mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: var(--radius);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    font-size: 0.58rem;
    color: var(--text-muted);
    cursor: help;
  }

  /* Popup del hint — position:fixed con coordenadas calculadas en JS
     (igual que .cs-menu en CustomSelect): escapa de overflow:hidden y
     scroll de cualquier ancestro, nunca puede quedar tapado. */
  .hint-popup {
    position: fixed;
    transform: translateX(-50%);
    background: var(--bg-surface);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5rem 0.7rem;
    font-size: 0.72rem;
    line-height: 1.5;
    width: 220px;
    z-index: 999;
    pointer-events: none;
    text-align: left;
    font-weight: 400;
  }

  .cat-fixed-note {
    font-size: 0.75rem;
    color: var(--text-muted);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    padding: 0.4rem 0.75rem;
  }
  .cat-fixed-note strong { color: var(--accent); }

  .metas-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
  }
  .metas-hint a { color: var(--accent); text-decoration: none; }
  .metas-hint a:hover { text-decoration: underline; }

  /* ── Botón de guardar — tamaño de contenido, alineado a la derecha
     (convención de escritorio: la acción primaria no necesita ocupar todo
     el ancho de la tarjeta). ── */
  .save-bar-wrap {
    flex-shrink: 0;
    max-width: 880px;
    width: 100%;
    padding: 0 1.5rem 1.25rem;
    box-sizing: border-box;
    display: flex;
    justify-content: flex-end;
  }

  .save-bar {
    padding: 0.7rem 2.25rem;
    background: var(--accent);
    color: var(--bg-base);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-size: 0.8rem;
    font-weight: 700;
    transition: background 0.15s, opacity 0.15s;
  }
  .save-bar.save-income  { background: var(--success); }
  .save-bar.save-expense { background: var(--danger); }
  .save-bar.save-fuel    { background: var(--accent); }
  .save-bar:hover:not(:disabled) { opacity: 0.9; }
  .save-bar:disabled { opacity: 0.4; cursor: not-allowed; }

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
    flex-direction: row;
    align-items: flex-start;
    justify-content: space-between;
  }

  .banner-body { display: flex; flex-direction: column; gap: 0.2rem; flex: 1; }

  .banner-close {
    background: none;
    border: none;
    color: currentColor;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 0 0 0.5rem;
    cursor: pointer;
    opacity: 0.6;
    flex-shrink: 0;
  }
  .banner-close:hover { opacity: 1; }

  .banner pre { font-size: 0.72rem; opacity: 0.8; white-space: pre-wrap; word-break: break-all; }

  .auto-gas-note { font-size: 0.78rem; opacity: 0.85; }
  .fillup-warn { display: block; color: var(--accent); opacity: 1; }

  /* ════════════════════════════════════════════
     CONTEXTO (derecha) — un solo panel consolidado
  ════════════════════════════════════════════ */
  .context-col {
    width: 320px;
    flex-shrink: 0;
    overflow: hidden;
    padding: 1rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }

  :global(.ctx-scroll) { flex: 1; min-height: 0; }

  .ctx-panel {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.875rem 1rem;
    display: flex;
    flex-direction: column;
  }

  .ctx-panel-title {
    font-size: 0.72rem;
    font-weight: 700;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--accent);
    padding-bottom: 0.65rem;
    margin-bottom: 0.65rem;
    border-bottom: 1px solid var(--border);
  }

  .ctx-div { height: 1px; background: var(--border); margin: 0.65rem 0; }

  .ctx-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .ctx-row-col { flex-direction: column; align-items: stretch; gap: 0.4rem; }

  .ctx-row-label {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .ctx-row-value {
    font-size: 1rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }
  .ctx-row-value.income  { color: var(--success); }
  .ctx-row-value.expense { color: var(--danger); }

  /* Loading state */
  .ctx-loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1.5rem 0.5rem;
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .ctx-loading-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50%       { opacity: 1; }
  }

  /* Último registro */
  .ctx-last-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .ctx-last-cat {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ctx-last-amount {
    font-size: 0.92rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    color: var(--danger);
    flex-shrink: 0;
  }
  .ctx-last-amount.income { color: var(--success); }

  .ctx-last-meta {
    font-size: 0.75rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Este mes */
  .ctx-two-col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .ctx-stat { display: flex; flex-direction: column; gap: 0.2rem; }
  .ctx-stat-label { font-size: 0.72rem; color: var(--text-muted); }
  .ctx-stat-val {
    font-size: 0.88rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
  }
  .ctx-stat-val.income  { color: var(--success); }
  .ctx-stat-val.expense { color: var(--danger); }

  /* Objetivo */
  .ctx-goal-name {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ctx-progress-bar { height: 3px; background: var(--bg-elevated); overflow: hidden; }
  .ctx-progress-fill { height: 100%; background: var(--accent); transition: width 0.2s ease; }

  .ctx-goal-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.72rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
  }
  .ctx-goal-pct { font-weight: 600; color: var(--accent); }

  /* Presupuesto de categoría */
  .budget-bar-track { height: 4px; background: var(--bg-elevated); overflow: hidden; }
  .budget-bar-fill { height: 100%; background: var(--accent); transition: width 0.2s ease; }
  .budget-bar-fill.over { background: var(--danger); }

  .budget-nums {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
  }
  .budget-current { font-weight: 700; color: var(--text-primary); }
  .budget-current.over { color: var(--danger); }
  .budget-sep     { color: var(--text-muted); }
  .budget-target  { color: var(--text-secondary); }
  .budget-pct     { margin-left: auto; font-size: 0.72rem; color: var(--accent); font-weight: 600; }
  .budget-pct.over { color: var(--danger); }

  /* Últimas transacciones de la categoría */
  .ctx-tx-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .ctx-tx-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
  }

  .ctx-tx-date {
    flex-shrink: 0;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    width: 2.8rem;
  }

  .ctx-tx-note {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .ctx-tx-amount {
    flex-shrink: 0;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    color: var(--danger);
  }
  .ctx-tx-amount.income { color: var(--success); }
</style>
