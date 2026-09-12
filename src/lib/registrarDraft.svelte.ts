// Borrador de Registrar — vive en memoria mientras la app esté abierta, no
// en el componente de la página. Si el componente se desmonta (el usuario
// se va a otra sección) y se vuelve a montar (vuelve a Registrar), un
// $state local normal se reinicia solo — este objeto, al vivir en un
// módulo aparte, no. Se resetea solo tras guardar con éxito o con el botón
// "Limpiar formulario".
function todayISO(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export const registrarDraft = $state({
  kind: "ingreso" as "ingreso" | "gasto" | "tanqueo",
  categoryId: "",
  amountRaw: "",
  date: todayISO(),
  note: "",
  extraordinary: false,
  gasKmRaw: "",
  vehicleId: null as string | null,
  viajeOpen: false,
  fillupVehicleId: null as string | null,
  fillupAmountRaw: "",
  fillupPriceRaw: "",
  fillupDate: todayISO(),
  fillupNote: "",
});

/** Vacía el contenido que se estaba redactando — no toca `vehicleId` ni
 * `fillupVehicleId` (cuál es "mi vehículo" es una preferencia, no parte del
 * borrador que se está escribiendo). */
export function clearRegistrarDraft() {
  registrarDraft.kind          = "ingreso";
  registrarDraft.categoryId    = "";
  registrarDraft.amountRaw     = "";
  registrarDraft.date          = todayISO();
  registrarDraft.note          = "";
  registrarDraft.extraordinary = false;
  registrarDraft.gasKmRaw      = "";
  registrarDraft.viajeOpen     = false;
  registrarDraft.fillupAmountRaw = "";
  registrarDraft.fillupPriceRaw  = "";
  registrarDraft.fillupDate      = todayISO();
  registrarDraft.fillupNote      = "";
}
