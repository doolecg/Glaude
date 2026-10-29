//! The overlay: the shade (Glaude's own dim window over the game's monitor, with the rail of apps along the top of
//! the panel) and the chosen app's real window placed above it. An app's window is borrowed only while it is in the
//! panel: switching away or closing puts it back on the desktop exactly where it was.

use crate::settings::{AppEntry, Store};
use crate::win;
use serde::Serialize;
use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SHADE: &str = "shade";
/// Gap between the panel and the monitor edge, in 96-dpi pixels.
const MARGIN: f64 = 50.0;
/// Height of the rail at the top of the panel, in 96-dpi pixels (shade.svelte draws it at the same size).
const RAIL: f64 = 40.0;
/// Screen redraws to wait after resizing an app before showing it: long enough for Electron apps to repaint.
const PAINT_FRAMES: u32 = 3;
const START_TIMEOUT: Duration = Duration::from_secs(15);
/// How long after the started process quits to keep waiting for a new window before taking an existing one.
const HANDOVER: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Overlay {
    state: Mutex<State>,
    /// Set while an open, close or switch is running, so a second press in that time is ignored.
    busy: AtomicBool,
}

#[derive(Default)]
struct State {
    open: bool,
    game: isize,
    /// The app in the panel now, or used last: an index into the settings' apps.
    current: usize,
    /// Each app's window once found, by the same index; 0 = not found yet.
    windows: Vec<isize>,
    /// The window in the panel and where it was on the desktop before Glaude took it.
    taken: Option<(isize, win::Placement)>,
    /// Where the app goes: the panel, less the rail when there is one.
    area: Option<win::Rect>,
}

#[derive(Clone, Serialize)]
struct Open {
    dim: f64,
    apps: Vec<String>,
    current: usize,
    note: String,
}

#[derive(Clone, Serialize)]
struct Current {
    current: usize,
    note: String,
}

pub fn create_shade(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, SHADE, WebviewUrl::App("shade.html".into()))
        .title("Glaude")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()?;
    // The first time a web view is shown it sets up its drawing surface, which held up the first open by about
    // 200ms. Show it once off-screen at full size while nobody is looking, so every open is quick.
    let app = app.clone();
    std::thread::spawn(move || {
        sleep(Duration::from_secs(2));
        let shade = shade_hwnd(&app);
        let (m, _) = win::monitor_of(0);
        win::show_topmost(shade, win::Rect { x: -32_000, y: -32_000, w: m.w, h: m.h });
        win::wait_frames(10);
        if !app.state::<Overlay>().state.lock().unwrap().open {
            win::hide(shade);
        }
    });
    Ok(())
}

fn shade_hwnd(app: &AppHandle) -> isize {
    app.get_webview_window(SHADE).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize).unwrap_or(0)
}

/// Runs `job` on its own thread unless another open, close or switch is still running.
fn exclusive(app: &AppHandle, job: impl FnOnce(&AppHandle) + Send + 'static) {
    if app.state::<Overlay>().busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        job(&app);
        app.state::<Overlay>().busy.store(false, Ordering::SeqCst);
    });
}

pub fn toggle(app: &AppHandle) {
    let open = app.state::<Overlay>().state.lock().unwrap().open;
    exclusive(app, move |app| if open { close(app) } else { show(app) });
}

/// Puts app `index` in the panel in place of the one there now.
pub fn switch(app: &AppHandle, index: usize) {
    exclusive(app, move |app| {
        let overlay = app.state::<Overlay>();
        let (taken, game) = {
            let mut st = overlay.state.lock().unwrap();
            if !st.open || st.current == index {
                return;
            }
            (st.taken.take(), st.game)
        };
        if let Some((h, p)) = taken {
            win::restore(h, p, game);
        }
        take(app, index);
    });
}

