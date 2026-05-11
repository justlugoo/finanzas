<script lang="ts">
  import Nav from '$lib/components/Nav.svelte';
  import '../app.css';
  let { children } = $props();
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
