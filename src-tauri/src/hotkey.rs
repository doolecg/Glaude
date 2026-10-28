//! The global hotkey, registered with RegisterHotKey on its own thread.
//! Keys are stored by physical position (the browser's `KeyboardEvent.code`) and turned into the key code of the
//! keyboard layout in use, so Ctrl+` means the key left of 1 on a UK keyboard as well as a US one.

use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use tauri::AppHandle;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MAPVK_VSC_TO_VK_EX, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_APP, WM_HOTKEY};

const ID: i32 = 1;

type Request = (Option<String>, Sender<Result<(), String>>);

/// Talks to the hotkey thread: each request replaces the registered key (None = let go of it).
pub struct Hotkey {
    thread: u32,
    requests: Mutex<Sender<Request>>,
}

impl Hotkey {
    pub fn start(app: AppHandle) -> Hotkey {
        let (tx, rx) = channel::<Request>();
        let (id_tx, id_rx) = channel();
        std::thread::spawn(move || unsafe {
            let mut msg = MSG::default();
            // Makes the thread's message queue before anyone posts to it.
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            let _ = id_tx.send(GetCurrentThreadId());
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    crate::overlay::toggle(&app);
                } else if msg.message == WM_APP {
                    while let Ok((accel, reply)) = rx.try_recv() {
                        let _ = UnregisterHotKey(None, ID);
                        let _ = reply.send(accel.map_or(Ok(()), |a| register(&a)));
                    }
                }
            }
        });
        Hotkey { thread: id_rx.recv().unwrap_or(0), requests: Mutex::new(tx) }
    }

    /// Registers `accel` in place of the current hotkey, or lets go of it with None.
    pub fn set(&self, accel: Option<&str>) -> Result<(), String> {
        let (reply, answer) = channel();
        self.requests.lock().unwrap().send((accel.map(str::to_string), reply)).map_err(|e| e.to_string())?;
        unsafe { PostThreadMessageW(self.thread, WM_APP, Default::default(), Default::default()) }.map_err(|e| e.to_string())?;
        answer.recv().map_err(|e| e.to_string())?
    }
}

fn register(accel: &str) -> Result<(), String> {
    let mut mods = MOD_NOREPEAT;
    let mut key = None;
    for part in accel.split('+') {
        match part {
            "Ctrl" => mods |= MOD_CONTROL,
            "Alt" => mods |= MOD_ALT,
            "Shift" => mods |= MOD_SHIFT,
            "Super" => mods |= MOD_WIN,
            code => key = vk(code),
        }
    }
    let vk = key.ok_or_else(|| format!("{accel} is not a key combination Glaude knows."))?;
    unsafe { RegisterHotKey(None, ID, HOT_KEY_MODIFIERS(mods.0), vk) }.map_err(|_| format!("{accel} is taken by another program. Pick another hotkey."))
}

/// Key code for a `KeyboardEvent.code` on the current layout.
fn vk(code: &str) -> Option<u32> {
    if let Some(n) = code.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(0x70 + n - 1);
    }
    let scan = scan_code(code)?;
    match unsafe { MapVirtualKeyW(scan, MAPVK_VSC_TO_VK_EX) } {
        0 => None,
        vk => Some(vk),
    }
}

/// Scan code of a key by its position (scan code set 1), which is the same on every layout.
fn scan_code(code: &str) -> Option<u32> {
    if let Some(c) = code.strip_prefix("Key").and_then(|k| k.chars().next()) {
        for (row, first) in [("QWERTYUIOP", 0x10), ("ASDFGHJKL", 0x1E), ("ZXCVBNM", 0x2C)] {
            if let Some(i) = row.find(c) {
                return Some(first + i as u32);
            }
        }
        return None;
    }
    if let Some(d) = code.strip_prefix("Digit").and_then(|d| d.parse::<u32>().ok()) {
        return Some(if d == 0 { 0x0B } else { 0x01 + d });
    }
    Some(match code {
        "Backquote" => 0x29,
        "Minus" => 0x0C,
        "Equal" => 0x0D,
        "Tab" => 0x0F,
        "BracketLeft" => 0x1A,
        "BracketRight" => 0x1B,
        "Semicolon" => 0x27,
        "Quote" => 0x28,
        "Backslash" => 0x2B,
        "Comma" => 0x33,
        "Period" => 0x34,
        "Slash" => 0x35,
        "Space" => 0x39,
        "IntlBackslash" => 0x56,
        _ => return None,
    })
}
