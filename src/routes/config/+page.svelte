<script lang="ts">
  import { gasApi, budgetApi, categoryApi, vehicleApi, routeApi, systemApi, fillupApi } from "$lib/api";
  import type { GasPrice, WeeklyGasPoint, CategoryBudgetRow, RoutesCost, RouteV2, VehicleV2 } from "$lib/types";
  import { mPerLToKmPerGallon, mlToGallons, metersToKm, ML_PER_GALLON } from "$lib/constants";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import ScrollArea from "$lib/components/ScrollArea.svelte";

  let currentPrice   = $state<GasPrice | null>(null);
  let priceHistory   = $state<GasPrice[]>([]);
  let weeklyData     = $state<WeeklyGasPoint[]>([]);
  let budgets        = $state<CategoryBudgetRow[]>([]);
  let routeCosts     = $state<RoutesCost | null>(null);
  let customRoutes   = $state<RouteV2[]>([]);
  let vehicles       = $state<VehicleV2[]>([]);
  let selectedVehicleId = $state<string | null>(null);
  let selectedVehicle   = $derived(vehicles.find(v => v.id === selectedVehicleId) ?? null);
  let selectedVehicleKmPerGallon = $derived(selectedVehicle ? mPerLToKmPerGallon(selectedVehicle.efficiency_m_per_l) : 0);
  let loading        = $state(true);
  let pageError      = $state<string | null>(null);

  // ── Pestañas — una sección a la vez, no las cinco de golpe ───────────────
  type Tab = "gasolina" | "vehiculos" | "presupuestos" | "sistema" | "datos";
  let activeTab = $state<Tab>("gasolina");
  let showPriceTables = $state(false);

  // ── Vehículos ─────────────────────────────────────────────────────────────
  let newVehicleName      = $state("");
  let newVehicleKmRaw     = $state("");
  let newVehicleTankRaw   = $state("");
  let addingVehicle       = $state(false);
  let vehicleFormError    = $state<string | null>(null);
  let deletingVehicleId   = $state<string | null>(null);
  let editingVehicleId    = $state<string | null>(null);
  let editVehicleName     = $state("");
  let editVehicleKmRaw    = $state("");
  let editVehicleTankRaw  = $state("");
  let savingVehicle       = $state(false);

  // ── Rutas personalizadas ──────────────────────────────────────────────────
  let newRouteName  = $state("");
  let newRouteKmRaw = $state("");
  let newRouteDesc  = $state("");
  let addingRoute   = $state(false);
  let routeError    = $state<string | null>(null);
  let deletingRouteId = $state<string | null>(null);

  // ── Reset de nivel de tanque — no borra tanqueos ni viajes, solo ancla el
  // conteo desde hoy (útil tras corregir el rendimiento de un vehículo). ──
  let resetLevelRaw   = $state("");
  let resetLevelNote  = $state("");
  let resettingLevel  = $state(false);
  let resetLevelMsg   = $state<string | null>(null);
  let resetLevelError = $state<string | null>(null);
  let resetLevelGallons = $derived(parseFloat(resetLevelRaw.replace(",", ".")) || 0);

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
  let newBudgetType    = $state<"income" | "expense">("expense");
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
          budgetApi.listWithCategories(),
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
  function startEditBudget(categoryId: string, amount: number) {
    editingBudget = categoryId;
    editBudgetRaw = amount > 0 ? amount.toString() : "";
  }

  function handleBudgetInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "");
    editBudgetRaw = digits;
    e.currentTarget.value = digits ? new Intl.NumberFormat("es-CO").format(parseInt(digits, 10)) : "";
  }

  async function saveEditBudget(categoryId: string) {
    const amount = parseInt(editBudgetRaw, 10);
    if (isNaN(amount) || amount < 0) { editingBudget = null; return; }
    savingBudget = true;

    const prevBudgets = budgets;
    budgets = budgets.map(b => b.category.id === categoryId ? { ...b, monthly_cop: amount } : b);
    editingBudget = null;

    try {
      await budgetApi.setMonthly(categoryId, amount);
      savedBudgetCategory = categoryId;
      setTimeout(() => { savedBudgetCategory = null; }, 1000);
    } catch (e) {
      budgets = prevBudgets;
      console.error("[config] save budget error:", e);
      pageError = "No se pudo guardar el presupuesto. Intenta de nuevo.";
    } finally {
      savingBudget = false;
    }
  }

  function handleBudgetKeydown(e: KeyboardEvent, categoryId: string) {
    if (e.key === "Enter")  saveEditBudget(categoryId);
    if (e.key === "Escape") { editingBudget = null; }
  }

  async function saveRouteAssoc(row: CategoryBudgetRow, routeId: string | null) {
    try {
      const updated = await categoryApi.update(row.category.id, row.category.is_fixed, routeId);
      budgets = budgets.map(b => b.category.id === row.category.id ? { ...b, category: updated } : b);
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
    const tank = parseFloat(newVehicleTankRaw.trim().replace(",", "."));
    if (!name) { vehicleFormError = "El nombre es obligatorio."; return; }
    if (!km || km <= 0) { vehicleFormError = "El rendimiento debe ser mayor que 0."; return; }
    if (!tank || tank <= 0) { vehicleFormError = "La capacidad del tanque debe ser mayor que 0."; return; }
    addingVehicle = true; vehicleFormError = null;
    try {
      const created = await vehicleApi.create({ name, km_per_gallon: km, tank_gallons: tank });
      vehicles = [...vehicles, created].sort((a, b) => a.name.localeCompare(b.name));
      if (selectedVehicleId === null) selectedVehicleId = created.id;
      newVehicleName = ""; newVehicleKmRaw = ""; newVehicleTankRaw = "";
    } catch (e: any) {
      vehicleFormError = e?.message ?? "No se pudo crear el vehículo.";
    } finally {
      addingVehicle = false;
    }
  }

  function startEditVehicle(v: VehicleV2) {
    editingVehicleId   = v.id;
    editVehicleName    = v.name;
    editVehicleKmRaw   = mPerLToKmPerGallon(v.efficiency_m_per_l).toFixed(1);
    // Vehículos creados antes de que la capacidad fuera obligatoria pueden no
    // tenerla — se deja en blanco y hay que completarla para poder guardar.
    editVehicleTankRaw = v.tank_capacity_ml != null ? mlToGallons(v.tank_capacity_ml).toFixed(1) : "";
  }

  async function saveEditVehicle(id: string) {
    const name = editVehicleName.trim();
    const km = parseFloat(editVehicleKmRaw.replace(",", "."));
    const tank = parseFloat(editVehicleTankRaw.trim().replace(",", "."));
    if (!name || !km || km <= 0) { editingVehicleId = null; return; }
    if (!tank || tank <= 0) { editingVehicleId = null; return; }
    savingVehicle = true;
    try {
      const updated = await vehicleApi.update(id, { name, km_per_gallon: km, tank_gallons: tank });
      vehicles = vehicles.map(v => v.id === id ? updated : v);
      editingVehicleId = null;
    } catch (e) {
      console.error("[config] save vehicle error:", e);
      pageError = "No se pudo guardar el vehículo.";
    } finally {
      savingVehicle = false;
    }
  }

  async function deleteVehicle(id: string) {
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

  async function toggleFixed(row: CategoryBudgetRow) {
    togglingFixed = row.category.id;
    try {
      const updated = await categoryApi.update(row.category.id, !row.category.is_fixed, row.category.route_id);
      budgets = budgets.map(b => b.category.id === row.category.id ? { ...b, category: updated } : b);
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
      const created = await categoryApi.create({
        name, kind: newBudgetType, is_fixed: newBudgetType === "income" ? newBudgetIsFixed : false, route_id: null,
      });
      budgets = [...budgets, { category: created, monthly_cop: 0 }].sort((a, b) => a.category.name.localeCompare(b.category.name));
      newBudgetName = "";
      newBudgetIsFixed = false;
    } catch (e: any) {
      budgetFormError = e?.message ?? "No se pudo crear la categoría.";
    } finally {
      addingBudget = false;
    }
  }

  async function deleteBudget(categoryId: string) {
    deletingBudget = categoryId;
    try {
      await categoryApi.remove(categoryId);
      budgets = budgets.filter(b => b.category.id !== categoryId);
      if (editingBudget === categoryId) editingBudget = null;
    } catch (e: any) {
      console.error("[config] delete budget error:", e);
      pageError = e?.message ?? "No se pudo eliminar la categoría.";
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

  async function removeCustomRoute(id: string) {
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

  async function handleResetFuelLevel(ev: Event) {
    ev.preventDefault();
    if (!selectedVehicleId) { resetLevelError = "Selecciona un vehículo."; return; }
    if (resetLevelRaw === "" || resetLevelGallons < 0) { resetLevelError = "Ingresa un nivel válido."; return; }
    resettingLevel = true; resetLevelError = null; resetLevelMsg = null;
    try {
      const now = new Date();
      const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
      await fillupApi.resetFuelLevel({
        vehicle_id: selectedVehicleId,
        level_ml: Math.round(resetLevelGallons * ML_PER_GALLON),
        occurred_on: today,
        note: resetLevelNote.trim() || null,
      });
      resetLevelMsg = `Nivel reseteado a ${resetLevelGallons.toFixed(1)} gal. Los tanqueos y viajes anteriores siguen intactos.`;
      resetLevelRaw = ""; resetLevelNote = "";
      setTimeout(() => { resetLevelMsg = null; }, 4000);
    } catch (e) {
      console.error("[config] reset fuel level error:", e);
      resetLevelError = "No se pudo resetear el nivel. Intenta de nuevo.";
    } finally {
      resettingLevel = false;
    }
  }
</script>

<div class="config-shell">
  <header class="config-header">
    <h1>Configuración</h1>
  </header>

  {#if pageError}
    <div class="banner error"><strong>Error</strong> {pageError}</div>
  {/if}

  <nav class="tabs">
    <button type="button" class="tab" class:active={activeTab === "gasolina"} onclick={() => { activeTab = "gasolina"; }}>Gasolina</button>
    <button type="button" class="tab" class:active={activeTab === "vehiculos"} onclick={() => { activeTab = "vehiculos"; }}>Vehículos</button>
    <button type="button" class="tab" class:active={activeTab === "presupuestos"} onclick={() => { activeTab = "presupuestos"; }}>Presupuestos</button>
    <button type="button" class="tab" class:active={activeTab === "sistema"} onclick={() => { activeTab = "sistema"; }}>Sistema</button>
    <button type="button" class="tab tab-danger" class:active={activeTab === "datos"} onclick={() => { activeTab = "datos"; }}>Datos</button>
  </nav>

  <div class="tab-body">
    <ScrollArea class="tab-scroll" scrollbar="thin">
    {#if loading}
      <p class="muted">Cargando…</p>
    {:else}

      <!-- ══════════════════════ GASOLINA ══════════════════════ -->
      {#if activeTab === "gasolina"}

        <div class="panel">
          <div class="price-hero">
            {#if currentPrice}
              <span class="price-value">{formatCOP(currentPrice.price_per_gallon)}</span>
              <span class="price-unit">/galón</span>
            {:else}
              <span class="price-value muted">Sin precio registrado</span>
            {/if}
          </div>
          {#if currentPrice}
            <div class="price-meta">
              <span>{currentPrice.date}</span>
              <span class="source-badge source-{currentPrice.source}">{currentPrice.source}</span>
            </div>
          {/if}

          {#if saveMsg}<div class="banner success small">{saveMsg}</div>{/if}
          {#if saveError}<div class="banner error small">{saveError}</div>{/if}
          <form onsubmit={handleSavePrice} class="inline-form">
            <input
              type="text"
              inputmode="numeric"
              placeholder="Nuevo precio por galón"
              value={newPriceRaw ? new Intl.NumberFormat("es-CO").format(newPrice) : ""}
              oninput={handlePriceInput}
            />
            <button type="submit" class="btn-primary" disabled={saving || newPrice <= 0}>
              {saving ? "Guardando…" : "Guardar"}
            </button>
          </form>
        </div>

        <div class="panel">
          <div class="panel-header">
            <span class="panel-title">Costos por ruta</span>
            {#if routeCosts}
              <span class="panel-title-hint">{formatCOP(routeCosts.precio_galon)}/gal{#if selectedVehicle} · {selectedVehicleKmPerGallon.toFixed(1)} km/gal{/if}</span>
            {/if}
          </div>

          {#if vehicles.length > 1}
            <div class="chip-grid chip-grid-sm">
              {#each vehicles as v (v.id)}
                <button
                  type="button"
                  class="chip chip-sm"
                  class:active={selectedVehicleId === v.id}
                  onclick={() => { selectedVehicleId = v.id; }}
                >{v.name}</button>
              {/each}
            </div>
          {/if}

          {#if customRoutes.length > 0 && selectedVehicle}
            <div class="item-list">
              {#each customRoutes as route (route.id)}
                {@const routeKm = metersToKm(route.distance_m)}
                {@const cost = Math.round(routeKm / selectedVehicleKmPerGallon * routeCosts!.precio_galon)}
                <div class="item-row">
                  <span class="item-name">{route.name}</span>
                  <span class="item-meta">{routeKm} km</span>
                  <span class="item-value">{formatCOP(cost)}</span>
                  <div class="item-actions">
                    <button
                      class="item-act danger"
                      onclick={() => removeCustomRoute(route.id)}
                      disabled={deletingRouteId === route.id}
                    >{deletingRouteId === route.id ? "…" : "Eliminar"}</button>
                  </div>
                </div>
              {/each}
            </div>
          {:else if customRoutes.length === 0}
            <p class="muted small">Sin rutas todavía.</p>
          {:else}
            <p class="muted small">Agrega un vehículo para ver los costos.</p>
          {/if}

          {#if routeError}<div class="banner error small">{routeError}</div>{/if}
          <form class="inline-form-3" onsubmit={addCustomRoute}>
            <input type="text" placeholder="Nombre de la ruta" bind:value={newRouteName} disabled={addingRoute} />
            <input type="text" inputmode="decimal" placeholder="km redondo" bind:value={newRouteKmRaw} class="input-narrow" disabled={addingRoute} />
            <button type="submit" class="btn-secondary" disabled={addingRoute || !newRouteName.trim() || !newRouteKmRaw}>
              {addingRoute ? "…" : "+ Agregar"}
            </button>
          </form>
        </div>

        <div class="panel">
          <div class="panel-header"><span class="panel-title">Nivel de tanque</span></div>
          <p class="panel-hint">
            Resetea el nivel de un vehículo sin borrar tanqueos ni viajes anteriores — útil si corriges su rendimiento (km/gal) y quieres que la autonomía se calcule de nuevo desde hoy.
          </p>

          {#if vehicles.length === 0}
            <p class="muted small">Agrega un vehículo primero.</p>
          {:else}
            {#if vehicles.length > 1}
              <div class="chip-grid chip-grid-sm">
                {#each vehicles as v (v.id)}
                  <button
                    type="button"
                    class="chip chip-sm"
                    class:active={selectedVehicleId === v.id}
                    onclick={() => { selectedVehicleId = v.id; }}
                  >{v.name}</button>
                {/each}
              </div>
            {/if}

            {#if resetLevelMsg}<div class="banner success small">{resetLevelMsg}</div>{/if}
            {#if resetLevelError}<div class="banner error small">{resetLevelError}</div>{/if}

            <form class="inline-form-3" onsubmit={handleResetFuelLevel}>
              <input type="text" inputmode="decimal" class="input-narrow" placeholder="Nivel actual (gal)" bind:value={resetLevelRaw} disabled={resettingLevel} />
              <input type="text" placeholder="Nota (opcional)" bind:value={resetLevelNote} disabled={resettingLevel} />
              <button type="submit" class="btn-secondary" disabled={resettingLevel || !selectedVehicleId || resetLevelRaw === ""}>
                {resettingLevel ? "…" : "Resetear nivel"}
              </button>
            </form>
          {/if}
        </div>

        <div class="panel">
          <button type="button" class="disclosure-toggle" onclick={() => { showPriceTables = !showPriceTables; }}>
            <span class="panel-title">Historial y comparación semanal</span>
            <span class="switch" class:on={showPriceTables}></span>
          </button>

          {#if showPriceTables}
            <div class="disclosure-body">
              {#if priceHistory.length > 0}
                <div class="subpanel">
                  <span class="subpanel-title">Historial de precios</span>
                  <div class="record-list">
                    {#each priceHistory as p (p.id)}
                      <div class="record-row">
                        <span class="record-date">{p.date}</span>
                        <span class="source-badge source-{p.source}">{p.source}</span>
                        <span class="record-gap"></span>
                        <span class="record-value">{formatCOP(p.price_per_gallon)}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if weeklyData.length > 0}
                <div class="subpanel">
                  <span class="subpanel-title">Comparación semanal</span>
                  <div class="record-list">
                    {#each weeklyData as w, i}
                      {@const prev = weeklyData[i + 1]}
                      <div class="record-row">
                        <span class="record-date">{w.week_start}</span>
                        <span class="record-count">{w.entry_count} registro{w.entry_count !== 1 ? "s" : ""}</span>
                        <span class="record-gap"></span>
                        <span class="record-value">
                          {formatCOP(w.avg_price)}
                          {#if prev}
                            {@const delta = w.avg_price - prev.avg_price}
                            <span class="delta" class:up={delta > 0} class:down={delta < 0}>
                              {delta > 0 ? "↑" : delta < 0 ? "↓" : "—"}
                            </span>
                          {/if}
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if priceHistory.length === 0 && weeklyData.length === 0}
                <p class="muted small">Todavía no hay suficientes datos.</p>
              {/if}
            </div>
          {/if}
        </div>

      {/if}

      <!-- ══════════════════════ VEHÍCULOS ══════════════════════ -->
      {#if activeTab === "vehiculos"}

        <div class="panel">
          <div class="panel-header"><span class="panel-title">Vehículos</span></div>

          {#if vehicles.length === 0}
            <p class="muted">Sin vehículos todavía.</p>
          {:else}
            <div class="item-list">
              {#each vehicles as v (v.id)}
                {#if editingVehicleId === v.id}
                  <div class="edit-row">
                    <input type="text" bind:value={editVehicleName} disabled={savingVehicle} placeholder="Nombre" />
                    <input type="text" inputmode="decimal" class="input-narrow" bind:value={editVehicleKmRaw} disabled={savingVehicle} placeholder="km/gal" />
                    <input type="text" inputmode="decimal" class="input-narrow" bind:value={editVehicleTankRaw} disabled={savingVehicle} placeholder="galones" />
                    <button class="icon-btn confirm" onclick={() => saveEditVehicle(v.id)} disabled={savingVehicle} title="Guardar">✓</button>
                    <button class="icon-btn" onclick={() => { editingVehicleId = null; }} disabled={savingVehicle} title="Cancelar">✕</button>
                  </div>
                {:else}
                  <div class="item-row">
                    <span class="item-name">{v.name}</span>
                    <span class="item-meta">{mPerLToKmPerGallon(v.efficiency_m_per_l).toFixed(1)} km/gal{#if v.tank_capacity_ml != null} · {mlToGallons(v.tank_capacity_ml).toFixed(1)} gal{/if}</span>
                    <div class="item-actions">
                      <button class="item-act" onclick={() => startEditVehicle(v)}>Editar</button>
                      <button
                        class="item-act danger"
                        onclick={() => deleteVehicle(v.id)}
                        disabled={deletingVehicleId === v.id}
                      >{deletingVehicleId === v.id ? "…" : "Eliminar"}</button>
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          {/if}

          <div class="form-section">
            <span class="form-section-label">Agregar vehículo</span>

            {#if vehicleFormError}<div class="banner error small">{vehicleFormError}</div>{/if}
            <form class="inline-form-3" onsubmit={addVehicle}>
              <input type="text" placeholder="Nombre (ej. Moto, Carro)" bind:value={newVehicleName} disabled={addingVehicle} />
              <input type="text" inputmode="decimal" class="input-narrow" placeholder="km/gal" bind:value={newVehicleKmRaw} disabled={addingVehicle} />
              <input type="text" inputmode="decimal" class="input-narrow" placeholder="galones" bind:value={newVehicleTankRaw} disabled={addingVehicle} />
              <button type="submit" class="btn-secondary" disabled={addingVehicle || !newVehicleName.trim() || !newVehicleKmRaw || !newVehicleTankRaw}>
                {addingVehicle ? "…" : "+ Agregar"}
              </button>
            </form>
            <p class="panel-hint">
              La capacidad del tanque (en galones, igual que el resto de la app) es obligatoria: además de mostrar el % de nivel y la autonomía en el Dashboard, es lo que permite detectar un rendimiento (km/gal) mal configurado — si un tanqueo deja el nivel calculado por encima de la capacidad real, la app te avisa.
            </p>
          </div>
        </div>

      {/if}

      <!-- ══════════════════════ PRESUPUESTOS ══════════════════════ -->
      {#if activeTab === "presupuestos"}

        <div class="panel">
          <div class="panel-header"><span class="panel-title">Presupuestos mensuales</span></div>

          {#if budgets.length === 0}
            <p class="muted">Sin categorías todavía.</p>
          {:else}
            <div class="item-list">
              {#each budgets as b (b.category.id)}
                <div class="budget-row" class:row-saved={savedBudgetCategory === b.category.id}>
                  <div class="budget-cat">
                    <span class="item-name">{b.category.name}</span>
                    {#if b.category.kind === "income"}
                      <button
                        class="pill-toggle"
                        class:on={b.category.is_fixed}
                        onclick={() => toggleFixed(b)}
                        disabled={togglingFixed === b.category.id}
                        title={b.category.is_fixed ? "Ingreso fijo — clic para marcar como variable" : "Ingreso variable — clic para marcar como fijo"}
                      >{b.category.is_fixed ? "Fijo" : "Variable"}</button>
                    {:else}
                      <span class="type-pill expense">Gasto</span>
                    {/if}
                  </div>

                  <div class="budget-route" style="--cs-padding: 0.2rem 0.5rem; font-size: 0.75rem;">
                    <CustomSelect
                      value={b.category.route_id}
                      options={[
                        { value: null, label: "Sin ruta" },
                        ...customRoutes.map(r => ({ value: r.id, label: r.name })),
                      ]}
                      onchange={(v) => saveRouteAssoc(b, v)}
                    />
                  </div>

                  <div class="budget-amount">
                    {#if editingBudget === b.category.id}
                      <div class="edit-row edit-row-inline">
                        <!-- svelte-ignore a11y_autofocus -->
                        <input type="text" inputmode="numeric" class="input-narrow"
                          value={editBudgetRaw ? new Intl.NumberFormat("es-CO").format(parseInt(editBudgetRaw, 10)) : ""}
                          oninput={handleBudgetInput} onkeydown={(e) => handleBudgetKeydown(e, b.category.id)}
                          disabled={savingBudget} autofocus />
                        <button class="icon-btn confirm" onclick={() => saveEditBudget(b.category.id)} disabled={savingBudget} title="Guardar">✓</button>
                        <button class="icon-btn" onclick={() => { editingBudget = null; }} disabled={savingBudget} title="Cancelar">✕</button>
                      </div>
                    {:else}
                      <button class="amount-btn" onclick={() => startEditBudget(b.category.id, b.monthly_cop)}>
                        {b.monthly_cop > 0 ? formatCOP(b.monthly_cop) : "—"}
                      </button>
                    {/if}
                  </div>

                  <div class="budget-row-actions">
                    <button
                      class="item-act danger"
                      onclick={() => deleteBudget(b.category.id)}
                      disabled={deletingBudget === b.category.id}
                    >{deletingBudget === b.category.id ? "…" : "Eliminar"}</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          {#if budgetFormError}<div class="banner error small">{budgetFormError}</div>{/if}
          <form class="inline-form-3" onsubmit={addBudget}>
            <input type="text" placeholder="Nombre de la categoría" bind:value={newBudgetName} disabled={addingBudget} />
            <div class="input-narrow" style="--cs-padding: 0.4rem 0.6rem;">
              <CustomSelect
                bind:value={newBudgetType}
                options={[
                  { value: "expense", label: "Gasto" },
                  { value: "income",  label: "Ingreso" },
                ]}
                disabled={addingBudget}
              />
            </div>
            {#if newBudgetType === "income"}
              <button
                type="button"
                class="pill-toggle"
                class:on={newBudgetIsFixed}
                onclick={() => newBudgetIsFixed = !newBudgetIsFixed}
                disabled={addingBudget}
              >{newBudgetIsFixed ? "Fijo" : "Variable"}</button>
            {/if}
            <button type="submit" class="btn-secondary" disabled={addingBudget || !newBudgetName.trim()}>
              {addingBudget ? "…" : "+ Agregar"}
            </button>
          </form>
        </div>

      {/if}

      <!-- ══════════════════════ SISTEMA ══════════════════════ -->
      {#if activeTab === "sistema"}

        <div class="panel">
          <div class="row-between">
            <div>
              <span class="row-label">Iniciar con el sistema</span>
              <span class="row-hint">Abrir FinCapX automáticamente al iniciar sesión</span>
            </div>
            {#if autostartLoading}
              <span class="muted">…</span>
            {:else}
              <button
                type="button"
                class="switch"
                class:on={autostartEnabled}
                onclick={toggleAutostart}
                aria-label="Autoarranque"
              ></button>
            {/if}
          </div>
          {#if autostartError}<div class="banner error small">{autostartError}</div>{/if}
        </div>

        <div class="panel">
          <div class="panel-header"><span class="panel-title">Base de datos local</span></div>
          {#if backupPath}<div class="banner success small">Backup guardado en: {backupPath}</div>{/if}
          {#if backupError}<div class="banner error small">{backupError}</div>{/if}
          <button type="button" class="btn-secondary" onclick={handleBackup} disabled={backupBusy}>
            {backupBusy ? "Exportando…" : "Exportar backup"}
          </button>
        </div>

      {/if}

      <!-- ══════════════════════ DATOS (peligro) ══════════════════════ -->
      {#if activeTab === "datos"}

        <div class="panel panel-danger">
          <div class="panel-header"><span class="panel-title danger-title">Restablecer datos de fábrica</span></div>
          {#if resetSuccess}<div class="banner success small">Datos eliminados. La app está lista para usar.</div>{/if}
          <p class="danger-hint">
            Elimina permanentemente todas las transacciones, objetivos, historial de gasolina, categorías, rutas y vehículos.
            La app quedará vacía, lista para configurar desde cero. Esta acción no se puede deshacer.
          </p>
          <button type="button" class="btn-danger" onclick={openReset}>Restablecer datos de fábrica</button>
        </div>

      {/if}

    {/if}
    </ScrollArea>
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
    gap: 0.75rem;
    box-sizing: border-box;
  }

  .config-header { flex-shrink: 0; }

  h1 { font-size: 1.1rem; font-weight: 700; color: var(--text-primary); letter-spacing: -0.02em; }

  /* ── Pestañas ── */
  .tabs {
    flex-shrink: 0;
    display: flex;
    gap: 1.75rem;
    border-bottom: 1px solid var(--border);
  }

  .tab {
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
  .tab:hover { color: var(--text-secondary); }
  .tab.active { color: var(--accent); border-color: var(--accent); }
  .tab-danger.active { color: var(--danger); border-color: var(--danger); }

  /* ── Cuerpo de la pestaña — .tab-body debe ser flex para que su hijo
     (el ScrollArea) reciba una altura acotada real; si no, el ScrollArea
     crece a su contenido y overflow:hidden del padre recorta el exceso
     en vez de mostrar scrollbar. ── */
  .tab-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }

  :global(.tab-scroll) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .muted { color: var(--text-muted); font-size: 0.85rem; }
  .muted.small, p.muted.small { font-size: 0.78rem; }

  /* ── Panel base ── */
  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .panel-danger { border-color: var(--danger); }

  .panel-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .panel-title {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-muted);
  }

  .danger-title { color: var(--danger); }

  .panel-title-hint {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .panel-hint { font-size: 0.78rem; color: var(--text-muted); line-height: 1.5; margin: 0; }

  /* Separa visualmente un formulario de "agregar" de la lista de arriba —
     sin esto, filas de datos e inputs de un formulario quedan pegados y se
     confunden a simple vista. */
  .form-section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border-top: 1px solid var(--border);
    padding-top: 0.85rem;
  }

  .form-section-label {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  /* ── Precio hero ── */
  .price-hero { display: flex; align-items: baseline; gap: 0.4rem; }
  .price-value {
    font-size: 2.5rem;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }
  .price-value.muted { font-size: 1.05rem; font-weight: 500; }
  .price-unit { font-size: 0.85rem; color: var(--text-muted); }

  .price-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .source-badge {
    font-size: 0.6rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius);
    background: transparent;
  }
  .source-manual   { border: 1px solid var(--accent);  color: var(--accent);  }
  .source-scraping { border: 1px solid var(--success); color: var(--success); }

  /* ── Chips (selección de vehículo) — mismo patrón que Registrar: grid
     reparte 1fr por columna, todas las celdas quedan del mismo ancho sin
     importar el largo del texto. ── */
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
    overflow-wrap: break-word;
  }
  .chip-sm { min-height: 2rem; padding: 0.4rem 0.5rem; font-size: 0.74rem; }
  .chip:hover:not(.active) { color: var(--text-primary); border-color: var(--text-secondary); }
  .chip.active { background: var(--accent); color: var(--bg-base); border-color: var(--accent); font-weight: 700; }

  /* ── Listas de ítems (rutas, vehículos) ── */
  .item-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .item-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
    min-height: 38px;
    transition: background 0.1s;
  }
  .item-list .item-row:last-child { border-bottom: none; }
  .item-row:hover { background: var(--bg-elevated); }

  .item-name { flex: 1; min-width: 0; color: var(--text-primary); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item-meta { font-size: 0.75rem; color: var(--text-muted); font-family: var(--font-mono); white-space: nowrap; }
  .item-value { color: var(--accent); font-weight: 700; font-family: var(--font-mono); font-size: 0.85rem; white-space: nowrap; }

  /* Acciones de fila — ocultas hasta hover, igual que .tx-actions en
     Historial: reducen ruido visual cuando no se están usando. */
  .item-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.15rem;
    opacity: 0;
    transition: opacity 0.15s;
    min-width: 70px;
    justify-content: flex-end;
  }
  .item-row:hover .item-actions,
  .budget-row:hover .budget-row-actions { opacity: 1; }

  .item-act {
    font-size: 0.66rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius);
    transition: color 0.15s, background 0.15s;
    white-space: nowrap;
  }
  .item-act:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-surface); }
  .item-act.danger:hover:not(:disabled) { color: var(--danger); }
  .item-act:disabled { opacity: 0.4; cursor: not-allowed; }

  .edit-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }
  .edit-row-inline { padding: 0; border-bottom: none; }

  /* ── Botones de icono — solo para confirmar/cancelar edición inline
     (estado activo, deben verse siempre, a diferencia de las acciones
     de fila normales que se revelan al hover). ── */
  .icon-btn {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    color: var(--text-muted);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .icon-btn:hover:not(:disabled) { color: var(--text-primary); border-color: var(--text-secondary); }
  .icon-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .icon-btn.danger:hover:not(:disabled) { color: var(--danger); border-color: var(--danger); }
  .icon-btn.confirm { color: var(--success); }
  .icon-btn.confirm:hover:not(:disabled) { border-color: var(--success); }

  /* ── Formularios inline ── */
  .inline-form { display: flex; gap: 0.5rem; }
  .inline-form-3 { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; }

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
    flex: 1;
    min-width: 140px;
  }
  input[inputmode="numeric"],
  input[inputmode="decimal"] { font-family: var(--font-mono); }
  input:focus { border-color: var(--accent); }

  .input-narrow { flex: 0 0 auto; width: 110px; min-width: 0; }

  /* ── Botones ── */
  .btn-primary {
    padding: 0.5rem 1.1rem;
    background: var(--accent);
    color: var(--bg-base);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.78rem;
    font-weight: 700;
    border-radius: var(--radius);
    white-space: nowrap;
    transition: background 0.15s, opacity 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-secondary {
    padding: 0.5rem 1.1rem;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.78rem;
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    white-space: nowrap;
    transition: border-color 0.15s, color 0.15s;
  }
  .btn-secondary:hover:not(:disabled) { color: var(--text-primary); border-color: var(--text-secondary); }
  .btn-secondary:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-danger {
    align-self: flex-start;
    padding: 0.5rem 1.1rem;
    background: transparent;
    color: var(--danger);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.78rem;
    font-weight: 700;
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    transition: background 0.15s, color 0.15s;
  }
  .btn-danger:hover { background: var(--danger); color: var(--bg-base); }

  /* ── Interruptor (switch) — mismas medidas que Registrar. ── */
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
    top: 2px; left: 2px;
    width: 11px; height: 11px;
    background: var(--text-muted);
    transition: transform 0.15s, background 0.15s;
  }
  .switch.on { background: color-mix(in srgb, var(--accent) 22%, var(--bg-elevated)); border-color: var(--accent); }
  .switch.on::after { transform: translateX(13px); background: var(--accent); }

  /* ── Disclosure (historial/semanal) ── */
  .disclosure-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    text-align: left;
  }

  .disclosure-body {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .subpanel { display: flex; flex-direction: column; gap: 0.4rem; }
  .subpanel-title { font-size: 0.68rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }

  /* ── Listas de registros (historial de precios, comparación semanal) —
     filas planas en vez de tabla HTML, mismo idioma que .tx-row/.ctx-tx-item. ── */
  .record-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .record-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.8rem;
    transition: background 0.1s;
  }
  .record-list .record-row:last-child { border-bottom: none; }
  .record-row:hover { background: var(--bg-elevated); }
  .record-date { font-family: var(--font-mono); color: var(--text-secondary); white-space: nowrap; }
  .record-count { font-family: var(--font-mono); font-size: 0.7rem; color: var(--text-muted); white-space: nowrap; }
  .record-gap { flex: 1; min-width: 0.5rem; }
  .record-value {
    font-family: var(--font-mono);
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .delta { font-size: 0.7rem; margin-left: 0.2rem; }
  .delta.up   { color: var(--danger); }
  .delta.down { color: var(--success); }

  /* ── Presupuestos — mismo idioma de fila que .item-row: acciones de
     borrar ocultas hasta hover. ── */
  .budget-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.7rem;
    border-bottom: 1px solid var(--border);
    min-height: 38px;
    transition: background 0.1s;
  }
  .item-list .budget-row:last-child { border-bottom: none; }
  .budget-row:hover { background: var(--bg-elevated); }
  .budget-row.row-saved { background: color-mix(in srgb, var(--success) 12%, transparent); }

  .budget-cat { flex: 1; min-width: 0; display: flex; align-items: center; gap: 0.4rem; }
  .budget-route { flex: 0 0 130px; min-width: 0; }
  .budget-amount { flex: 0 0 auto; min-width: 90px; display: flex; justify-content: flex-end; }
  .budget-row-actions {
    flex: 0 0 auto;
    min-width: 64px;
    display: flex;
    justify-content: flex-end;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .amount-btn {
    font-size: 0.82rem;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius);
    transition: background 0.15s, color 0.15s;
  }
  .amount-btn:hover { background: var(--bg-elevated); color: var(--accent); }

  .type-pill {
    font-size: 0.6rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .type-pill.expense { border: 1px solid var(--danger); color: var(--danger); }

  .pill-toggle {
    font-size: 0.6rem;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius);
    white-space: nowrap;
    flex-shrink: 0;
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border);
    transition: color 0.15s, border-color 0.15s;
  }
  .pill-toggle.on { color: var(--accent); border-color: var(--accent); }
  .pill-toggle:disabled { opacity: 0.4; cursor: not-allowed; }

  /* ── Sistema ── */
  .row-between { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  .row-label { display: block; font-size: 0.875rem; font-weight: 500; color: var(--text-primary); }
  .row-hint  { display: block; font-size: 0.75rem; color: var(--text-muted); margin-top: 0.1rem; }

  /* ── Datos (peligro) ── */
  .danger-hint { font-size: 0.8rem; color: var(--text-muted); line-height: 1.5; }

  /* ── Banners ── */
  .banner { border-radius: var(--radius); padding: 0.55rem 0.9rem; font-size: 0.82rem; }
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
  .banner.small { font-size: 0.78rem; padding: 0.4rem 0.75rem; }

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
  .modal-body { font-size: 0.875rem; color: var(--text-secondary); line-height: 1.5; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; }

  .btn-cancel {
    padding: 0.45rem 1rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.78rem;
    font-weight: 600;
  }
  .btn-cancel:hover:not(:disabled) { color: var(--text-primary); border-color: var(--text-secondary); }
  .btn-cancel:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn-danger-confirm {
    padding: 0.45rem 1rem;
    background: var(--danger);
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    color: var(--bg-base);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.78rem;
    font-weight: 700;
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
    box-sizing: border-box;
    transition: border-color 0.15s;
  }
  .reset-input:focus { border-color: var(--danger); }
</style>
