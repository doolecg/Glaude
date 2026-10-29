//! Updates from Glaude's GitHub releases: checked at start and every few hours, downloaded in the background,
//! installed when you press Install or when Glaude quits. The installer is signed with Glaude's update key and the
//! updater plugin refuses one whose signature does not match.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const STATUS_EVENT: &str = "updater://status";
const FIRST_CHECK: Duration = Duration::from_secs(20);
const EVERY: Duration = Duration::from_secs(4 * 60 * 60);
/// The notes of every version, newest first; this version's section is shown when there is no newer release.
const OWN_NOTES: &str = include_str!("../../RELEASE_NOTES.md");

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Downloading { version: String, percent: u8 },
    Ready { version: String },
    Error { message: String },
}

#[derive(Clone, Serialize)]
pub struct Notes {
    version: String,
    markdown: String,
}

pub struct Updater {
    status: Mutex<Status>,
    /// The downloaded, signature-checked update waiting to be installed.
    ready: Mutex<Option<(Update, Vec<u8>)>>,
    notes: Mutex<Notes>,
    busy: AtomicBool,
}

impl Updater {
    pub fn new(version: String) -> Updater {
        // The top section runs to the next version's "# Glaude" heading.
        let markdown = OWN_NOTES.split("\n# ").next().unwrap_or_default().trim().to_string();
        Updater { status: Mutex::new(Status::Idle), ready: Mutex::new(None), notes: Mutex::new(Notes { version, markdown }), busy: AtomicBool::new(false) }
    }
}

fn set_status(app: &AppHandle, status: Status) {
    *app.state::<Updater>().status.lock().unwrap() = status.clone();
    let _ = app.emit(STATUS_EVENT, &status);
    crate::tray_update_ready(app, match &status {
        Status::Ready { version } => Some(version.as_str()),
        _ => None,
    });
}

async fn run_check(app: &AppHandle) -> Status {
    let mut builder = app.updater_builder();
    // Points one run at another release feed, for testing an update before it is published.
    if let Some(url) = std::env::var("GLAUDE_UPDATE_URL").ok().and_then(|u| u.parse().ok()) {
        builder = match builder.endpoints(vec![url]) {
            Ok(b) => b,
            Err(e) => return Status::Error { message: e.to_string() },
        };
    }
    let update = match builder.build() {
        Ok(updater) => updater.check().await,
        Err(e) => return Status::Error { message: e.to_string() },
    };
    let update = match update {
        Ok(Some(update)) => update,
        Ok(None) => return Status::UpToDate,
        Err(e) => return Status::Error { message: format!("Could not check for updates: {e}") },
    };
    let version = update.version.clone();
    *app.state::<Updater>().notes.lock().unwrap() = Notes { version: version.clone(), markdown: update.body.clone().unwrap_or_default() };
    let (mut received, mut last) = (0u64, None);
    let downloaded = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let percent = total.filter(|t| *t > 0).map(|t| (received * 100 / t).min(100) as u8).unwrap_or(0);
                if last != Some(percent) {
                    last = Some(percent);
                    set_status(app, Status::Downloading { version: version.clone(), percent });
                }
            },
            || {},
        )
        .await;
    match downloaded {
        Ok(bytes) => {
            *app.state::<Updater>().ready.lock().unwrap() = Some((update, bytes));
            Status::Ready { version }
        }
        Err(e) => Status::Error { message: format!("The update did not download: {e}") },
    }
}

async fn check(app: &AppHandle) {
    let updater = app.state::<Updater>();
    if updater.ready.lock().unwrap().is_some() || updater.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    set_status(app, Status::Checking);
    let status = run_check(app).await;
    set_status(app, status);
    updater.busy.store(false, Ordering::SeqCst);
}

/// Checks shortly after start, then every few hours.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            tauri::async_runtime::block_on(check(&app));
            std::thread::sleep(EVERY);
        }
    });
}

/// Runs the waiting installer, if there is one. Windows' installer closes Glaude, replaces it and starts it again.
pub fn install(app: &AppHandle) -> Result<bool, String> {
    let Some((update, bytes)) = app.state::<Updater>().ready.lock().unwrap().take() else { return Ok(false) };
    update.install(bytes).map_err(|e| format!("The update did not install: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub fn update_status(updater: State<Updater>) -> Status {
    updater.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn update_check(app: AppHandle) {
    std::thread::spawn(move || tauri::async_runtime::block_on(check(&app)));
}

#[tauri::command]
pub fn update_install(app: AppHandle) {
    crate::overlay::release(&app);
    if let Err(message) = install(&app) {
        set_status(&app, Status::Error { message });
    }
}

#[tauri::command]
pub fn update_notes(updater: State<Updater>) -> Notes {
    updater.notes.lock().unwrap().clone()
}
