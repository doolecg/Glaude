<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let dim = $state(0.72);
  let note = $state('');
  let shown = $state(false);

  listen('shade:open', (e) => {
    dim = e.payload.dim;
    note = e.payload.note;
    shown = true;
  });
  listen('shade:close', () => (shown = false));
  listen('shade:dim', (e) => (dim = e.payload));
</script>

<!-- A click anywhere on the dim closes the overlay; the app window sits on top of the panel area. -->
<button class="shade" class:shown style:--dim={dim} onclick={() => invoke('overlay_close')} aria-label="Close the overlay">
  {#if note}<span>{note}</span>{/if}
</button>

<style>
  .shade {
    position: fixed;
    inset: 0;
    width: 100%;
    border: 0;
    padding: 0;
    margin: 0;
    cursor: default;
    background: rgba(11, 13, 16, var(--dim));
    color: #e6e8eb;
    font: 17px/1.5 'Segoe UI', system-ui, sans-serif;
    opacity: 0;
    transition: opacity 100ms linear;
  }
  .shade.shown {
    opacity: 1;
    transition-duration: 120ms;
  }
</style>
