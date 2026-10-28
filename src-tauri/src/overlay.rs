//! The overlay: the shade (Glaude's own dim window over the game's monitor) and the app's real window placed above it.

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
const FADE_OUT_MS: u64 = 100;
const START_TIMEOUT: Duration = Duration::from_secs(15);
/// How long after the started process quits to keep waiting for a new window before taking an existing one.
const HANDOVER: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Overlay {
    state: Mutex<State>,
    /// Set while an open or close is running, so a second hotkey press in that time is ignored.
    busy: AtomicBool,
}

#[derive(Default)]
struct State {
    open: bool,
    game: isize,
    app: isize,
}

#[derive(Clone, Serialize)]
struct Open {
    dim: f64,
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
    Ok(())
}

fn shade_hwnd(app: &AppHandle) -> isize {
    app.get_webview_window(SHADE).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize).unwrap_or(0)
}

pub fn toggle(app: &AppHandle) {
    let overlay = app.state::<Overlay>();
    if overlay.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let open = overlay.state.lock().unwrap().open;
    let app = app.clone();
    std::thread::spawn(move || {
        if open {
            close(&app);
        } else {
            show(&app);
        }
        app.state::<Overlay>().busy.store(false, Ordering::SeqCst);
    });
}

fn show(app: &AppHandle) {
    let settings = app.state::<Store>().get();
    let overlay = app.state::<Overlay>();
    let shade = shade_hwnd(app);
    let front = win::foreground();
    let known = {
        let mut st = overlay.state.lock().unwrap();
        st.open = true;
        st.game = if front == shade || front == st.app { 0 } else { front };
        st.app
    };
    let (monitor, scale) = win::monitor_of(front);
    let starting = !win::alive(known);
    let note = if starting { format!("Starting {}…", settings.app.name) } else { String::new() };
    let _ = app.emit_to(SHADE, "shade:open", Open { dim: settings.dim, note });
    // Twice: moving onto a monitor with a different scale makes the window resize itself once on arrival.
    win::show_topmost(shade, monitor);
    win::show_topmost(shade, monitor);

    let found = if starting { start(&settings.app) } else { Ok(known) };
    let h = match found {
        Ok(h) => h,
        Err(e) => {
            let _ = app.emit_to(SHADE, "shade:open", Open { dim: settings.dim, note: e });
            return;
        }
    };
    {
        let mut st = overlay.state.lock().unwrap();
        st.app = h;
        if !st.open {
            win::park(h);
            return;
        }
    }
    let panel = monitor.inset((MARGIN * scale).round() as i32);
    win::show_in(h, panel);
    if starting {
        settle(app.clone(), h, panel);
    }
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

/// A window the app already has open.
fn running_window(entry: &AppEntry) -> Option<isize> {
    let exe = window_exe(entry);
    win::top_windows().into_iter().find(|&(_, pid)| win::exe_name(pid).eq_ignore_ascii_case(exe)).map(|(h, _)| h)
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

/// Uses this window for the app from the next open; it stays where it is until then.
pub fn adopt(app: &AppHandle, h: isize) {
    app.state::<Overlay>().state.lock().unwrap().app = h;
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
fn settle(app: AppHandle, h: isize, panel: win::Rect) {
    std::thread::spawn(move || {
        for _ in 0..10 {
            sleep(Duration::from_millis(100));
            let overlay = app.state::<Overlay>();
            let st = overlay.state.lock().unwrap();
            if !st.open || st.app != h {
                return;
            }
            if win::rect(h) != panel {
                win::show_in(h, panel);
            }
        }
    });
}

/// Closes the overlay if the app window goes away while it is open (you closed the app inside the panel).
fn watch(app: AppHandle, h: isize) {
    std::thread::spawn(move || loop {
        sleep(Duration::from_millis(300));
        let overlay = app.state::<Overlay>();
        let st = overlay.state.lock().unwrap();
        if !st.open || st.app != h {
            return;
        }
        drop(st);
        if !win::alive(h) {
            close(&app);
            return;
        }
    });
}

pub fn close(app: &AppHandle) {
    let overlay = app.state::<Overlay>();
    let (game, h) = {
        let mut st = overlay.state.lock().unwrap();
        if !st.open {
            return;
        }
        st.open = false;
        (st.game, st.app)
    };
    if win::alive(h) {
        win::park(h);
    }
    if win::alive(game) {
        win::focus(game);
    }
    let _ = app.emit_to(SHADE, "shade:close", ());
    sleep(Duration::from_millis(FADE_OUT_MS));
    if !overlay.state.lock().unwrap().open {
        win::hide(shade_hwnd(app));
    }
}

/// Lets go of the app window, putting it back on screen as a normal window: on quit, or when a different app is chosen.
pub fn release(app: &AppHandle) {
    close(app);
    let h = std::mem::take(&mut app.state::<Overlay>().state.lock().unwrap().app);
    if win::alive(h) {
        win::unpark(h);
    }
}

pub fn set_dim(app: &AppHandle, dim: f64) {
    let _ = app.emit_to(SHADE, "shade:dim", dim);
}
