# Glaude: design notes

The why behind decisions, and the results of any tests that informed them. The bible says what was decided; this says why. Newest first.

## Milestone 1 deviations from the design (2026-10-28)
- **Decision (later the same day):** the hotkey moved off `tauri-plugin-global-shortcut` to Glaude's own RegisterHotKey thread. Why: the plugin turns key names into US key codes, so on a UK keyboard Ctrl+` registered the wrong key and never fired. Glaude now stores the key's physical position and asks Windows for that key's code on the layout in use.
- **Decision (later the same day):** the app is picked in Settings by browsing or from a list of running windows, pulled forward from milestone 2 at doolecg's request. A started app that quits at once and hands over to a running copy (Claude, T3 Code) is taken over in its existing window after 2 seconds, rather than reporting no window.
- **Decision:** parked app windows are hidden with `SW_HIDE` as well as moved off-screen. The hotkey uses the official `tauri-plugin-global-shortcut` instead of raw `windows` crate calls.
- **Why:** an off-screen window that is still visible keeps a taskbar button, which would show Terminal sitting there while the game runs. Hiding removes it. The plugin is the supported way to register a global shortcut in Tauri, so there is less code to own than with direct Win32 hotkey calls.
- **Evidence:** none yet; milestone 1's 20-open check will show whether hide and show cause flicker.

## Name and defaults (2026-10-28)
- **Decision:** hotkey `` Ctrl+` ``, dim 72%, apps start on first open, "Glaude" is the shipped name.
- **Why:** `` Ctrl+` `` is the drop-down terminal convention and is free in games and in all four apps; Shift+Tab is owned by Steam and used by Claude Code. 72% keeps the game readable as context while the panel stands out. Starting apps on first open means nothing runs that was not asked for, at the cost of a one-off wait of a second or two. The name is fine as a repo name and the About page spells it out once.
- **Evidence:** the four recommended answers in section 10 of `docs/overlay-design.html`, accepted on 2026-10-28.

## Look (2026-10-28)
- **Decision:** one ground, one ink, teal `#2fbfa8` as the only accent. Segoe UI. Shade `#0b0d10`. Rail 40px, active app underlined. Fade in 120ms, out 100ms, no slide, no glow, no blur.
- **Why:** follows the design bank's rules for the dark style already approved: no gradients, no shadows, no uppercase letter-spaced labels. The app inside the panel is the real content, so Glaude stays out of its way.

## Keys (2026-10-28)
- **Decision:** open and close with `` Ctrl+` `` (rebindable). Esc does not close. Switch apps with `Ctrl+1` to `Ctrl+9` while open, and release those keys when the overlay is closed.
- **Why:** Claude Code and OpenCode use Esc to stop a reply, so closing on Esc would lose work. Releasing the switch keys when closed means a game that uses `Ctrl+1` still gets it. Glaude registers only its own shortcuts.

## Where it works (2026-10-28)
- **Decision:** borderless window and Fullscreen Optimisations modes are supported. True exclusive fullscreen is not. When the hotkey is pressed over a game Glaude cannot classify, it still switches focus to the app like an Alt-Tab, and Settings explains how to set the game to Borderless. If a game keeps taking focus back, Glaude does not fight it.
- **Why:** Windows composites every window in those two modes, so a topmost window shows over the game. Exclusive fullscreen would need a hook inside the game, which is what Steam and Discord do by injecting code and what this project avoids. Fighting for focus would be a losing and unfriendly arms race.
- **Evidence:** design doc section 3. Peek mode (a live thumbnail with no focus change) is the parked answer if foreground-fighting games turn out common.

## Stack and borrowed code (2026-10-28)
- **Decision:** Tauri 2, Rust and Svelte 5, with the `windows` crate for the few Win32 calls. The Win32 helpers are copied from Notifi, not shared.
- **Why:** same stack as Notifi, so the patterns carry over: showing a window without stealing focus, parking a window off-screen, and letting go of global shortcuts while a page in front needs its keys. They are a few dozen lines each, and copying means Glaude does not depend on Notifi.

## Two windows, real app window (2026-10-28)
- **Decision:** the overlay is the shade (Glaude's own window) plus the app's real window, moved to the panel rectangle, put above the shade with `HWND_TOPMOST` and focused. Glaude does not show a picture of the app.
- **Why:** the user can type straight away, the app needs no changes, and nothing is injected into the game. On close Glaude parks the app, hides the shade and returns the foreground to the window that was in front before, which also brings the game's mouse lock back.
