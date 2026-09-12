<script lang="ts">
  import { onMount } from "svelte";

  // Se ancla con coordenadas calculadas en JS y `position:fixed` — igual
  // que el menú de CustomSelect o el hint de "Extraordinario" en Registrar
  // — para escapar de cualquier ancestro con overflow:hidden o scroll (el
  // ScrollArea de cada pantalla) y quedar siempre por encima de todo.
  //
  // `side`:
  // - "top" (por defecto): el cuadro aparece arriba del elemento, flecha
  //   apuntando hacia abajo; si no cabe (elemento pegado al techo) se
  //   voltea para aparecer abajo, con la flecha apuntando hacia arriba.
  // - "right": el cuadro aparece a la derecha del elemento, flecha
  //   apuntando hacia la izquierda — para señalar un bloque ancho (un
  //   formulario completo) sin taparlo por encima o por debajo.
  //
  // En ambos casos el cuadro se clampea para no salirse de la ventana,
  // pero la flecha se desplaza dentro del cuadro para seguir señalando el
  // centro real del elemento anclado. Nunca se superpone con la barra de
  // navegación del tour (fixed, abajo) — se detecta vía [data-tour-navbar].
  let { text, side = "top" }: { text: string; side?: "top" | "right" } = $props();

  let rootEl: HTMLElement | undefined = $state(undefined);
  // Posición inicial explícita fuera de la ventana — un `position:fixed`
  // sin top/left/bottom/right todavía asignados puede pintarse por un
  // instante en su "posición estática" (como si estuviera en el flujo
  // normal), lo que alcanzó a ensanchar el layout de la página. Con
  // coordenadas ya fijadas desde el primer render, nunca ocupa espacio.
  let style    = $state("left: -9999px; top: -9999px; visibility: hidden;");
  let flipped  = $state(false);

  function clamp(v: number, min: number, max: number): number {
    return Math.min(Math.max(v, min), Math.max(min, max));
  }

  function reposition() {
    const anchor = rootEl?.parentElement;
    if (!anchor || !rootEl) return;
    const rect = anchor.getBoundingClientRect();

    const bubbleWidth  = rootEl.offsetWidth  || 200;
    const bubbleHeight = rootEl.offsetHeight || 44;
    const margin = 12;
    const gap    = 8;

    if (side === "right") {
      flipped = false;
      // A la derecha del elemento anclado, centrado verticalmente respecto
      // a ESE elemento (el formulario/bloque completo, no la ventana).
      const centerY = rect.top + rect.height / 2;
      const top = clamp(centerY - bubbleHeight / 2, margin, window.innerHeight - bubbleHeight - margin);
      const left = clamp(rect.right + gap, margin, window.innerWidth - bubbleWidth - margin);
      style = `left: ${left}px; top: ${top}px; --arrow-top: ${bubbleHeight / 2}px;`;
      return;
    }

    const centerX = rect.left + rect.width / 2;
    const left = clamp(centerX - bubbleWidth / 2, margin, window.innerWidth - bubbleWidth - margin);
    const arrowLeft = clamp(centerX - left, 16, bubbleWidth - 16);

    // Si no cabe arriba del elemento (pegado al techo de la ventana), se
    // voltea hacia abajo en vez de taparlo.
    flipped = rect.top < bubbleHeight + gap + 16;

    let posStyle: string;
    if (flipped) {
      posStyle = `top: ${rect.bottom + gap}px;`;
    } else {
      let bottom = window.innerHeight - rect.top + gap;
      const navbar = document.querySelector("[data-tour-navbar]");
      if (navbar) {
        const navTop = navbar.getBoundingClientRect().top;
        bottom = Math.max(bottom, window.innerHeight - navTop + 10);
      }
      posStyle = `bottom: ${bottom}px;`;
    }

    style = `left: ${left}px; ${posStyle} --arrow-left: ${arrowLeft}px;`;
  }

  onMount(() => {
    reposition();
    // Doble rAF: da tiempo a que el layout de la página recién navegada
    // (fuentes, ScrollArea, etc.) se asiente antes de medir de nuevo.
    requestAnimationFrame(() => requestAnimationFrame(reposition));

    const ro = new ResizeObserver(reposition);
    if (rootEl?.parentElement) ro.observe(rootEl.parentElement);

    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", reposition);
      window.removeEventListener("scroll", reposition, true);
    };
  });
</script>

<div bind:this={rootEl} class="tour-point" class:flipped class:side-right={side === "right"} style={style}>{text}</div>

<style>
  .tour-point {
    position: fixed;
    z-index: 9999;
    width: max-content;
    max-width: 230px;
    background: var(--accent);
    border: 2px solid var(--accent);
    border-radius: var(--radius);
    padding: 0.55rem 0.8rem;
    font-size: 0.82rem;
    font-weight: 600;
    line-height: 1.4;
    color: var(--bg-base);
    text-align: center;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
    pointer-events: none;
  }
  /* Por defecto: cuadro arriba del elemento, flecha apuntando hacia abajo. */
  .tour-point::after {
    content: "";
    position: absolute;
    top: 100%;
    left: var(--arrow-left, 50%);
    transform: translateX(-50%);
    border: 6px solid transparent;
    border-top-color: var(--accent);
  }
  /* Volteado: cuadro abajo del elemento, flecha apuntando hacia arriba. */
  .tour-point.flipped::after {
    top: auto;
    bottom: 100%;
    border-top-color: transparent;
    border-bottom-color: var(--accent);
  }
  /* A la derecha del elemento: flecha apuntando hacia la izquierda. */
  .tour-point.side-right::after {
    top: var(--arrow-top, 50%);
    bottom: auto;
    left: auto;
    right: 100%;
    transform: translateY(-50%);
    border-top-color: transparent;
    border-right-color: var(--accent);
  }
</style>
