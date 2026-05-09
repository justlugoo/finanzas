<script lang="ts">
  import { page } from '$app/stores';

  const tabs = [
    { href: '/',           label: 'Resumen'   },
    { href: '/registrar',  label: 'Registrar' },
    { href: '/historial',  label: 'Historial' },
    { href: '/objetivos',  label: 'Objetivos' },
    { href: '/config',     label: 'Config'    },
  ];
</script>

<nav class="app-nav">
  {#each tabs as tab}
    <a
      href={tab.href}
      class="tab"
      class:active={$page.url.pathname === tab.href}
    >
      {tab.label}
    </a>
  {/each}
</nav>

<style>
  .app-nav {
    display: flex;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    padding: 0 1.5rem;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .tab {
    padding: 0.7rem 1rem;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab:hover { color: var(--text-primary); }

  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
</style>
