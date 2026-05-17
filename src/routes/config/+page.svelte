<script lang="ts">
  import { gasApi, budgetApi, vehicleApi, routeApi, systemApi } from "$lib/api";
  import type { GasPrice, WeeklyGasPoint, Budget, RoutesCost, CustomRoute, Vehicle } from "$lib/types";
  import CustomSelect from "$lib/components/CustomSelect.svelte";

  let currentPrice   = $state<GasPrice | null>(null);
  let priceHistory   = $state<GasPrice[]>([]);
  let weeklyData     = $state<WeeklyGasPoint[]>([]);
  let budgets        = $state<Budget[]>([]);
  let routeCosts     = $state<RoutesCost | null>(null);
  let customRoutes   = $state<CustomRoute[]>([]);
  let vehicles       = $state<Vehicle[]>([]);
  let selectedVehicleId = $state<number | null>(null);
  let selectedVehicle   = $derived(vehicles.find(v => v.id === selectedVehicleId) ?? null);
  let loading        = $state(true);
  let pageError      = $state<string | null>(null);

  // ── Vehículos ─────────────────────────────────────────────────────────────
  let newVehicleName      = $state("");
  let newVehicleKmRaw     = $state("");
  let addingVehicle       = $state(false);
  let vehicleFormError    = $state<string | null>(null);
  let deletingVehicleId   = $state<number | null>(null);
  let editingVehicleId    = $state<number | null>(null);
  let editVehicleName     = $state("");
  let editVehicleKmRaw    = $state("");
  let savingVehicle       = $state(false);

  // ── Rutas personalizadas ──────────────────────────────────────────────────
  let newRouteName  = $state("");
  let newRouteKmRaw = $state("");
  let newRouteDesc  = $state("");
  let addingRoute   = $state(false);
  let routeError    = $state<string | null>(null);
  let deletingRouteId = $state<number | null>(null);

  // ── Actualizar precio ─────────────────────────────────────────────────────
  let newPriceRaw = $state("");
  let saving      = $state(false);
  let saveMsg     = $state<string | null>(null);
  let saveError   = $state<string | null>(null);

  let newPrice = $derived(parseInt(newPriceRaw.replace(/\D/g, ""), 10) || 0);

  // ── Presupuestos — edición inline ────────────────────────────────────────
  let editingBudget       = $state<string | null>(null);
  let editBudgetRaw       = $state("");
  let savingBudget        = $state(false);
  let savedBudgetCategory = $state<string | null>(null);

  // ── Presupuestos — crear / eliminar ──────────────────────────────────────
  let newBudgetName    = $state("");
  let newBudgetType    = $state<"ingreso" | "gasto">("gasto");
  let newBudgetIsFixed = $state(false);
  let addingBudget     = $state(false);
  let budgetFormError  = $state<string | null>(null);
  let deletingBudget   = $state<string | null>(null);
  let togglingFixed    = $state<string | null>(null);

  // ── Helpers ───────────────────────────────────────────────────────────────
  function formatCOP(n: number): string {
    return new Intl.NumberFormat("es-CO", {
      style: "currency", currency: "COP", minimumFractionDigits: 0,
    }).format(n);
  }

  function handlePriceInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    newPriceRaw = digits;
    e.currentTarget.value = digits ? new Intl.NumberFormat("es-CO").format(parseInt(digits, 10)) : "";
  }

  // ── Carga inicial ─────────────────────────────────────────────────────────
  $effect(() => {
    async function load() {
      loading = true; pageError = null;
      try {
        const [price, history, weekly, buds, routes, vehs] = await Promise.all([
          gasApi.getCurrent(),
          gasApi.list(20),
          gasApi.getWeeklyComparison(),
          budgetApi.list(),
          routeApi.list(),
          vehicleApi.list(),
        ]);

        currentPrice = price;
        priceHistory = history;

        weeklyData   = weekly;
        budgets      = buds;
        customRoutes = routes;
        vehicles     = vehs;
        if (selectedVehicleId === null && vehs.length > 0) selectedVehicleId = vehs[0].id;
        routeCosts   = await gasApi.getRouteCosts();
      } catch (e) {
        console.error("[config] load error:", e);
        pageError = "Error al cargar la configuración. Recarga la app.";
      } finally {
        loading = false;
      }
    }
    load();
  });

  // ── Guardar precio ────────────────────────────────────────────────────────
  async function handleSavePrice(ev: Event) {
    ev.preventDefault();
    if (newPrice <= 0) { saveError = "El precio debe ser mayor que 0."; return; }
    saving = true; saveError = null; saveMsg = null;
    try {
      const saved = await gasApi.registerManual(newPrice);
      currentPrice = saved;
      priceHistory = [saved, ...priceHistory.filter(p => p.date !== saved.date)].slice(0, 20);
      routeCosts   = await gasApi.getRouteCosts();
      saveMsg = `Precio actualizado: ${formatCOP(saved.price_per_gallon)}/galón`;
      newPriceRaw = "";
      setTimeout(() => { saveMsg = null; }, 3000);
    } catch (e) {
      console.error("[config] save price error:", e);
      saveError = "No se pudo guardar el precio. Intenta de nuevo.";
    } finally {
      saving = false;
    }
  }

  // ── Edición de presupuesto ────────────────────────────────────────────────
  function startEditBudget(category: string, amount: number) {
    editingBudget = category;
    editBudgetRaw = amount > 0 ? amount.toString() : "";
  }

  function handleBudgetInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    editBudgetRaw = digits;
    e.currentTarget.value = digits ? new Intl.NumberFormat("es-CO").format(parseInt(digits, 10)) : "";
  }

  async function saveEditBudget(category: string) {
    const amount = parseInt(editBudgetRaw, 10);
    if (isNaN(amount) || amount < 0) { editingBudget = null; return; }
    savingBudget = true;

    const prevBudgets = budgets;
    budgets = budgets.map(b => b.category === category ? { ...b, monthly_amount: amount } : b);
    editingBudget = null;

    try {
      const updated = await budgetApi.updateAmount(category, amount);
      budgets = budgets.map(b => b.category === category ? updated : b);
      savedBudgetCategory = category;
      setTimeout(() => { savedBudgetCategory = null; }, 1000);
    } catch (e) {
      budgets = prevBudgets;
      console.error("[config] save budget error:", e);
      pageError = "No se pudo guardar el presupuesto. Intenta de nuevo.";
    } finally {
      savingBudget = false;
    }
  }

  function handleBudgetKeydown(e: KeyboardEvent, category: string) {
    if (e.key === "Enter")  saveEditBudget(category);
    if (e.key === "Escape") { editingBudget = null; }
  }

  async function saveRouteAssoc(category: string, routeId: number | null) {
    try {
      await budgetApi.updateRoute(category, routeId);
      budgets = budgets.map(b => b.category === category ? { ...b, route_id: routeId } : b);
    } catch (e) {
      console.error("[config] save route assoc error:", e);
      pageError = "No se pudo guardar la asociación de ruta.";
    }
  }

  // ── Autoarranque ──────────────────────────────────────────────────────────
  let autostartEnabled = $state(false);
  let autostartLoading = $state(true);
  let autostartError   = $state<string | null>(null);

  $effect(() => {
    systemApi.getAutostart()
      .then(v => { autostartEnabled = v; autostartLoading = false; })
      .catch(() => { autostartLoading = false; });
  });

  async function toggleAutostart() {
    autostartError = null;
    const next = !autostartEnabled;
    try {
      await systemApi.setAutostart(next);
      autostartEnabled = next;
    } catch (e) {
      console.error("[config] autostart error:", e);
      const msg = typeof e === "string" ? e : (e as any)?.message ?? "";
      autostartError = msg || "No se pudo cambiar el autoarranque.";
    }
  }

  // ── Backup ────────────────────────────────────────────────────────────────
  let backupPath  = $state<string | null>(null);
  let backupError = $state<string | null>(null);
  let backupBusy  = $state(false);

  // ── Factory reset ─────────────────────────────────────────────────────────
  let resetStep       = $state<0 | 1 | 2>(0); // 0=cerrado, 1=confirmar, 2=escribir
  let resetInput      = $state("");
  let resetBusy       = $state(false);
  let resetSuccess    = $state(false);
  const RESET_PHRASE  = "BORRAR TODO";

  function openReset()  { resetStep = 1; resetInput = ""; resetSuccess = false; }
  function closeReset() { if (!resetBusy) { resetStep = 0; resetInput = ""; } }

  async function doFactoryReset() {
    if (resetInput !== RESET_PHRASE || resetBusy) return;
    resetBusy = true;
    try {
      await systemApi.factoryReset();
      // Limpiar todo el estado en memoria para reflejar la DB vacía
      budgets        = [];
      customRoutes   = [];
      vehicles       = [];
      currentPrice   = null;
      priceHistory   = [];
      weeklyData     = [];
      routeCosts     = null;
      selectedVehicleId = null;
      resetSuccess = true;
      resetStep    = 0;
      resetInput   = "";
      setTimeout(() => { resetSuccess = false; }, 3000);
    } catch (e) {
      console.error("[config] factory_reset error:", e);
      pageError = "Error al restablecer los datos. Intenta de nuevo.";
      resetStep = 0;
    } finally {
      resetBusy = false;
    }
  }

  async function handleBackup() {
    backupBusy = true;
    backupPath  = null;
    backupError = null;
    try {
      const path = await systemApi.backup();
      backupPath = path;
      setTimeout(() => { backupPath = null; }, 6000);
    } catch (e) {
      console.error("[config] backup error:", e);
      backupError = "No se pudo crear el backup. Verifica que la carpeta Documents exista.";
    } finally {
      backupBusy = false;
    }
  }

  async function addVehicle(ev: Event) {
    ev.preventDefault();
    const name = newVehicleName.trim();
    const km = parseFloat(newVehicleKmRaw.replace(",", "."));
    if (!name) { vehicleFormError = "El nombre es obligatorio."; return; }
    if (!km || km <= 0) { vehicleFormError = "El rendimiento debe ser mayor que 0."; return; }
    addingVehicle = true; vehicleFormError = null;
    try {
      const created = await vehicleApi.create({ name, km_per_gallon: km });
      vehicles = [...vehicles, created].sort((a, b) => a.name.localeCompare(b.name));
      if (selectedVehicleId === null) selectedVehicleId = created.id;
      newVehicleName = ""; newVehicleKmRaw = "";
    } catch (e: any) {
      vehicleFormError = e?.message ?? "No se pudo crear el vehículo.";
    } finally {
      addingVehicle = false;
    }
  }

  function startEditVehicle(v: Vehicle) {
    editingVehicleId = v.id;
    editVehicleName  = v.name;
    editVehicleKmRaw = v.km_per_gallon.toString();
  }

  async function saveEditVehicle(id: number) {
    const name = editVehicleName.trim();
    const km = parseFloat(editVehicleKmRaw.replace(",", "."));
    if (!name || !km || km <= 0) { editingVehicleId = null; return; }
    savingVehicle = true;
    try {
      const updated = await vehicleApi.update(id, { name, km_per_gallon: km });
      vehicles = vehicles.map(v => v.id === id ? updated : v);
      editingVehicleId = null;
    } catch (e) {
      console.error("[config] save vehicle error:", e);
      pageError = "No se pudo guardar el vehículo.";
    } finally {
      savingVehicle = false;
    }
  }

  async function deleteVehicle(id: number) {
    deletingVehicleId = id;
    try {
      await vehicleApi.remove(id);
      vehicles = vehicles.filter(v => v.id !== id);
      if (selectedVehicleId === id) selectedVehicleId = vehicles[0]?.id ?? null;
      if (editingVehicleId === id) editingVehicleId = null;
    } catch (e) {
      console.error("[config] delete vehicle error:", e);
      pageError = "No se pudo eliminar el vehículo.";
    } finally {
      deletingVehicleId = null;
    }
  }

  async function toggleFixed(category: string, currentFixed: boolean) {
    togglingFixed = category;
    try {
      const updated = await budgetApi.updateFixed(category, !currentFixed);
      budgets = budgets.map(b => b.category === category ? updated : b);
    } catch (e) {
      console.error("[config] toggle fixed error:", e);
      pageError = "No se pudo cambiar el tipo de ingreso.";
    } finally {
      togglingFixed = null;
    }
  }

  async function addBudget(ev: Event) {
    ev.preventDefault();
    const name = newBudgetName.trim();
    if (!name) { budgetFormError = "El nombre es obligatorio."; return; }
    addingBudget = true; budgetFormError = null;
    try {
      const created = await budgetApi.create(name, 0, newBudgetType, newBudgetType === "ingreso" ? newBudgetIsFixed : false);
      budgets = [...budgets, created].sort((a, b) => a.category.localeCompare(b.category));
      newBudgetName = "";
      newBudgetIsFixed = false;
    } catch (e: any) {
      budgetFormError = e?.message ?? "No se pudo crear la categoría.";
    } finally {
      addingBudget = false;
    }
  }

  async function deleteBudget(category: string) {
    deletingBudget = category;
    try {
      await budgetApi.remove(category);
      budgets = budgets.filter(b => b.category !== category);
      if (editingBudget === category) editingBudget = null;
    } catch (e) {
      console.error("[config] delete budget error:", e);
      pageError = "No se pudo eliminar la categoría.";
    } finally {
      deletingBudget = null;
    }
  }

  async function addCustomRoute(ev: Event) {
    ev.preventDefault();
    const km = parseFloat(newRouteKmRaw.replace(",", "."));
    if (!newRouteName.trim()) { routeError = "El nombre es obligatorio."; return; }
    if (!km || km <= 0) { routeError = "Los km deben ser mayores que 0."; return; }
    addingRoute = true; routeError = null;
    try {
      const saved = await routeApi.save({ name: newRouteName.trim(), km_round_trip: km, description: newRouteDesc.trim() || null });
      customRoutes = [...customRoutes, saved].sort((a, b) => a.name.localeCompare(b.name));
      newRouteName = ""; newRouteKmRaw = ""; newRouteDesc = "";
    } catch (e) {
      console.error("[config] save route error:", e);
      routeError = "No se pudo guardar la ruta.";
    } finally {
      addingRoute = false;
    }
  }

  async function removeCustomRoute(id: number) {
    deletingRouteId = id;
    try {
      await routeApi.remove(id);
      customRoutes = customRoutes.filter(r => r.id !== id);
    } catch (e) {
      console.error("[config] delete route error:", e);
      pageError = "No se pudo eliminar la ruta.";
    } finally {
      deletingRouteId = null;
    }
  }
