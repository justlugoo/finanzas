import { goto } from "$app/navigation";

// El tour cubre solo lo esencial para llevar el control de finanzas —
// presupuestos, registrar movimientos, historial, metas y el resumen. Nada
// de vehículos/gasolina (no todos tienen vehículo), ni de sistema/datos
// (autoarranque, backup, borrar todo no son parte de la misión de la app).
//
// Cada paso puede tener más de un punto (`pointCount`), pero solo se
// muestra UNO a la vez — nunca todos los de una sección juntos — para que
// no sea una pared de mensajes flotantes. El texto de cada punto vive
// junto al elemento real al que se refiere (TourPoint, en cada página),
// gateado con `isActivePoint(stepId, index)`.
export type TourStepId = "presupuestos" | "registros" | "historial" | "metas" | "resumen";

export interface TourStep {
  id: TourStepId;
  route: string;
  title: string;
  pointCount: number;
}

export const TOUR_STEPS: TourStep[] = [
  { id: "presupuestos", route: "/config?tab=presupuestos", title: "Presupuestos", pointCount: 1 },
  { id: "registros",    route: "/registrar",                title: "Registros",    pointCount: 1 },
  { id: "historial",    route: "/historial",                title: "Historial",    pointCount: 2 },
  { id: "metas",        route: "/metas",                     title: "Metas",        pointCount: 1 },
  { id: "resumen",      route: "/",                          title: "Resumen",      pointCount: 2 },
];

export const tour = $state<{ active: boolean; stepIndex: number; pointIndex: number }>({
  active: false, stepIndex: 0, pointIndex: 0,
});

export function currentStep(): TourStep { return TOUR_STEPS[tour.stepIndex]; }

export function isActiveStep(stepId: TourStepId): boolean {
  return tour.active && currentStep()?.id === stepId;
}

/** Punto puntual dentro del paso activo — solo uno a la vez está activo. */
export function isActivePoint(stepId: TourStepId, index: number): boolean {
  return isActiveStep(stepId) && tour.pointIndex === index;
}

export function isFirstPoint(): boolean { return tour.stepIndex === 0 && tour.pointIndex === 0; }
export function isLastPoint(): boolean {
  return tour.stepIndex === TOUR_STEPS.length - 1 && tour.pointIndex === currentStep().pointCount - 1;
}

async function goToStep(index: number) {
  const step = TOUR_STEPS[index];
  if (!step) return;
  tour.stepIndex = index;
  tour.pointIndex = 0;
  await goto(step.route);
}

export async function startTour() { tour.active = true; await goToStep(0); }

export async function nextStep() {
  const step = currentStep();
  if (tour.pointIndex < step.pointCount - 1) { tour.pointIndex++; return; }
  if (tour.stepIndex < TOUR_STEPS.length - 1) await goToStep(tour.stepIndex + 1);
  else endTour();
}

export async function prevStep() {
  if (tour.pointIndex > 0) { tour.pointIndex--; return; }
  if (tour.stepIndex > 0) {
    const prevIndex = tour.stepIndex - 1;
    const prev = TOUR_STEPS[prevIndex];
    tour.stepIndex = prevIndex;
    tour.pointIndex = prev.pointCount - 1;
    await goto(prev.route);
  }
}

export function endTour() { tour.active = false; }
