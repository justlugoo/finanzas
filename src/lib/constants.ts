export const MESES = [
  "enero","febrero","marzo","abril","mayo","junio",
  "julio","agosto","septiembre","octubre","noviembre","diciembre",
];

export const MESES_CORTO = [
  "ene","feb","mar","abr","may","jun","jul","ago","sep","oct","nov","dic",
];

export const DIAS_SEMANA = ["dom","lun","mar","mié","jue","vie","sáb"];

export const WIDGET_RECENT_SIZE   = 10;
export const HISTORY_PAGE_SIZE    = 20;
export const DASHBOARD_RECENT_SIZE = 6;

// Conversión de unidades para la capa de presentación — el backend v2 solo
// trabaja en mililitros/metros enteros (docs/schema-v2.md, principio 1).
export const ML_PER_GALLON = 3785.411784;
export const mlToGallons = (ml: number) => ml / ML_PER_GALLON;
export const metersToKm  = (m: number) => m / 1000;
export const mPerLToKmPerGallon = (mPerL: number) => (mPerL * ML_PER_GALLON) / 1_000_000;
export const kmToM = (km: number) => Math.round(km * 1000);