</script>

<div class="config-shell">
  <div class="config-header">
    <h1>Configuración</h1>
  </div>

  {#if pageError}
    <div class="banner error"><strong>Error</strong> {pageError}</div>
  {/if}

  <div class="config-grid">
    <div class="config-left">
      {#if loading}
        <p class="muted">Cargando…</p>
      {:else}

    <!-- ══ Gasolina ══════════════════════════════════════════════════════════ -->
    <section class="section">
      <h2>Gasolina</h2>

      <!-- Precio actual -->
      <div class="gas-card">
        {#if currentPrice}
          <div class="gas-price-big">
            {formatCOP(currentPrice.price_per_gallon)}<span class="unit">/galón</span>
          </div>
          <div class="gas-meta">
            <span>{currentPrice.date}</span>
            <span class="source-badge source-{currentPrice.source}">{currentPrice.source}</span>
          </div>
        {:else}
          <p class="muted">Sin precio registrado.</p>
        {/if}
      </div>

      <!-- Costos por ruta -->
      {#if routeCosts}
        <div class="subsection">
          <h3>Costos por ruta <span class="hint-inline">· {formatCOP(routeCosts.precio_galon)}/gal{#if selectedVehicle} · {selectedVehicle.km_per_gallon} km/gal{/if}</span></h3>
          {#if vehicles.length > 1}
            <div class="vehicle-select-row">
              <span class="muted small">Vehículo:</span>
              <div style="--cs-padding: 0.18rem 0.4rem; font-size: 0.75rem;">
                <CustomSelect
                  bind:value={selectedVehicleId}
                  options={vehicles.map(v => ({ value: v.id, label: `${v.name} (${v.km_per_gallon} km/gal)` }))}
                />
              </div>
            </div>
          {/if}
          {#if customRoutes.length > 0 && selectedVehicle}
            <div class="route-costs">
              {#each customRoutes as route (route.id)}
                {@const cost = Math.round(route.km_round_trip / selectedVehicle.km_per_gallon * routeCosts.precio_galon)}
                <div class="route-row">
                  <span class="route-name">{route.name}</span>
                  <span class="route-km">{route.km_round_trip} km</span>
                  <span class="route-cost">{formatCOP(cost)}</span>
                  <button
                    class="cr-del"
                    onclick={() => removeCustomRoute(route.id)}
                    disabled={deletingRouteId === route.id}
                    aria-label="Eliminar ruta"
                  >{deletingRouteId === route.id ? "…" : "✕"}</button>
                </div>
              {/each}
            </div>
          {:else if customRoutes.length === 0}
            <p class="muted small">Sin rutas. Agrégalas abajo.</p>
          {:else}
            <p class="muted small">Agrega un vehículo para ver los costos.</p>
          {/if}
          {#if routeError}
            <div class="banner error small">{routeError}</div>
          {/if}
          <form class="route-add-form" onsubmit={addCustomRoute}>
            <input
              type="text"
              placeholder="Nombre"
              bind:value={newRouteName}
              class="route-input"
              disabled={addingRoute}
            />
            <input
              type="text"
              inputmode="decimal"
              placeholder="km redondo"
              bind:value={newRouteKmRaw}
              class="route-input route-input-km"
              disabled={addingRoute}
            />
            <button type="submit" class="btn-primary small" disabled={addingRoute || !newRouteName.trim() || !newRouteKmRaw}>
              {addingRoute ? "…" : "Agregar"}
            </button>
          </form>
        </div>
      {/if}

      <!-- Actualizar precio -->
      <div class="subsection">
        <h3>Actualizar precio hoy</h3>
        {#if saveMsg}
          <div class="banner success small">{saveMsg}</div>
        {/if}
        {#if saveError}
          <div class="banner error small">{saveError}</div>
        {/if}
        <form onsubmit={handleSavePrice} class="inline-form">
          <input
            type="text"
            inputmode="numeric"
            placeholder="Precio por galón"
            value={newPriceRaw ? new Intl.NumberFormat("es-CO").format(newPrice) : ""}
            oninput={handlePriceInput}
          />
          <button type="submit" class="btn-primary" disabled={saving || newPrice <= 0}>
            {saving ? "Guardando…" : "Guardar"}
          </button>
        </form>
      </div>

      <!-- Historial -->
      {#if priceHistory.length > 0}
        <div class="subsection">
          <h3>Historial de precios</h3>
          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>Fecha</th>
                  <th class="right">Precio/galón</th>
                  <th>Fuente</th>
                </tr>
              </thead>
              <tbody>
                {#each priceHistory as p (p.id)}
                  <tr>
                    <td>{p.date}</td>
                    <td class="right">{formatCOP(p.price_per_gallon)}</td>
                    <td><span class="source-badge source-{p.source}">{p.source}</span></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- Comparación semanal -->
      {#if weeklyData.length > 0}
        <div class="subsection">
          <h3>Comparación semanal</h3>
          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>Semana (lunes)</th>
                  <th class="right">Precio promedio</th>
                  <th class="right">Registros</th>
                </tr>
              </thead>
              <tbody>
                {#each weeklyData as w, i}
                  {@const prev = weeklyData[i + 1]}
                  <tr>
                    <td>{w.week_start}</td>
                    <td class="right">
                      {formatCOP(w.avg_price)}
                      {#if prev}
                        {@const delta = w.avg_price - prev.avg_price}
                        <span class="delta" class:up={delta > 0} class:down={delta < 0}>
                          {delta > 0 ? "↑" : delta < 0 ? "↓" : "—"}
                        </span>
                      {/if}
                    </td>
                    <td class="right muted">{w.entry_count}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </section>

    <!-- ══ Vehículos ══════════════════════════════════════════════════════════ -->
    <section class="section">
      <h2>Vehículos</h2>

      {#if vehicles.length === 0}
        <p class="muted">Sin vehículos. Agrega uno abajo.</p>
      {:else}
        <div class="vehicle-list">
          {#each vehicles as v (v.id)}
            <div class="vehicle-row">
              {#if editingVehicleId === v.id}
                <div class="vehicle-edit-form">
                  <input
                    type="text"
                    class="route-input"
                    bind:value={editVehicleName}
                    disabled={savingVehicle}
                    placeholder="Nombre"
                  />
                  <input
                    type="text"
                    inputmode="decimal"
                    class="route-input route-input-km"
                    bind:value={editVehicleKmRaw}
                    disabled={savingVehicle}
                    placeholder="km/gal"
                  />
                  <button
                    class="budget-icon-btn budget-save"
                    onclick={() => saveEditVehicle(v.id)}
                    disabled={savingVehicle}
                    title="Guardar"
                  >✓</button>
                  <button
                    class="budget-icon-btn budget-cancel"
                    onclick={() => { editingVehicleId = null; }}
                    disabled={savingVehicle}
                    title="Cancelar"
                  >✕</button>
                </div>
              {:else}
                <span class="vehicle-name">{v.name}</span>
                <span class="vehicle-km">{v.km_per_gallon} km/gal</span>
                <button class="cr-edit" onclick={() => startEditVehicle(v)} title="Editar">✎</button>
                <button
                  class="cr-del"
                  onclick={() => deleteVehicle(v.id)}
                  disabled={deletingVehicleId === v.id}
                  title="Eliminar"
                >{deletingVehicleId === v.id ? "…" : "✕"}</button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Agregar vehículo -->
      <div class="subsection">
        <h3>Agregar vehículo</h3>
        {#if vehicleFormError}
          <div class="banner error small">{vehicleFormError}</div>
        {/if}
        <form class="route-add-form" onsubmit={addVehicle}>
          <input
            type="text"
            placeholder="Nombre (ej. Moto, Carro)"
            bind:value={newVehicleName}
            class="route-input"
            disabled={addingVehicle}
          />
          <input
            type="text"
            inputmode="decimal"
            placeholder="km/gal"
            bind:value={newVehicleKmRaw}
            class="route-input route-input-km"
            disabled={addingVehicle}
          />
          <button
            type="submit"
            class="btn-primary small"
            disabled={addingVehicle || !newVehicleName.trim() || !newVehicleKmRaw}
          >{addingVehicle ? "…" : "Agregar"}</button>
        </form>
      </div>
    </section>

      {/if}
    </div>

    <div class="config-right">
      {#if !loading}
    <!-- ══ Presupuestos ══════════════════════════════════════════════════════ -->
    <section class="section">
      <h2>Presupuestos mensuales</h2>

      {#if budgets.length === 0}
        <p class="muted">Sin categorías. Agrega una abajo.</p>
      {:else}
        <div class="budget-list">
          {#each budgets as b (b.category)}
            <div class="budget-row" class:row-saved={savedBudgetCategory === b.category}>
              <div class="budget-cat">
                <span class="budget-name">{b.category}</span>
                {#if b.type === "ingreso"}
                  <button
                    class="fixed-pill"
                    class:fixed-pill-on={b.is_fixed}
                    onclick={() => toggleFixed(b.category, b.is_fixed)}
                    disabled={togglingFixed === b.category}
                    title={b.is_fixed ? "Ingreso fijo — clic para marcar como variable" : "Ingreso variable — clic para marcar como fijo"}
                  >{b.is_fixed ? "Fijo" : "Variable"}</button>
                {:else}
                  <span class="type-pill type-gasto">Gasto</span>
                {/if}
              </div>

              <div style="--cs-padding: 0.18rem 0.4rem; font-size: 0.75rem;">
                <CustomSelect
                  value={b.route_id}
                  options={[
                    { value: null, label: "Sin ruta" },
                    ...customRoutes.map(r => ({ value: r.id, label: r.name })),
                  ]}
                  onchange={(v) => saveRouteAssoc(b.category, v)}
                />
              </div>

              <div class="budget-amount-cell">
                {#if editingBudget === b.category}
                  <div class="budget-edit-row">
                    <!-- svelte-ignore a11y_autofocus -->
                    <input type="text" inputmode="numeric" class="inline-input"
                      value={editBudgetRaw ? new Intl.NumberFormat("es-CO").format(parseInt(editBudgetRaw, 10)) : ""}
                      oninput={handleBudgetInput} onkeydown={(e) => handleBudgetKeydown(e, b.category)}
                      disabled={savingBudget} autofocus />
                    <button class="budget-icon-btn budget-save" onclick={() => saveEditBudget(b.category)} disabled={savingBudget} title="Guardar">✓</button>
                    <button class="budget-icon-btn budget-cancel" onclick={() => { editingBudget = null; }} disabled={savingBudget} title="Cancelar">✕</button>
                  </div>
                {:else}
                  <button class="amount-btn" onclick={() => startEditBudget(b.category, b.monthly_amount)}>
                    {b.monthly_amount > 0 ? formatCOP(b.monthly_amount) : "—"}
                  </button>
                {/if}
              </div>

              <button
                class="cr-del"
                onclick={() => deleteBudget(b.category)}
                disabled={deletingBudget === b.category}
                title="Eliminar categoría"
              >{deletingBudget === b.category ? "…" : "✕"}</button>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Agregar categoría -->
      <div class="subsection">
        <h3>Agregar categoría</h3>
        {#if budgetFormError}
          <div class="banner error small">{budgetFormError}</div>
        {/if}
        <form class="budget-add-form" onsubmit={addBudget}>
          <input
            type="text"
            placeholder="Nombre"
            bind:value={newBudgetName}
            class="route-input"
            disabled={addingBudget}
          />
          <div class="budget-type-select">
            <CustomSelect
              bind:value={newBudgetType}
              options={[
                { value: "gasto",   label: "Gasto" },
                { value: "ingreso", label: "Ingreso" },
              ]}
              disabled={addingBudget}
            />
          </div>
          {#if newBudgetType === "ingreso"}
            <button
              type="button"
              class="fixed-pill"
              class:fixed-pill-on={newBudgetIsFixed}
              onclick={() => newBudgetIsFixed = !newBudgetIsFixed}
              disabled={addingBudget}
              title={newBudgetIsFixed ? "Ingreso fijo — clic para marcar como variable" : "Ingreso variable — clic para marcar como fijo"}
            >{newBudgetIsFixed ? "Fijo" : "Variable"}</button>
          {/if}
          <button type="submit" class="btn-primary small" disabled={addingBudget || !newBudgetName.trim()}>
            {addingBudget ? "…" : "Agregar"}
          </button>
        </form>
      </div>
    </section>
      {/if}

  <!-- ══ Sistema ═════════════════════════════════════════════════════════ -->
  <section class="section">
    <h2>Sistema</h2>


    <!-- Autoarranque -->
    <div class="subsection">
      <div class="row-between">
        <div>
          <span class="row-label">Iniciar con el sistema</span>
          <span class="row-hint">Abrir Finanzas automáticamente al iniciar sesión</span>
        </div>
        {#if autostartLoading}
          <span class="muted">…</span>
        {:else}
          <button
            type="button"
            class="toggle"
            class:on={autostartEnabled}
            onclick={toggleAutostart}
            aria-label="Autoarranque"
          ></button>
        {/if}
      </div>
      {#if autostartError}
        <div class="banner error small">{autostartError}</div>
      {/if}
    </div>

    <!-- Backup -->
    <div class="subsection">
      <h3>Base de datos local</h3>
      {#if backupPath}
        <div class="banner success small">Backup guardado en: {backupPath}</div>
      {/if}
      {#if backupError}
        <div class="banner error small">{backupError}</div>
      {/if}
      <button
        type="button"
        class="btn-secondary"
        onclick={handleBackup}
        disabled={backupBusy}
      >
        {backupBusy ? "Exportando…" : "💾 Exportar backup"}
      </button>
    </div>
  </section>

  <!-- ══ Datos ════════════════════════════════════════════════════════════ -->
  <section class="section danger-zone">
    <h2>Datos</h2>
    {#if resetSuccess}
      <div class="banner success small">Datos eliminados. La app está lista para usar.</div>
    {/if}
    <div class="subsection">
      <p class="danger-hint">Elimina permanentemente todas las transacciones, objetivos, historial de gasolina, categorías, rutas y vehículos. La app quedará vacía lista para configurar desde cero.</p>
      <button type="button" class="btn-danger" onclick={openReset}>
        🗑 Restablecer datos de fábrica
      </button>
    </div>
  </section>
    </div>
  </div>
</div>

<!-- Dialog factory reset paso 1 -->
{#if resetStep === 1}
  <div class="modal-overlay" role="button" tabindex="-1" onclick={closeReset} onkeydown={(e) => { if (e.key === "Escape") closeReset(); }}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>¿Restablecer datos de fábrica?</h2>
      <p class="modal-body">Esto eliminará <strong>TODAS</strong> las transacciones, objetivos e historial de gasolina. Esta acción no se puede deshacer.</p>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={closeReset}>Cancelar</button>
        <button class="btn-danger-confirm" onclick={() => { resetStep = 2; }}>Sí, continuar</button>
      </div>
    </div>
  </div>
{/if}

<!-- Dialog factory reset paso 2 -->
{#if resetStep === 2}
  <div class="modal-overlay" role="button" tabindex="-1" onclick={closeReset} onkeydown={(e) => { if (e.key === "Escape") closeReset(); }}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>Confirmación final</h2>
      <p class="modal-body">Escribe <strong>{RESET_PHRASE}</strong> para confirmar:</p>
      <input
        type="text"
        class="reset-input"
        bind:value={resetInput}
        placeholder={RESET_PHRASE}
        disabled={resetBusy}
        onkeydown={(e) => { if (e.key === "Enter") doFactoryReset(); }}
      />
      <div class="modal-actions">
        <button class="btn-cancel" onclick={closeReset} disabled={resetBusy}>Cancelar</button>
        <button
          class="btn-danger-confirm"
          onclick={doFactoryReset}
          disabled={resetInput !== RESET_PHRASE || resetBusy}
        >
          {resetBusy ? "Borrando…" : "Confirmar y borrar"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .config-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    padding: 0.875rem 1rem;
    gap: 0.5rem;
    box-sizing: border-box;
  }

  .config-header { flex-shrink: 0; }

  .config-grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    overflow: hidden;
  }

  .config-left,
  .config-right {
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
    padding-bottom: 1rem;
    scrollbar-width: thin;
    scrollbar-color: #2a2a40 transparent;
  }
  .config-left::-webkit-scrollbar,
  .config-right::-webkit-scrollbar { width: 4px; }
  .config-left::-webkit-scrollbar-track,
  .config-right::-webkit-scrollbar-track { background: transparent; }
  .config-left::-webkit-scrollbar-thumb,
  .config-right::-webkit-scrollbar-thumb {
    background: #2a2a40;
    border-radius: 999px;
  }
  .config-left::-webkit-scrollbar-thumb:hover,
  .config-right::-webkit-scrollbar-thumb:hover { background: #3a3a55; }

  h1 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); }
  h3 { font-size: 0.82rem; font-weight: 600; color: var(--text-secondary); margin-bottom: 0.5rem; }

  .hint-inline { font-weight: 400; color: var(--text-muted); }

  .section {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .subsection { display: flex; flex-direction: column; gap: 0.5rem; }

  /* ── Precio actual ── */
  .gas-card {
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 1rem 1.25rem;
  }

  .gas-price-big {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.03em;
    line-height: 1;
  }

  .unit { font-size: 0.85rem; font-weight: 400; color: var(--text-muted); margin-left: 0.25rem; }

  .gas-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.35rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .source-badge {
    font-size: 0.65rem;
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
  }
  .source-manual   { background: color-mix(in srgb, var(--accent)  20%, transparent); color: var(--accent);  }
  .source-scraping { background: color-mix(in srgb, var(--success) 20%, transparent); color: var(--success); }

  /* ── Costos por ruta ── */
  .route-costs {
    display: flex;
    flex-direction: column;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .route-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.875rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }

  .route-row:last-child { border-bottom: none; }

  .route-name { flex: 1; color: var(--text-primary); font-weight: 500; }
  .route-km   { color: var(--text-muted); font-size: 0.78rem; min-width: 45px; }
  .route-cost { color: var(--accent); font-weight: 700; font-size: 0.9rem; min-width: 80px; text-align: right; }

  /* ── Rutas personalizadas ── */
  .route-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .custom-route-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.75rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
  }
  .custom-route-row:last-child { border-bottom: none; }

  .cr-name { font-weight: 500; color: var(--text-primary); flex-shrink: 0; }
  .cr-km   { font-size: 0.75rem; color: var(--text-muted); flex-shrink: 0; min-width: 50px; }
  .cr-desc { font-size: 0.75rem; color: var(--text-muted); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .cr-del {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 0.7rem;
    color: var(--text-muted);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    transition: color 0.15s, background 0.15s;
  }
  .cr-del:hover:not(:disabled) { color: var(--danger); background: color-mix(in srgb, var(--danger) 12%, transparent); }
  .cr-del:disabled { opacity: 0.4; cursor: not-allowed; }

  .route-add-form {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .route-input {
    -webkit-appearance: none;
    appearance: none;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.78rem;
    padding: 0.32rem 0.6rem;
    outline: none;
    flex: 1;
    min-width: 120px;
    transition: border-color 0.15s;
  }
  .route-input:focus { border-color: var(--accent); }
  .route-input::placeholder { color: var(--text-muted); }
  .route-input-km { max-width: 100px; flex: none; }

  .small { font-size: 0.78rem; }
  .btn-primary.small { padding: 0.32rem 0.7rem; font-size: 0.78rem; }

  /* ── Formulario inline ── */
  .inline-form { display: flex; gap: 0.5rem; }

  /* ── Tablas ── */
  .table-wrap { overflow-x: auto; }

  .data-table {
    width: 100%;
    font-size: 0.82rem;
    border-collapse: collapse;
  }

  .data-table th,
  .data-table td {
    padding: 0.4rem 0.5rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .data-table th { color: var(--text-muted); font-weight: 500; font-size: 0.72rem; }
  .data-table td { color: var(--text-secondary); }

  .right { text-align: right; }

  .delta { font-size: 0.7rem; margin-left: 0.2rem; }
  .delta.up   { color: var(--danger);  }
  .delta.down { color: var(--success); }

  /* ── Presupuestos CRUD ── */
  .budget-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .budget-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 110px 148px 28px;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--border);
    transition: background 0.3s;
  }
  .budget-row:first-child {
    border-top-left-radius: calc(var(--radius) - 1px);
    border-top-right-radius: calc(var(--radius) - 1px);
  }
  .budget-row:last-child {
    border-bottom: none;
    border-bottom-left-radius: calc(var(--radius) - 1px);
    border-bottom-right-radius: calc(var(--radius) - 1px);
  }
  .budget-row.row-saved { background: color-mix(in srgb, var(--success) 12%, transparent); }

  .budget-cat {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
  }
  .budget-name {
    font-size: 0.82rem;
    color: var(--text-primary);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .type-pill {
    font-size: 0.62rem;
    font-weight: 600;
    padding: 0.1rem 0.35rem;
    border-radius: 999px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .type-pill.type-ingreso {
    background: color-mix(in srgb, var(--success) 18%, transparent);
    color: var(--success);
  }
  .type-pill.type-gasto {
    background: color-mix(in srgb, var(--danger) 15%, transparent);
    color: var(--danger);
  }

  .fixed-pill {
    font-size: 0.62rem;
    font-weight: 600;
    padding: 0.1rem 0.35rem;
    border-radius: 999px;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    background: color-mix(in srgb, var(--text-muted) 15%, transparent);
    color: var(--text-muted);
    border: 1px solid color-mix(in srgb, var(--text-muted) 25%, transparent);
  }
  .fixed-pill.fixed-pill-on {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .fixed-pill:hover:not(:disabled) { opacity: 0.75; }
  .fixed-pill:disabled { opacity: 0.4; cursor: not-allowed; }

  .budget-type-select {
    --cs-padding: 0.32rem 0.6rem;
    font-size: 0.78rem;
    flex-shrink: 0;
    min-width: 90px;
  }


  .route-placeholder { font-size: 0.78rem; color: var(--text-muted); width: 110px; text-align: center; }

  .budget-amount-cell { display: flex; justify-content: flex-end; width: 148px; overflow: hidden; }

  .budget-add-form {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    flex-wrap: wrap;
  }


  .amount-btn {
    font-size: 0.82rem;
    color: var(--text-secondary);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    transition: background 0.15s, color 0.15s;
    cursor: pointer;
  }
  .amount-btn:hover { background: var(--bg-elevated); color: var(--accent); }

  .budget-edit-row {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    justify-content: flex-end;
  }

  .inline-input {
    -webkit-appearance: none;
    appearance: none;
    background-color: #14141f;
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: #e8e8f0;
    font: inherit;
    font-size: 0.82rem;
    padding: 0.2rem 0.4rem;
    outline: none;
    text-align: right;
  }

  .budget-icon-btn {
    width: 24px;
    height: 24px;
    border-radius: 5px;
    font-size: 0.78rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
  }
  .budget-icon-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .budget-save {
    background: color-mix(in srgb, var(--success) 18%, var(--bg-elevated));
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
  }
  .budget-save:hover:not(:disabled) { background: color-mix(in srgb, var(--success) 30%, var(--bg-elevated)); }

  .budget-cancel {
    background: var(--bg-elevated);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .budget-cancel:hover:not(:disabled) { color: var(--danger); }

  /* ── Inputs ── */
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

  /* inline-input needs to come after input[type="text"] to win the cascade */
  .budget-edit-row input {
    width: 80px;
    flex-shrink: 0;
    box-sizing: border-box;
  }

  /* ── Botones ── */
  .btn-primary {
    padding: 0.5rem 1rem;
    background: var(--accent);
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    border-radius: var(--radius);
    white-space: nowrap;
    flex-shrink: 0;
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Banners ── */
  .banner {
    border-radius: var(--radius);
    padding: 0.55rem 0.9rem;
    font-size: 0.82rem;
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
  .banner.small { padding: 0.35rem 0.75rem; }

  .hint { font-size: 0.75rem; color: var(--text-muted); }
  .muted { color: var(--text-muted); font-size: 0.82rem; }

  /* ── Vehículos ── */
  .vehicle-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .vehicle-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
  }
  .vehicle-row:last-child { border-bottom: none; }

  .vehicle-name { flex: 1; font-weight: 500; color: var(--text-primary); }
  .vehicle-km   { font-size: 0.75rem; color: var(--text-muted); flex-shrink: 0; min-width: 70px; }

  .vehicle-edit-form {
    display: flex;
    gap: 0.35rem;
    align-items: center;
    flex: 1;
  }

  .cr-edit {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    transition: color 0.15s, background 0.15s;
  }
  .cr-edit:hover { color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent); }

  .vehicle-select-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* ── Sistema ── */
  .row-between {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .row-label { font-size: 0.875rem; font-weight: 500; color: var(--text-primary); }
  .row-hint  { font-size: 0.75rem; color: var(--text-muted); display: block; margin-top: 0.1rem; }

  .toggle {
    width: 40px;
    height: 22px;
    border-radius: 999px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    position: relative;
    flex-shrink: 0;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
  }
  .toggle::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: transform 0.2s, background 0.2s;
  }
  .toggle.on {
    background: color-mix(in srgb, var(--accent) 25%, var(--bg-elevated));
    border-color: var(--accent);
  }
  .toggle.on::after {
    transform: translateX(18px);
    background: var(--accent);
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 500;
    align-self: flex-start;
    transition: color 0.15s, background 0.15s;
  }
  .btn-secondary:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-surface); }
  .btn-secondary:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── Datos ── */
  .danger-zone {
    border-color: color-mix(in srgb, var(--danger) 35%, transparent);
  }
  .danger-zone h2 { color: var(--danger); }
  .danger-hint {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0;
  }
  .btn-danger {
    padding: 0.5rem 1rem;
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-elevated));
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    border-radius: var(--radius);
    color: var(--danger);
    font-size: 0.85rem;
    font-weight: 600;
    align-self: flex-start;
    transition: background 0.15s;
  }
  .btn-danger:hover { background: color-mix(in srgb, var(--danger) 25%, var(--bg-elevated)); }

  /* ── Modal ── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.5rem;
    width: min(420px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .modal h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); }
  .modal-body { font-size: 0.875rem; color: var(--text-secondary); margin: 0; line-height: 1.5; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
  .btn-cancel {
    padding: 0.45rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 500;
  }
  .btn-cancel:hover:not(:disabled) { color: var(--text-primary); }
  .btn-cancel:disabled { opacity: 0.45; cursor: not-allowed; }
  .btn-danger-confirm {
    padding: 0.45rem 1rem;
    background: var(--danger);
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    transition: opacity 0.15s;
  }
  .btn-danger-confirm:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-danger-confirm:hover:not(:disabled) { opacity: 0.85; }
  .reset-input {
    -webkit-appearance: none;
    appearance: none;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.9rem;
    padding: 0.5rem 0.75rem;
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
    box-sizing: border-box;
  }
  .reset-input:focus { border-color: var(--danger); }
</style>
