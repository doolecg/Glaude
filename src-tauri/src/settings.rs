use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// A program the overlay can show.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppEntry {
    pub name: String,
    pub exe: String,
    /// Passed to the program as written, so quoted paths work.
    pub args: String,
    /// For launchers that hand the window to another process (wt.exe starts WindowsTerminal.exe): that process's exe name.
    pub window_exe: String,
    /// Window class of the app's main window, learned the first time Glaude uses it, so a hidden main window (an app
    /// closed to the tray) can be told apart from the app's hidden helper windows.
    pub window_class: String,
}

impl Default for AppEntry {
    fn default() -> Self {
        AppEntry { name: "Terminal".into(), exe: "wt.exe".into(), args: "-w new".into(), window_exe: "WindowsTerminal.exe".into(), window_class: String::new() }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    /// How dark the shade is over the game, 0 to 0.95.
    pub dim: f64,
    /// The apps on the rail, in order.
    pub apps: Vec<AppEntry>,
    /// The single app of version 0.1.0 settings files, moved into `apps` on load.
    #[serde(skip_serializing)]
    app: Option<AppEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { hotkey: "Ctrl+Backquote".into(), dim: 0.72, apps: vec![AppEntry::default()], app: None }
    }
}

pub struct Store {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl Store {
    pub fn load(app: &AppHandle) -> Store {
        let path = app.path().app_config_dir().unwrap_or_default().join("settings.json");
        let mut current: Settings =
            std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok()).unwrap_or_default();
        if let Some(old) = current.app.take() {
            current.apps = vec![old];
        }
        Store { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn set(&self, s: Settings) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&s).unwrap()).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = s;
        Ok(())
    }
}
