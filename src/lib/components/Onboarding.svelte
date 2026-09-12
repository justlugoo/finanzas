<script lang="ts">
  // Confirmación inicial (modal bloqueante, es una decisión única sin campo
  // al que anclarse) → barra de navegación mínima durante el tour. Es un
  // recorrido, no una tarea: nunca bloquea "Siguiente" ni exige crear nada
  // — el mensaje de cada paso vive anclado sobre cada input/botón real
  // (TourPoint, en cada página), no acá.
  import { tour, TOUR_STEPS, startTour, nextStep, prevStep, endTour, isFirstPoint, isLastPoint } from "$lib/tour.svelte";

  let {
    firstInstall,
    onDismiss,
  }: {
    firstInstall: boolean;
    onDismiss: () => void;
  } = $props();

  let confirmed = $state(false);

  async function answerYes() {
    confirmed = true;
    await startTour();
  }

  function answerNo() {
    onDismiss();
  }

  async function handleNext() {
    if (isLastPoint()) {
      endTour();
      onDismiss();
      return;
    }
    await nextStep();
  }

  function handleSkip() {
    endTour();
    onDismiss();
  }
</script>

{#if !confirmed}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={answerNo}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <h2>{firstInstall ? "Bienvenido a FinCapX" : "FinCapX se actualizó"}</h2>
    <p class="body">¿Quieres un recorrido guiado por la app?</p>
    <div class="modal-actions">
      <button class="btn-secondary" onclick={answerNo}>No, gracias</button>
      <button class="btn-primary" onclick={answerYes}>Sí, vamos</button>
    </div>
  </div>
{:else if tour.active}
  {@const step = TOUR_STEPS[tour.stepIndex]}
  <div class="navbar" data-tour-navbar>
    <div class="dots">
      {#each TOUR_STEPS as _, i}
        <span class="dot" class:active={i === tour.stepIndex}></span>
      {/each}
    </div>
    <span class="step-title">{step.title}</span>
    <span class="spacer"></span>
    <button class="skip-link" onclick={handleSkip}>Omitir tour</button>
    {#if !isFirstPoint()}
      <button class="btn-secondary" onclick={prevStep}>Atrás</button>
    {/if}
    <button class="btn-primary" onclick={handleNext}>
      {isLastPoint() ? "Terminar" : "Siguiente"}
    </button>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 100;
  }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 1.5rem; z-index: 101;
    width: min(380px, 92vw);
    display: flex; flex-direction: column; gap: 0.75rem;
  }

  h2 { font-size: 1rem; font-weight: 700; color: var(--text-primary); }
  .body { font-size: 0.85rem; color: var(--text-secondary); line-height: 1.55; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.25rem; }

  .btn-primary {
    padding: 0.45rem 1rem; background: var(--accent); color: var(--bg-base);
    font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em;
    font-size: 0.78rem; font-weight: 700; border-radius: var(--radius);
    transition: background 0.15s;
  }
  .btn-primary:hover { background: var(--accent-hover); }

  .btn-secondary {
    padding: 0.45rem 1rem; background: transparent; color: var(--text-secondary);
    font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em;
    font-size: 0.78rem; font-weight: 600; border-radius: var(--radius);
    border: 1px solid var(--border); transition: border-color 0.15s, color 0.15s;
  }
  .btn-secondary:hover { border-color: var(--text-secondary); color: var(--text-primary); }

  /* ── Barra de navegación del tour — sin texto de instrucción propio; el
     mensaje real vive anclado sobre cada input/botón (TourPoint). ── */
  .navbar {
    position: fixed;
    left: 50%;
    bottom: 1rem;
    transform: translateX(-50%);
    z-index: 90;
    max-width: calc(100vw - 2rem);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5rem 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    box-shadow: 0 2px 12px rgba(0,0,0,0.4);
  }

  .dots { display: flex; gap: 0.3rem; flex-shrink: 0; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--border); }
  .dot.active { background: var(--accent); }

  .step-title {
    font-size: 0.72rem; font-family: var(--font-mono); text-transform: uppercase;
    letter-spacing: 0.04em; color: var(--text-secondary); white-space: nowrap;
  }

  .spacer { flex: 1; min-width: 0.5rem; }

  .skip-link {
    font-size: 0.72rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    white-space: nowrap;
    transition: color 0.15s;
  }
  .skip-link:hover { color: var(--text-secondary); }
</style>
