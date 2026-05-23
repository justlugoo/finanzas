<script lang="ts">
  import ScrollArea from "$lib/components/ScrollArea.svelte";

  interface Option {
    value: any;
    label: string;
  }
  interface Group {
    label: string;
    options: Option[];
  }

  let {
    value = $bindable(),
    options = [] as Option[],
    groups = [] as Group[],
    placeholder = "Selecciona…",
    disabled = false,
    onchange,
  }: {
    value?: any;
    options?: Option[];
    groups?: Group[];
    placeholder?: string;
    disabled?: boolean;
    onchange?: (v: any) => void;
  } = $props();

  let open = $state(false);
  let wrapEl = $state<HTMLDivElement | undefined>(undefined);
  let triggerEl = $state<HTMLButtonElement | undefined>(undefined);
  let menuStyle = $state("");

  let allOptions = $derived([
    ...options,
    ...groups.flatMap((g: Group) => g.options),
  ]);

  let selectedLabel = $derived(
    allOptions.find((o: Option) => o.value === value)?.label ?? placeholder
  );

  let widestLabel = $derived(
    [placeholder, ...allOptions.map((o: Option) => String(o.label))]
      .reduce((a, b) => b.length > a.length ? b : a, "")
  );

  function select(v: any) {
    value = v;
    open = false;
    onchange?.(v);
  }

  function openMenu() {
    if (!open && triggerEl) {
      const rect = triggerEl.getBoundingClientRect();
      menuStyle = `top:${rect.bottom + 4}px;left:${rect.left}px;min-width:${rect.width}px`;
    }
    open = !open;
  }

  $effect(() => {
    if (!open) return;
    function onWindowClick(e: MouseEvent) {
      if (wrapEl && !wrapEl.contains(e.target as Node)) open = false;
    }
    function onScroll(e: Event) {
      if (wrapEl?.contains(e.target as Node)) return;
      open = false;
    }
    const id = setTimeout(() => {
      window.addEventListener("click", onWindowClick);
      window.addEventListener("scroll", onScroll, true);
    }, 0);
    return () => {
      clearTimeout(id);
      window.removeEventListener("click", onWindowClick);
      window.removeEventListener("scroll", onScroll, true);
    };
  });
</script>

<div class="cs-wrap" bind:this={wrapEl}>
  <div class="cs-sizer" aria-hidden="true">
    <span class="cs-sizer-lbl">{widestLabel}</span>
    <span class="cs-arrow">▾</span>
  </div>
  <button
    type="button"
    class="cs-trigger"
    bind:this={triggerEl}
    onclick={openMenu}
    {disabled}
  >
    <span class="cs-label">{selectedLabel}</span>
    <span class="cs-arrow" class:open>▾</span>
  </button>
  {#if open}
    <div class="cs-menu" style={menuStyle}>
      <ScrollArea maxHeight="260px" scrollbar="none">
        {#each options as opt (String(opt.value))}
          <button
            type="button"
            class="cs-opt"
            class:selected={opt.value === value}
            onclick={() => select(opt.value)}
          >{opt.label}</button>
        {/each}
        {#each groups as group, gi}
          <div class="cs-group-label" class:first={gi === 0}>{group.label}</div>
          {#each group.options as opt (String(opt.value))}
            <button
              type="button"
              class="cs-opt cs-opt-in"
              class:selected={opt.value === value}
              onclick={() => select(opt.value)}
            >{opt.label}</button>
          {/each}
        {/each}
      </ScrollArea>
    </div>
  {/if}
</div>

<style>
  .cs-wrap {
    position: relative;
    display: block;
  }

  .cs-sizer {
    visibility: hidden;
    pointer-events: none;
    padding: var(--cs-padding, 0.55rem 0.75rem);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    white-space: nowrap;
  }
  .cs-sizer-lbl { flex: 1; }

  .cs-trigger {
    position: absolute;
    inset: 0;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font: inherit;
    padding: var(--cs-padding, 0.55rem 0.75rem);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    transition: border-color 0.15s;
    text-align: left;
  }
  .cs-trigger:hover:not(:disabled) { border-color: var(--accent); }
  .cs-trigger:focus-visible { border-color: var(--accent); outline: none; }
  .cs-trigger:disabled { opacity: 0.45; cursor: not-allowed; }

  .cs-label { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .cs-arrow {
    font-size: 0.65rem;
    color: var(--text-muted);
    transition: transform 0.15s;
    display: inline-block;
    flex-shrink: 0;
  }
  .cs-arrow.open { transform: rotate(180deg); }

  .cs-menu {
    position: fixed;
    background: #14141f;
    border: 1px solid var(--border);
    border-radius: 8px;
    z-index: 1000;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.55);
  }

  .cs-opt {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.5rem 1rem 0.5rem 0.75rem;
    font: inherit;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
  }
  .cs-opt:hover {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
  }
  .cs-opt.selected { color: var(--accent); font-weight: 600; }
  .cs-opt-in { padding-left: 1.25rem; }

  .cs-group-label {
    padding: 0.35rem 0.75rem 0.1rem;
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-top: 1px solid var(--border);
  }
  .cs-group-label.first { border-top: none; }
</style>
