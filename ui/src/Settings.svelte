<script>
  import { invoke } from '@tauri-apps/api/core';

  let settings = $state(null);
  let hotkeyError = $state('');
  let saveError = $state('');
  let recording = $state(false);

  invoke('settings_get').then((v) => {
    settings = v.settings;
    hotkeyError = v.hotkeyError ?? '';
  });

  const MODS = ['Control', 'Shift', 'Alt', 'Meta'];

  function label(accel) {
    return accel
      .split('+')
      .map((k) => k.replace(/^Key|^Digit/, '').replace('Backquote', '`').replace('Super', 'Win'))
      .join(' + ');
  }

  async function save(next) {
    saveError = '';
    try {
      await invoke('settings_set', { settings: next });
      settings = next;
      return true;
    } catch (e) {
      saveError = String(e);
      return false;
    }
  }

  async function record() {
    recording = true;
    await invoke('hotkey_pause', { paused: true });
  }

  async function stopRecording() {
    recording = false;
    await invoke('hotkey_pause', { paused: false });
  }

  async function keydown(e) {
    if (!recording) return;
    e.preventDefault();
    if (e.key === 'Escape') return stopRecording();
    if (MODS.includes(e.key)) return;
    const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
    if (!mods.length && !/^F\d+$/.test(e.code)) {
      saveError = 'Use at least one of Ctrl, Alt, Shift or Win, so the key still works in games.';
      return;
    }
    if (await save({ ...settings, hotkey: [...mods, e.code].join('+') })) hotkeyError = '';
    await stopRecording();
  }

  function setApp(field, value) {
    save({ ...settings, app: { ...settings.app, [field]: value } });
  }

  let running = $state(null);

  async function browse() {
    saveError = '';
    try {
      const app = await invoke('app_browse');
      if (app) settings = { ...settings, app };
    } catch (e) {
      saveError = String(e);
    }
  }

  async function listRunning() {
    running = running ? null : await invoke('apps_running');
  }

  async function attach(w) {
    saveError = '';
    try {
      settings = { ...settings, app: await invoke('app_attach', { hwnd: w.hwnd }) };
      running = null;
    } catch (e) {
      saveError = String(e);
    }
  }

  const fileName = (path) => path.split(/[\\/]/).pop();
</script>

<svelte:window onkeydown={keydown} onblur={() => recording && stopRecording()} />

{#if settings}
  <main>
    <h1>Glaude settings</h1>

    <section>
      <h2>Hotkey</h2>
      <p class="soft">Opens and closes the overlay. Esc never closes it, because your apps use Esc to stop a reply.</p>
      <button class="key" class:recording onclick={record}>
        {recording ? 'Press the new keys (Esc to cancel)' : label(settings.hotkey)}
      </button>
      {#if hotkeyError}<p class="error">{hotkeyError}</p>{/if}
    </section>

    <section>
      <h2>Dim</h2>
      <label class="row">
        <input
          type="range"
          min="0"
          max="95"
          value={Math.round(settings.dim * 100)}
          onchange={(e) => save({ ...settings, dim: e.currentTarget.value / 100 })}
        />
        <span>{Math.round(settings.dim * 100)}%</span>
      </label>
    </section>

    <section>
      <h2>App</h2>
      <p class="soft">Started the first time you open the overlay, or taken over if it is already open, then kept out of sight between opens.</p>
      <div class="buttons">
        <button onclick={browse}>Browse for a program</button>
        <button onclick={listRunning}>{running ? 'Hide running apps' : 'Pick a running app'}</button>
      </div>
      {#if running}
        <ul class="running">
          {#each running as w (w.hwnd)}
            <li><button onclick={() => attach(w)}><span>{w.title}</span><span class="soft">{fileName(w.exe)}</span></button></li>
          {:else}
            <li class="soft">No app windows are open.</li>
          {/each}
        </ul>
      {/if}
      <label>Name <input value={settings.app.name} onchange={(e) => setApp('name', e.currentTarget.value)} /></label>
      <label>Program <input value={settings.app.exe} onchange={(e) => setApp('exe', e.currentTarget.value)} /></label>
      <label>Arguments <input value={settings.app.args} onchange={(e) => setApp('args', e.currentTarget.value)} /></label>
      <label>
        Window comes from
        <input value={settings.app.windowExe} placeholder="Only if the program hands off to another" onchange={(e) => setApp('windowExe', e.currentTarget.value)} />
      </label>
    </section>

    {#if saveError}<p class="error">{saveError}</p>{/if}
  </main>
{/if}

<style>
  :global(body) {
    margin: 0;
    background: #0f1215;
    color: #e6e8eb;
    font: 15px/1.5 'Segoe UI', system-ui, sans-serif;
  }
  main {
    padding: 24px 28px;
  }
  h1 {
    font-size: 1.4rem;
    margin: 0 0 20px;
  }
  h2 {
    font-size: 1rem;
    margin: 0 0 4px;
  }
  section {
    background: #15191d;
    border-radius: 8px;
    padding: 14px 16px;
    margin-bottom: 12px;
  }
  p {
    margin: 0 0 10px;
  }
  .soft {
    color: #9aa3ad;
    font-size: 0.9rem;
  }
  .error {
    color: #e8896b;
    margin: 8px 0 0;
  }
  label {
    display: grid;
    grid-template-columns: 140px 1fr;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
    color: #9aa3ad;
  }
  label.row {
    grid-template-columns: 1fr 48px;
  }
  label.row span {
    color: #e6e8eb;
    text-align: right;
  }
  input:not([type='range']),
  .key {
    background: #0f1215;
    color: #e6e8eb;
    border: 0;
    border-radius: 6px;
    padding: 7px 10px;
    font: inherit;
  }
  input:focus-visible,
  .key:focus-visible {
    outline: 2px solid #2fbfa8;
  }
  input[type='range'] {
    accent-color: #2fbfa8;
  }
  .key {
    cursor: pointer;
    min-width: 160px;
    text-align: left;
  }
  .key.recording {
    color: #2fbfa8;
  }
  .buttons {
    display: flex;
    gap: 8px;
  }
  .buttons button,
  .running button {
    background: #0f1215;
    color: #e6e8eb;
    border: 0;
    border-radius: 6px;
    padding: 7px 12px;
    font: inherit;
    cursor: pointer;
  }
  .buttons button:hover,
  .running button:hover {
    color: #2fbfa8;
  }
  .running {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-height: 220px;
    overflow-y: auto;
  }
  .running li + li {
    margin-top: 4px;
  }
  .running button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    text-align: left;
  }
  .running button span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
