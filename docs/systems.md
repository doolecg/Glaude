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
- **What it does:** a borderless, transparent, always-on-top Tauri window stretched over the game's monitor. It paints the dim (default 72%, `#0b0d10`) and takes a click on the dim to close the overlay. From milestone 2 it also paints the rail.
- **Where:** `src-tauri/src/overlay.rs` plus `ui/src/Shade.svelte`; Win32 placement in `src-tauri/src/win.rs`.
- **Talks to:** the hotkey (open and close), the app host (the panel sits above it), the settings store (dim level, animation).
- **Saved:** nothing; the dim level comes from settings.

## App host and parking
- **What it does:** starts the configured app on first open, finds its first top-level window, and keeps that window handle. Open moves the window to the panel (monitor minus 50px on every side, display-scaled), sets it above the shade with `HWND_TOPMOST` and focuses it. Close parks it again: moved off-screen and hidden with `SW_HIDE`, so there is no taskbar button. It also remembers the previously focused window (the game) and hands the foreground back on close. If the app's window disappears, the next open starts it again.
- **Where:** `src-tauri/src/overlay.rs` (`start`, `settle`, `watch`, `release`) and `src-tauri/src/win.rs`.
- **Talks to:** the hotkey and shade window (open and close sequence), the settings store (app path and arguments).
- **Saved:** nothing; the window handle is held in memory only.

## Settings store
- **What it does:** holds the user's options (milestone 1: hotkey, one app with path and arguments, dim level), applies changes live and writes them on change.
- **Where:** `src-tauri/src/settings.rs` (store, `%APPDATA%\io.glaude.app\settings.json`) and `ui/src/Settings.svelte`.
- **Talks to:** the hotkey, shade window and app host, which read it; the Settings window, which writes it.
- **Saved:** a settings file in the app's data folder, written on every change.
