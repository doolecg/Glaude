#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hotkey;
mod overlay;
mod settings;
mod updater;
mod win;

use hotkey::Hotkey;
use settings::{AppEntry, Settings, Store};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Why the hotkey could not be registered, shown on the settings page.
#[derive(Default)]
struct HotkeyError(Mutex<Option<String>>);

fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Glaude settings")
        .inner_size(560.0, 680.0)
        .resizable(true)
        .build();
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    settings: Settings,
    hotkey_error: Option<String>,
}

#[tauri::command]
fn settings_get(store: State<Store>, error: State<HotkeyError>) -> SettingsView {
    SettingsView { settings: store.get(), hotkey_error: error.0.lock().unwrap().clone() }
}

/// Saves and applies a change. A hotkey that cannot be registered is refused and the old one stays.
fn save(app: &AppHandle, settings: Settings) -> Result<(), String> {
    let store = app.state::<Store>();
    let old = store.get();
    if settings.hotkey != old.hotkey {
        let hotkey = app.state::<Hotkey>();
        if let Err(e) = hotkey.set(Some(&settings.hotkey)) {
            let _ = hotkey.set(Some(&old.hotkey));
            return Err(e);
        }
        *app.state::<HotkeyError>().0.lock().unwrap() = None;
    }
    if settings.apps != old.apps {
        overlay::release(app);
    }
    overlay::set_dim(app, settings.dim);
    store.set(settings)
}

#[tauri::command]
fn settings_set(app: AppHandle, settings: Settings) -> Result<(), String> {
    save(&app, settings)
}

/// Lets go of the hotkey while the settings page records a new one, so pressing the current one does not open the overlay.
#[tauri::command]
fn hotkey_pause(paused: bool, store: State<Store>, hotkey: State<Hotkey>) {
    let accel = store.get().hotkey;
    let _ = hotkey.set(if paused { None } else { Some(&accel) });
}

#[tauri::command]
fn apps_running() -> Vec<overlay::Running> {
    overlay::running()
}

/// Adds the program in this window to the apps, using the window itself.
#[tauri::command]
fn app_attach(app: AppHandle, hwnd: isize) -> Result<Settings, String> {
    if !win::alive(hwnd) {
        return Err("That window has closed.".into());
    }
    let mut settings = app.state::<Store>().get();
    settings.apps.push(AppEntry { window_class: win::class_of(hwnd), ..entry_for(&win::exe_path(pid_of(hwnd))) });
    save(&app, settings.clone())?;
    overlay::adopt(&app, settings.apps.len() - 1, hwnd);
    Ok(settings)
}

/// Shows the Windows open dialog and adds the chosen program to the apps.
#[tauri::command]
async fn app_browse(app: AppHandle, window: WebviewWindow) -> Result<Option<Settings>, String> {
    let owner = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let Some(path) = tauri::async_runtime::spawn_blocking(move || win::pick_exe(owner)).await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let mut settings = app.state::<Store>().get();
    settings.apps.push(entry_for(&path));
    save(&app, settings.clone())?;
    Ok(Some(settings))
}

fn pid_of(h: isize) -> u32 {
    win::top_windows().into_iter().find(|&(w, _)| w == h).map(|(_, p)| p).unwrap_or(0)
}

/// An app that runs this program with no arguments and takes its window from the same exe.
fn entry_for(path: &str) -> AppEntry {
    let file = win::file_name(path).to_string();
    let name = file.strip_suffix(".exe").or(file.strip_suffix(".EXE")).unwrap_or(&file).to_string();
    AppEntry { name, exe: path.to_string(), args: String::new(), window_exe: file, window_class: String::new() }
}

#[tauri::command]
fn overlay_close(app: AppHandle) {
    std::thread::spawn(move || overlay::close(&app));
}

#[tauri::command]
fn overlay_switch(app: AppHandle, index: usize) {
    overlay::switch(&app, index);
}

/// The tray menu's update line, enabled only while an update is downloaded and waiting.
struct UpdateItem(MenuItem<tauri::Wry>);

/// Shows in the tray that an update is ready to install, or that none is.
pub fn tray_update_ready(app: &AppHandle, version: Option<&str>) {
    if let Some(item) = app.try_state::<UpdateItem>() {
        let _ = item.0.set_text(version.map_or("No update waiting".to_string(), |v| format!("Install update {v}")));
        let _ = item.0.set_enabled(version.is_some());
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(version.map_or("Glaude".to_string(), |v| format!("Glaude: update {v} ready"))));
    }
}

fn tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open overlay", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", "No update waiting", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Glaude", true, None::<&str>)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Glaude")
        .menu(&Menu::with_items(app, &[&open, &settings, &update, &quit])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => overlay::toggle(app),
            "settings" => open_settings(app),
            "update" => updater::update_install(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                open_settings(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(UpdateItem(update));
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            settings_get,
            settings_set,
            hotkey_pause,
            apps_running,
            app_attach,
            app_browse,
            overlay_close,
            overlay_switch,
            updater::update_status,
            updater::update_check,
            updater::update_install,
            updater::update_notes
        ])
        .setup(|app| {
            let store = Store::load(app.handle());
            let accel = store.get().hotkey;
            app.manage(store);
            app.manage(overlay::Overlay::default());
            let hotkey = Hotkey::start(app.handle().clone());
            app.manage(HotkeyError(Mutex::new(hotkey.set(Some(&accel)).err())));
            app.manage(hotkey);
            overlay::create_shade(app.handle())?;
            app.manage(updater::Updater::new(app.package_info().version.to_string()));
            tray(app.handle())?;
            updater::start(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Glaude failed to start")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                overlay::release(app);
                // An update downloaded but not yet installed goes in now, as Glaude quits.
                let _ = updater::install(app);
            }
        });
}