fn show(app: &AppHandle) {
    let settings = app.state::<Store>().get();
    let overlay = app.state::<Overlay>();
    let shade = shade_hwnd(app);
    let front = win::foreground();
    let current = {
        let mut st = overlay.state.lock().unwrap();
        st.open = true;
        st.game = if front == shade || st.windows.contains(&front) { 0 } else { front };
        st.current = st.current.min(settings.apps.len().saturating_sub(1));
        st.current
    };
    let (monitor, scale) = win::monitor_of(front);
    let panel = monitor.inset((MARGIN * scale).round() as i32);
    let rail = if settings.apps.len() > 1 { (RAIL * scale).round() as i32 } else { 0 };
    overlay.state.lock().unwrap().area = Some(win::Rect { x: panel.x, y: panel.y + rail, w: panel.w, h: panel.h - rail });
    let note = if settings.apps.is_empty() { "Add an app in Glaude's settings.".to_string() } else { String::new() };
    let apps = settings.apps.iter().map(|a| a.name.clone()).collect();
    let _ = app.emit_to(SHADE, "shade:open", Open { dim: settings.dim, apps, current, note });
    // Twice: moving onto a monitor with a different scale makes the window resize itself once on arrival.
    win::show_topmost(shade, monitor);
    win::show_topmost(shade, monitor);
    if !settings.apps.is_empty() {
        take(app, current);
    }
}

/// Finds app `index`'s window (starting the app if needed) and moves it into the panel.
fn take(app: &AppHandle, index: usize) {
    let settings = app.state::<Store>().get();
    let Some(entry) = settings.apps.get(index) else { return };
    let overlay = app.state::<Overlay>();
    let known = {
        let mut st = overlay.state.lock().unwrap();
        st.current = index;
        st.windows.resize(settings.apps.len(), 0);
        st.windows[index]
    };
    let found = if win::alive(known) {
        Ok(known)
    } else if let Some(h) = running_window(entry) {
        Ok(h)
    } else {
        let _ = app.emit_to(SHADE, "shade:current", Current { current: index, note: format!("Starting {}…", entry.name) });
        start(entry).map(|h| {
            settle_later(app.clone(), h);
            h
        })
    };
    let h = match found {
        Ok(h) => h,
        Err(note) => {
            let _ = app.emit_to(SHADE, "shade:current", Current { current: index, note });
            return;
        }
    };
    let area = {
        let mut st = overlay.state.lock().unwrap();
        st.windows[index] = h;
        if !st.open || st.current != index {
            return;
        }
        st.taken = Some((h, win::placement(h)));
        st.area
    };
    learn_class(app, index, h);
    let _ = app.emit_to(SHADE, "shade:current", Current { current: index, note: String::new() });
    let Some(area) = area else { return };
    // Resized under the shade's solid panel first, and lifted into view only once it has drawn at the new size,
    // so the move and the redraw are never seen.
    win::place(h, area);
    win::wait_frames(PAINT_FRAMES);
    let st = overlay.state.lock().unwrap();
    if st.open && st.taken.is_some_and(|(t, _)| t == h) {
        win::raise(h);
    }
    drop(st);
    watch(app.clone(), h);
}

/// The exe name that owns the app's window.
fn window_exe(entry: &AppEntry) -> &str {
    if entry.window_exe.is_empty() {
        win::file_name(&entry.exe)
    } else {
        &entry.window_exe
    }
}

/// A window the app already has open, so an app that is open is never started a second time: its front-most
/// visible window, or else its hidden main window (an app closed to the tray), matched by window class.
fn running_window(entry: &AppEntry) -> Option<isize> {
    let exe = window_exe(entry);
    // Class first: it is a cheap read, while the exe name means opening the process.
    let mine = |&(h, pid): &(isize, u32)| {
        (entry.window_class.is_empty() || win::class_of(h) == entry.window_class) && win::exe_name(pid).eq_ignore_ascii_case(exe)
    };
    if let Some((h, _)) = win::top_windows().into_iter().find(mine) {
        return Some(h);
    }
    if entry.window_class.is_empty() {
        return None;
    }
    win::hidden_windows().into_iter().find(mine).map(|(h, _)| h)
}

/// Remembers the class of the window Glaude now uses for app `index`, if it did not know it yet.
fn learn_class(app: &AppHandle, index: usize, h: isize) {
    let store = app.state::<Store>();
    let mut settings = store.get();
    if let Some(entry) = settings.apps.get_mut(index).filter(|e| e.window_class.is_empty()) {
        entry.window_class = win::class_of(h);
        let _ = store.set(settings);
    }
}

#[derive(Serialize)]
pub struct Running {
    hwnd: isize,
    title: String,
    exe: String,
}

