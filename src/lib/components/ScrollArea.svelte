<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    orientation = "vertical",
    maxHeight,
    class: className = "",
    fadeEdges = false,
    scrollbar = "thin",
    children,
  }: {
    orientation?: "vertical" | "horizontal" | "both";
    maxHeight?: string;
    class?: string;
    fadeEdges?: boolean;
    scrollbar?: "auto" | "thin" | "none";
    children?: Snippet;
  } = $props();

  let overflowX = $derived(orientation === "horizontal" || orientation === "both" ? "auto" : "hidden");
  let overflowY = $derived(orientation === "vertical"   || orientation === "both" ? "auto" : "hidden");
</script>

<div
  class="scroll-area {className}"
  class:fade-edges={fadeEdges}
  class:no-scrollbar={scrollbar === "none"}
  style:max-height={maxHeight}
  style:overflow-x={overflowX}
  style:overflow-y={overflowY}
  style:scrollbar-width={scrollbar === "none" ? "none" : scrollbar === "thin" ? "thin" : "auto"}
>
  {@render children?.()}
</div>

<style>
  .scroll-area {
    scrollbar-color: var(--border) transparent;
  }
  .scroll-area::-webkit-scrollbar { width: 2px; height: 2px; }
  .scroll-area::-webkit-scrollbar-track { background: transparent; }
  .scroll-area::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

  .no-scrollbar::-webkit-scrollbar { display: none; }

  .fade-edges {
    -webkit-mask-image: linear-gradient(
      to bottom,
      transparent 0%,
      black 8%,
      black 92%,
      transparent 100%
    );
    mask-image: linear-gradient(
      to bottom,
      transparent 0%,
      black 8%,
      black 92%,
      transparent 100%
    );
  }
</style>
