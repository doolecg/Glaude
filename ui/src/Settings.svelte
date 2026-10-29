<script>
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { listen } from '@tauri-apps/api/event';
  import { markdown } from './markdown.js';

  let version = $state('');
  let update = $state({ state: 'idle' });
  let notes = $state(null);
  getVersion().then((v) => (version = v));
  const loadUpdate = () => {
    invoke('update_status').then((s) => (update = s));
    invoke('update_notes').then((n) => (notes = n));
  };
  loadUpdate();
  listen('updater://status', loadUpdate);
  const busy = $derived(update.state === 'checking' || update.state === 'downloading');
  const updateText = $derived(
    {
      idle: 'Not checked yet.',
      checking: 'Checking for updates…',
      upToDate: 'Glaude is up to date.',
      downloading: `Downloading ${update.version}: ${update.percent}%`,
      ready: `Version ${update.version} is ready. It installs when you press Install or quit Glaude.`,
      error: update.message,
    }[update.state],
  );

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

  function setApp(i, field, value) {
    save({ ...settings, apps: settings.apps.map((a, j) => (j === i ? { ...a, [field]: value } : a)) });
  }

  function move(i, by) {
    const apps = [...settings.apps];
    [apps[i], apps[i + by]] = [apps[i + by], apps[i]];
    save({ ...settings, apps });
  }

  function remove(i) {
    save({ ...settings, apps: settings.apps.filter((_, j) => j !== i) });
  }

  let running = $state(null);

  async function browse() {
    saveError = '';
    try {
      const next = await invoke('app_browse');
      if (next) settings = next;
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
      settings = await invoke('app_attach', { hwnd: w.hwnd });
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
      <h2>Apps</h2>
      <p class="soft">
        The hotkey opens the one you used last; with two or more, a strip along the top of the panel switches between them. An app
        that is already open is shown as it is, and goes back to its place on your desktop when you switch away or close.
      </p>
      <ul class="apps">
        {#each settings.apps as a, i (i)}
          <li>
            <input class="name" value={a.name} title="Name on the strip" onchange={(e) => setApp(i, 'name', e.currentTarget.value)} />
            <span class="soft exe" title={a.exe}>{fileName(a.exe)}</span>
            <input class="args" value={a.args} placeholder="Arguments" onchange={(e) => setApp(i, 'args', e.currentTarget.value)} />
            <button title="Move up" disabled={i === 0} onclick={() => move(i, -1)}>↑</button>
            <button title="Move down" disabled={i === settings.apps.length - 1} onclick={() => move(i, 1)}>↓</button>
            <button title="Remove" onclick={() => remove(i)}>Remove</button>
          </li>
        {:else}
          <li class="soft">No apps yet. Add one below.</li>
        {/each}
      </ul>
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
    </section>

    {#if saveError}<p class="error">{saveError}</p>{/if}

    <section>
      <h2>Updates</h2>
      <p class:error={update.state === 'error'} class:soft={update.state !== 'error'}>{updateText}</p>
      <div class="buttons">
        <button disabled={busy} onclick={() => invoke('update_check')}>Check for updates</button>
        {#if update.state === 'ready'}<button class="go" onclick={() => invoke('update_install')}>Install {update.version}</button>{/if}
      </div>
      {#if notes?.markdown}
        <div class="notes">
          <p class="soft">{notes.version === version ? "What's new in this version" : `What's new in ${notes.version}`}</p>
          {@html markdown(notes.markdown)}
        </div>
      {/if}
    </section>

    <section>
      <h2>About</h2>
      <p>Glaude {version}. MIT licence, copyright 2026 doolecg.</p>
      <p class="soft">
        Provided "as is", without warranty, under the MIT licence. To the extent permitted by law, the authors are not
        liable for any loss or damage from using it. Back up your work.
      </p>
      <p class="soft">Updates come from github.com/doolecg/Glaude releases. Glaude sends nothing else off your computer.</p>
      <p class="soft">
        Glaude moves other programs' windows and starts the programs you add. It does not change anything inside them, and
        cannot promise how a game or its anti-cheat reacts. Built on open-source software; the licences are in
        THIRD_PARTY_NOTICES.txt in Glaude's install folder. Not affiliated with Anthropic, Microsoft or the makers of the
        apps and games you use with it.
      </p>
    </section>
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
    margin-top: 10px;
  }
  .apps {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .apps li {
    display: grid;
    grid-template-columns: 1fr 110px 1fr auto auto auto;
    gap: 6px;
    align-items: center;
    margin-top: 6px;
  }
  .apps .exe {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .apps button {
    background: #0f1215;
    color: #e6e8eb;
    border: 0;
    border-radius: 6px;
    padding: 7px 10px;
    font: inherit;
    cursor: pointer;
  }
  .apps button:disabled {
    color: #3a4048;
    cursor: default;
  }
  .apps input {
    min-width: 0;
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
  .buttons button:disabled {
    color: #3a4048;
    cursor: default;
  }
  .buttons .go {
    color: #2fbfa8;
  }
  .notes {
    margin-top: 12px;
    font-size: 0.92rem;
  }
  .notes :global(h1) {
    font-size: 1.05rem;
    margin: 0 0 6px;
  }
  .notes :global(h2) {
    font-size: 0.95rem;
    margin: 10px 0 4px;
  }
  .notes :global(ul) {
    margin: 0;
    padding-left: 20px;
  }
  .notes :global(li) {
    margin-bottom: 4px;
  }
  .notes :global(hr) {
    border: 0;
    margin: 12px 0;
  }
  .running button span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