/// Every app window open now, for "Pick a running app".
pub fn running() -> Vec<Running> {
    let me = std::process::id();
    win::top_windows()
        .into_iter()
        .filter(|&(_, pid)| pid != me)
        .map(|(h, pid)| Running { hwnd: h, title: win::title(h), exe: win::exe_path(pid) })
        .filter(|r| !r.exe.is_empty())
        .collect()
}

/// Uses this window for app `index` from the next time it is shown.
pub fn adopt(app: &AppHandle, index: usize, h: isize) {
    let overlay = app.state::<Overlay>();
    let mut st = overlay.state.lock().unwrap();
    if st.windows.len() <= index {
        st.windows.resize(index + 1, 0);
    }
    st.windows[index] = h;
}

/// Starts the program and waits for its first new window.
fn start(entry: &AppEntry) -> Result<isize, String> {
    let before: HashSet<isize> = win::top_windows().into_iter().map(|(h, _)| h).collect();
    let mut cmd = Command::new(&entry.exe);
    if !entry.args.trim().is_empty() {
        cmd.raw_arg(&entry.args);
    }
    let mut child = cmd.spawn().map_err(|e| format!("Could not start {}: {e}", entry.exe))?;
    let pid = child.id();
    let deadline = Instant::now() + START_TIMEOUT;
    let mut exited: Option<Instant> = None;
    while Instant::now() < deadline {
        sleep(Duration::from_millis(100));
        for (h, p) in win::top_windows() {
            if before.contains(&h) {
                continue;
            }
            if p == pid || win::exe_name(p).eq_ignore_ascii_case(window_exe(entry)) {
                return Ok(h);
            }
        }
        if exited.is_none() && child.try_wait().ok().flatten().is_some() {
            exited = Some(Instant::now());
        }
        // A second copy of a one-copy app (Claude, T3 Code) hands over to the running one and quits without a new
        // window; once it has been gone a moment, use the window that app already has open.
        if exited.is_some_and(|t| t.elapsed() > HANDOVER) {
            if let Some(h) = running_window(entry) {
                return Ok(h);
            }
        }
    }
    Err(format!("{} started but showed no window within {} seconds.", entry.name, START_TIMEOUT.as_secs()))
}

/// A new window can still move itself to its remembered spot once it has drawn; puts it back for its first second.
fn settle_later(app: AppHandle, h: isize) {
    std::thread::spawn(move || {
        for _ in 0..12 {
            sleep(Duration::from_millis(100));
            let overlay = app.state::<Overlay>();
            let st = overlay.state.lock().unwrap();
            let in_panel = st.open && st.taken.is_some_and(|(t, _)| t == h);
            if let (true, Some(area)) = (in_panel, st.area) {
                if win::rect(h) != area {
                    win::place(h, area);
                }
            }
        }
    });
}

/// Closes the overlay if the window in the panel goes away (you closed the app inside the panel).
fn watch(app: AppHandle, h: isize) {
    std::thread::spawn(move || loop {
        sleep(Duration::from_millis(300));
        let overlay = app.state::<Overlay>();
        let st = overlay.state.lock().unwrap();
        if !st.open || !st.taken.is_some_and(|(t, _)| t == h) {
            return;
        }
        drop(st);
        if !win::alive(h) {
            overlay.state.lock().unwrap().taken = None;
            close(&app);
            return;
        }
    });
}

pub fn close(app: &AppHandle) {
    let overlay = app.state::<Overlay>();
    let (game, taken) = {
        let mut st = overlay.state.lock().unwrap();
        if !st.open {
            return;
        }
        st.open = false;
        (st.game, st.taken.take())
    };
    if let Some((h, p)) = taken.filter(|(h, _)| win::alive(*h)) {
        win::restore(h, p, game);
    }
    if win::alive(game) {
        win::focus(game);
    }
    win::hide(shade_hwnd(app));
}

/// Closes the overlay and forgets every app's window: on quit, or when the list of apps changes.
pub fn release(app: &AppHandle) {
    close(app);
    app.state::<Overlay>().state.lock().unwrap().windows.clear();
}

pub fn set_dim(app: &AppHandle, dim: f64) {
    let _ = app.emit_to(SHADE, "shade:dim", dim);
}
