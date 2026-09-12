<script lang="ts">
  // Mensaje del tour anclado sobre un elemento puntual (un input, botón o
  // control específico) — a diferencia del viejo TourHint (un solo bloque
  // de texto por pantalla), acá cada input/botón relevante tiene el suyo,
  // todos visibles a la vez, sin bloquear nada ni requerir interacción.
  // Se coloca dentro de un contenedor con la clase `tour-field` (o
  // cualquier ancestro con position:relative) para anclarse correctamente.
  let { text }: { text: string } = $props();
</script>

<div class="tour-point">{text}</div>

<style>
  .tour-point {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    width: max-content;
    max-width: 140px;
    background: color-mix(in srgb, var(--accent) 16%, var(--bg-elevated));
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    padding: 0.3rem 0.5rem;
    font-size: 0.66rem;
    font-weight: 500;
    line-height: 1.3;
    color: var(--text-primary);
    text-align: center;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    pointer-events: none;
  }
  .tour-point::after {
    content: "";
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 5px solid transparent;
    border-top-color: var(--accent);
  }
</style>
