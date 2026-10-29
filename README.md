<p align="center">
  <img src="docs/logo.png" alt="Glaude" width="128">
</p>

<h1 align="center">Glaude</h1>

<p align="center">
  <strong>Check on your AI coding agents without leaving the game.</strong><br>
  One hotkey opens Claude, T3 Code, a terminal or any app in a panel over the dimmed game. The same key puts you back in the game.<br>
  No Alt-Tab, no injected code, nothing sent off your computer.
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2fbfa8?style=flat-square&labelColor=0f1215"></a>
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%20%7C%2011-2fbfa8?style=flat-square&labelColor=0f1215">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-2fbfa8?style=flat-square&labelColor=0f1215">
  <img alt="Version 0.1.0" src="https://img.shields.io/badge/version-0.1.0-2fbfa8?style=flat-square&labelColor=0f1215">
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#why-glaude">Why Glaude</a> ·
  <a href="#features">Features</a> ·
  <a href="#where-it-works">Where it works</a> ·
  <a href="#what-glaude-does-on-your-machine">What it does on your machine</a> ·
  <a href="#privacy">Privacy</a> ·
  <a href="#development">Development</a> ·
  <a href="https://github.com/doolecg/Glaude/releases">Releases</a>
</p>

<p align="center">
  <img src="docs/images/hero.jpg" alt="T3 Code open over Over the Hill, with the dimmed game showing round the edge" width="100%">
</p>
<p align="center"><sub>T3 Code over Over the Hill, one key press from the game below.</sub></p>

---

You start an agent on a long task, then start a game while it works. Now you want to know if it is finished, or if it is stuck on a question. Alt-Tab drops you out of the game, some games minimise, and the mouse lock breaks. **Glaude is a small tray app that puts the agent's real window over the game and takes it away again with one key.** It does not draw inside the game: it moves the app's own window above it and hands focus back when you are done.

<p align="center">
  <img src="docs/images/game.jpg" alt="Over the Hill before the hotkey" width="49%">
  <img src="docs/images/hero.jpg" alt="The same moment after the hotkey: T3 Code in a panel over the game" width="49%">
</p>

## Quick start

Windows 10 or 11.

1. Download the `.msi` installer from [Releases](https://github.com/doolecg/Glaude/releases) and run it. Windows asks for admin permission to install.
2. Left-click the Glaude tray icon and press **Pick a running app** to add your agent's window (or **Browse for a program**).
3. In a game, press **Ctrl+`** (the key above Tab) to open the panel. Press it again to go back.

That's it. Glaude keeps itself up to date from the same Releases page. Read [What Glaude does on your machine](#what-glaude-does-on-your-machine) first.

## Why Glaude

**One key in, one key out.** The hotkey opens the panel and gives the app the keyboard straight away. The same key closes it and gives the game the foreground again, so its mouse lock comes back on its own. A click on the dim edge closes it too.

**Leave the game alone.** Steam and Discord draw their overlays by injecting code into the game. Glaude does not touch the game at all, so there is nothing for anti-cheat to object to.

**The app stays as it is.** What you see is the app's own window, not a copy or a restyle. You type into it as normal, and Esc stays with the app (Claude Code and OpenCode use Esc to stop a reply).

**Your apps stay yours.** Glaude borrows a window only while it is in the panel. Close the overlay and every app is back on your desktop where you left it, ready to use as normal. Opening takes a few milliseconds.

## Features

| Capability | What you get |
| --- | --- |
| Hotkey overlay | Ctrl+` opens the app in a panel 50px in from every edge of the game's monitor, over the game dimmed to 72% |
| Several apps, one strip | Add T3 Code, Chrome, Files, Discord or anything else; a strip along the top of the panel switches between them with a click |
| Back to the desktop as normal | Closing or switching puts each window back exactly where it was on your desktop, at its size, visible and on the taskbar |
| Pick a running app | Choose any open window from a list; Glaude adds it to the strip and uses that window |
| Browse for a program | Pick any program (or its shortcut); Glaude starts it the first time it is needed |
| Never a second copy | An app that is already open, even hidden in the tray, is shown in its own window; Glaude only starts apps that are not open at all |
| Any keyboard layout | The hotkey is stored by key position, so Ctrl+` is the key above Tab on UK and US keyboards alike |
| Settings, applied live | Rebind the hotkey by pressing it, set the dim level, change the apps; saved as you go |
| Updates | Checks GitHub at start and every few hours, downloads in the background, installs when you press Install or quit; release notes in Settings |
| Multi-monitor and scaling | The panel opens on the monitor the game is on, measured in display-scaled pixels so it looks the same at 1080p and 4K |

## Where it works

| Game display mode | Overlay |
| --- | --- |
| Borderless window, most modern games' default | Works |
| Fullscreen with Fullscreen Optimisations on, the Windows 10 and 11 default | Works |
| True exclusive fullscreen | Does not show, because the game owns the screen. Switch the game to Borderless |

Some anti-cheat games pull focus back every frame. Glaude does not fight them for it.

Not built yet: one-click presets for Claude, T3 Code and OpenCode. The plan is in the [design](docs/overlay-design.html) and the [bible](docs/bible.html).

Read this before you install. By using Glaude you agree to it doing the following.

- **Registers one global hotkey** (Ctrl+` unless you change it), and lets go of it while you record a new one.
- **Starts the programs you chose**, only when one is not open at all, with the arguments you gave. It never starts a second copy of a program that is already open.
- **Moves other programs' windows.** The app you pick is moved into the panel above the game and given the keyboard. When you switch away or close, it goes back to the exact place, size and state it had on your desktop; an app that was hidden in the tray is hidden again. Glaude does not read, change or save anything inside those programs.
- **Installs its own updates.** It downloads new versions of itself from GitHub and runs the Windows installer when you press Install or quit Glaude. The installer is checked against Glaude's signing key first and refused if it does not match. Windows shows its admin prompt before each install.
- **Keeps its settings** in `%APPDATA%\io.glaude.app\settings.json`: the hotkey, the dim level and your list of apps.

