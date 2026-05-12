<script lang="ts">
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

  let allOptions = $derived([
    ...options,
    ...groups.flatMap((g: Group) => g.options),
  ]);

  let selectedLabel = $derived(
    allOptions.find((o: Option) => o.value === value)?.label ?? placeholder
  );

  function select(v: any) {
    value = v;
    open = false;
    onchange?.(v);
  }

  $effect(() => {
    if (!open) return;
    function onWindowClick(e: MouseEvent) {
      if (wrapEl && !wrapEl.contains(e.target as Node)) {
        open = false;
      }
    }
    const id = setTimeout(() => window.addEventListener("click", onWindowClick), 0);
    return () => {
      clearTimeout(id);
      window.removeEventListener("click", onWindowClick);
    };
  });
</script>

<div class="cs-wrap" bind:this={wrapEl}>
  <button
    type="button"
    class="cs-trigger"
    onclick={() => (open = !open)}
    {disabled}
  >
    <span class="cs-label">{selectedLabel}</span>
    <span class="cs-arrow" class:open>▾</span>
  </button>
  {#if open}
    <div class="cs-menu">
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
    </div>
  {/if}
</div>

<style>
  .cs-wrap {
    position: relative;
    display: block;
  }

  .cs-trigger {
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
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 100%;
    background: #14141f;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    z-index: 200;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.55);
    max-height: 260px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #2a2a40 transparent;
  }

  .cs-opt {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.75rem;
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
