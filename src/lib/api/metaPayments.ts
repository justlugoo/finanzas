import { invoke } from "@tauri-apps/api/core";
import type { Entry, MetaPaymentInput } from "$lib/types";

// Un solo camino de abono para las tres clases de meta ("loan:<id>" |
// "goal:<id>") — el frontend deja de decidir si eso es un ingreso o un
// gasto, sección 12 (paso 6) de schema-v2.md.
export const addPayment = (input: MetaPaymentInput) =>
  invoke<Entry>("meta_add_payment", { input });