Glaude never draws inside or changes a game. Some games' anti-cheat may still dislike other windows taking focus; check a game's rules if you are unsure.

## Privacy

Glaude has no account, no analytics, no tracking and no crash reporting. Your settings stay on your computer. Its only connection is:

- **Update checks.** At start and every few hours Glaude asks GitHub (`github.com`) for the latest release and downloads the installer from there when there is a newer one. GitHub sees your IP address and that you run Glaude, under GitHub's own privacy terms.

The apps you open in Glaude (T3 Code, Claude, Chrome, Discord and so on) send your prompts, files and messages to their own services under their own terms. Glaude neither sees nor controls that traffic.

## Development

```sh
npm install          # once
npm run dev          # run the working tree with live reload
npm run main         # run the main branch from a clean copy in build/main-run
```

The code is small: `src-tauri/src/overlay.rs` (shade and app window), `hotkey.rs` (the global hotkey), `win.rs` (the Windows calls) and `settings.rs`, plus two Svelte pages in `ui/src`. How the parts fit is in [docs/systems.md](docs/systems.md) and the reasons in [docs/design.md](docs/design.md).

## Credits

The Windows tricks Glaude uses (showing a window without taking focus, letting go of shortcuts while another window needs them) come from my other project, Notifi. Over the Hill, shown in the screenshots, belongs to its makers. Glaude is not connected with it.

## Licence

Glaude is released under the [MIT Licence](LICENSE). Copyright (c) 2026 doolecg.

Glaude is built on open-source software (Tauri, Svelte, the `windows` crate and the packages they use), each under its own licence. Their copyright notices and licence texts are in [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), which the installer also places next to the program with LICENSE. `npm run notices` regenerates it from the dependency list.

## Disclaimer

This software is provided "as is", without warranty of any kind, as set out in the [LICENSE](LICENSE). To the extent permitted by law, the authors are not liable for any loss or damage from using it, including lost data, lost work or damage to files or projects. Nothing here limits liability that cannot be limited by law, such as for death or personal injury caused by negligence.

Glaude moves and focuses other programs' windows. It is not responsible for what those programs do, and it cannot guarantee how any game or its anti-cheat reacts to another window taking focus. Some games may not allow it; check a game's terms before using Glaude with it, especially in online or competitive play.

Not affiliated with or endorsed by Anthropic, Microsoft, Google, Discord, Valve, the makers of T3 Code or OpenCode, or the makers of any game. Claude is a trademark of Anthropic; Windows and File Explorer of Microsoft; Chrome of Google; Discord of Discord Inc.; Steam of Valve; Over the Hill and every other game named or shown are trademarks of their owners. Names and screenshots are used only to show what Glaude works with.

The AI apps you run in Glaude send your prompts and files to their own services under your account and their terms. AI output can be wrong; check it before relying on it.

Back up your work before using it. You use it at your own risk.
