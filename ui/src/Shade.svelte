<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let dim = $state(0.72);
  let apps = $state([]);
  let current = $state(0);
  let note = $state('');

  listen('shade:open', (e) => ({ dim, apps, current, note } = e.payload));
  listen('shade:current', (e) => ({ current, note } = e.payload));
  listen('shade:dim', (e) => (dim = e.payload));
</script>

<!-- A click on the dim closes the overlay. The panel is solid so an app can be resized behind it unseen before it is
     lifted on top, below the rail. -->
<div class="shade" style:--dim={dim} onclick={() => invoke('overlay_close')} role="presentation">
  <div class="panel" onclick={(e) => e.stopPropagation()} role="presentation">
    {#if apps.length > 1}
      <nav class="rail">
        {#each apps as name, i}
          <button class:on={i === current} onclick={() => invoke('overlay_switch', { index: i })}>{name}</button>
        {/each}
        <span class="hint">Click the dim to go back</span>
      </nav>
    {/if}
    {#if note}<p class="note">{note}</p>{/if}
  </div>
</div>

<style>
  .shade {
    position: fixed;
    inset: 0;
    background: rgba(11, 13, 16, var(--dim));
    color: #e6e8eb;
    font: 15px/1.5 'Segoe UI', system-ui, sans-serif;
  }
  /* 50px in from each edge, with the rail as its top 40px (MARGIN and RAIL in overlay.rs). */
  .panel {
    position: absolute;
    inset: 50px;
    background: #0f1215;
    border-radius: 8px;
    overflow: hidden;
  }
  .rail {
    height: 40px;
    display: flex;
    align-items: stretch;
    padding: 0 8px;
    background: #15191d;
  }
  .rail button {
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    color: #9aa3ad;
    font: inherit;
    padding: 0 14px;
    cursor: pointer;
  }
  .rail button:hover {
    color: #e6e8eb;
  }
  .rail button.on {
    color: #e6e8eb;
    border-bottom-color: #2fbfa8;
  }
  .hint {
    margin-left: auto;
    align-self: center;
    color: #6c757f;
    font-size: 13px;
    padding-right: 8px;
  }
  .note {
    margin: 0;
    position: absolute;
    top: 50%;
    width: 100%;
    text-align: center;
  }
</style>
