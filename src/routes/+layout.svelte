<script lang="ts">
  import { onMount } from 'svelte';
  import Nav from '$lib/components/Nav.svelte';
  import '../app.css';
  import { loadCache } from '$lib/appStore.svelte';

  let { children } = $props();

  onMount(() => { loadCache(); });
</script>

<Nav />
<div class="page-wrapper">
  {@render children()}
</div>

<style>
  .page-wrapper {
    min-height: calc(100vh - 41px);
  }
</style>
