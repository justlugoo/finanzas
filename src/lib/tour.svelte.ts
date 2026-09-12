import { goto } from "$app/navigation";

// Cada paso es una sección real de la app. El texto de cada paso NO vive acá
// — vive junto al elemento al que se refiere (un <TourPoint> anclado sobre
// el input/botón puntual en la propia página), así que este módulo solo
// necesita saber a qué ruta ir y qué título mostrar en la barra de progreso.
export type TourStepId =
  | "presupuestos"
  | "vehiculos"
  | "sistema"
  | "datos"
  | "registros"
  | "historial"
  | "metas"
  | "resumen";

export interface TourStep {
  id: TourStepId;
  route: string;
  title: string;
}

export const TOUR_STEPS: TourStep[] = [
  { id: "presupuestos", route: "/config?tab=presupuestos", title: "Presupuestos" },
  { id: "vehiculos",    route: "/config?tab=vehiculos",    title: "Vehículos y gasolina" },
  { id: "sistema",      route: "/config?tab=sistema",      title: "Sistema" },
  { id: "datos",        route: "/config?tab=datos",        title: "Datos" },
  { id: "registros",    route: "/registrar",               title: "Registros" },
  { id: "historial",    route: "/historial",               title: "Historial" },
  { id: "metas",        route: "/metas",                   title: "Metas" },
  { id: "resumen",      route: "/",                         title: "Resumen" },
];

export const tour = $state<{ active: boolean; stepIndex: number }>({ active: false, stepIndex: 0 });

export function currentStep(): TourStep { return TOUR_STEPS[tour.stepIndex]; }
export function isActiveStep(stepId: TourStepId): boolean { return tour.active && currentStep()?.id === stepId; }

async function goToStep(index: number) {
  const step = TOUR_STEPS[index];
  if (!step) return;
  tour.stepIndex = index;
  await goto(step.route);
}

export async function startTour() { tour.active = true; await goToStep(0); }
export async function nextStep() {
  if (tour.stepIndex < TOUR_STEPS.length - 1) await goToStep(tour.stepIndex + 1);
  else endTour();
}
export async function prevStep() { if (tour.stepIndex > 0) await goToStep(tour.stepIndex - 1); }
export function endTour() { tour.active = false; }
