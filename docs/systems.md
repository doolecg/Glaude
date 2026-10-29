# Glaude: systems

What each system is, where it lives and how it connects to the others. Update when a system is added or changes. File paths below are confirmed against the milestone 1 code (2026-10-28).

## Tray
- **What it does:** keeps Glaude running with no main window. The tray icon opens Settings and quits the app.
- **Where:** `src-tauri/src/main.rs` (tray, commands, setup).
- **Talks to:** the Settings window, through tray menu events.
- **Saved:** nothing.

## Hotkey
- **What it does:** registers the open/close shortcut (default Ctrl+`) and toggles the overlay when it is pressed.
- **Where:** `src-tauri/src/hotkey.rs` (RegisterHotKey on its own thread; keys by physical position, mapped through the active layout).
- **Talks to:** the overlay controller (shade and app host); reads the key from the settings store and re-registers it when the setting changes.
- **Saved:** the key itself lives in the settings store.

## Shade window
- **What it does:** a borderless, transparent, always-on-top Tauri window stretched over the game's monitor. It paints the dim (default 72%, `#0b0d10`) and takes a click on the dim to close the overlay. With two or more apps it also paints the rail (40px strip at the top of the panel); a click on a name switches app.
- **Where:** `src-tauri/src/overlay.rs` plus `ui/src/Shade.svelte`; Win32 placement in `src-tauri/src/win.rs`.
- **Talks to:** the hotkey (open and close), the app host (the panel sits above it), the settings store (dim level, animation).
- **Saved:** nothing; the dim level comes from settings.

## App host
- **What it does:** starts the configured app on first open, finds its first top-level window, and keeps that window handle. Open moves the window to the panel (monitor minus 50px on every side, display-scaled), sets it above the shade with `HWND_TOPMOST` and focuses it. It records the window's desktop placement when it takes it; switching away or closing restores that placement, visible and just below the game. An open app is never started again: its visible window is used, or its hidden main window (matched by exe and the window class saved in settings), which is hidden again on close. Only an app that is not open is started. It also remembers the previously focused window (the game) and hands the foreground back on close. If the app's window disappears, the next open starts it again.
- **Where:** `src-tauri/src/overlay.rs` (`take`, `switch`, `start`, `settle_later`, `watch`, `close`) and `src-tauri/src/win.rs`.
- **Talks to:** the hotkey and shade window (open and close sequence), the settings store (app path and arguments).
- **Saved:** nothing; the window handle is held in memory only.

## Updater
- **What it does:** checks `https://github.com/doolecg/Glaude/releases/latest/download/latest.json` 20 seconds after start and every 4 hours, downloads a newer `.msi` in the background (the Tauri updater plugin refuses it unless it is signed with Glaude's key), then installs it when you press Install in Settings or the tray, or when Glaude quits. Windows' installer replaces Glaude and starts it again.
- **Where:** `src-tauri/src/updater.rs`, the tray's update line in `src-tauri/src/main.rs`, and the Updates and About sections of `ui/src/Settings.svelte` (`ui/src/markdown.js` renders the notes).
- **Talks to:** GitHub releases (the only network use), the tray menu and the Settings page.
- **Saved:** nothing; a downloaded update waits in memory.

## Settings store
- **What it does:** holds the user's options (hotkey, the list of apps with path and arguments, dim level; a 0.1.0 file's single `app` is moved into the list on load), applies changes live and writes them on change.
- **Where:** `src-tauri/src/settings.rs` (store, `%APPDATA%\io.glaude.app\settings.json`) and `ui/src/Settings.svelte`.
- **Talks to:** the hotkey, shade window and app host, which read it; the Settings window, which writes it.
- **Saved:** a settings file in the app's data folder, written on every change.
