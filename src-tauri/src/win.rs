//! The few Win32 calls the overlay needs. Window handles travel as `isize` so they can cross threads.

use windows::core::{BOOL, PCWSTR, PWSTR};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, IsZoomed, SetForegroundWindow, SetWindowPos, ShowWindow, GWL_EXSTYLE, GW_OWNER, HWND_NOTOPMOST, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOWNA, WS_EX_TOOLWINDOW,
};

/// Off-screen spot an app window waits in between opens (Notifi's PARKED).
const PARKED: i32 = -30_000;

#[derive(Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn inset(self, by: i32) -> Rect {
        Rect { x: self.x + by, y: self.y + by, w: self.w - 2 * by, h: self.h - 2 * by }
    }
}

fn hwnd(h: isize) -> HWND {
    HWND(h as *mut _)
}

pub fn foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

pub fn alive(h: isize) -> bool {
    h != 0 && unsafe { IsWindow(Some(hwnd(h))).as_bool() }
}

/// The monitor a window is on, in physical pixels, and its display scale (1.0 = 96 dpi).
pub fn monitor_of(h: isize) -> (Rect, f64) {
    unsafe {
        let mon = if h != 0 {
            MonitorFromWindow(hwnd(h), MONITOR_DEFAULTTONEAREST)
        } else {
            MonitorFromPoint(Default::default(), MONITOR_DEFAULTTOPRIMARY)
        };
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        let r: RECT = mi.rcMonitor;
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        (Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }, dx as f64 / 96.0)
    }
}

pub fn rect(h: isize) -> Rect {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd(h), &mut r);
    }
    Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
}

/// Visible, titled, unowned top-level windows with their process ids: the windows a program opens for the user.
pub fn top_windows() -> Vec<(isize, u32)> {
    unsafe extern "system" fn each(h: HWND, out: LPARAM) -> BOOL {
        let out = &mut *(out.0 as *mut Vec<(isize, u32)>);
        let owned = GetWindow(h, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        let tool = GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0;
        if IsWindowVisible(h).as_bool() && !owned && !tool && GetWindowTextLengthW(h) > 0 {
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            out.push((h.0 as isize, pid));
        }
        true.into()
    }
    let mut out: Vec<(isize, u32)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

/// Full path of a process's exe.
pub fn exe_path(pid: u32) -> String {
    unsafe {
        let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(p);
        if !ok {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// File name of a process's exe, e.g. `WindowsTerminal.exe`.
pub fn exe_name(pid: u32) -> String {
    file_name(&exe_path(pid)).to_string()
}

pub fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or_default()
}

pub fn title(h: isize) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd(h), &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Asks for a program with the Windows open dialog, owned by `owner`. Shortcuts resolve to the program they start.
pub fn pick_exe(owner: isize) -> Option<String> {
    let filter: Vec<u16> = "Programs\0*.exe;*.lnk\0All files\0*.*\0\0".encode_utf16().collect();
    let mut buf = [0u16; 1024];
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd(owner),
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    unsafe {
        // The dialog needs a single-threaded COM apartment on the thread that shows it.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if !GetOpenFileNameW(&mut ofn).as_bool() {
            return None;
        }
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Shows a window over everything at `r` without activating it.
pub fn show_topmost(h: isize, r: Rect) {
    unsafe {
        let _ = SetWindowPos(hwnd(h), Some(HWND_TOPMOST), r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

/// Hides a window `show_topmost` showed. Tao never saw that show, so its own `hide` would do nothing.
pub fn hide(h: isize) {
    unsafe {
        let _ = ShowWindow(hwnd(h), SW_HIDE);
    }
}

/// Puts an app window over the shade at `r` and gives it the keyboard.
pub fn show_in(h: isize, r: Rect) {
    unsafe {
        if IsZoomed(hwnd(h)).as_bool() || IsIconic(hwnd(h)).as_bool() {
            let _ = ShowWindow(hwnd(h), SW_RESTORE);
        }
        show_topmost(h, r);
        let _ = ShowWindow(hwnd(h), SW_SHOWNA);
    }
    focus(h);
}

/// Moves an app window off-screen and hides it, so it keeps running but leaves no taskbar button.
pub fn park(h: isize) {
    unsafe {
        let _ = SetWindowPos(hwnd(h), Some(HWND_NOTOPMOST), PARKED, PARKED, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd(h), SW_HIDE);
    }
}

/// Gives a parked window back as a normal window in the middle of the primary monitor (on quit, or when the app is changed).
pub fn unpark(h: isize) {
    let (m, _) = monitor_of(0);
    let (w, hh) = (m.w * 3 / 5, m.h * 3 / 5);
    unsafe {
        let _ = SetWindowPos(hwnd(h), Some(HWND_NOTOPMOST), m.x + (m.w - w) / 2, m.y + (m.h - hh) / 2, w, hh, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

/// Makes a window the foreground window. Glaude may do this straight after its hotkey or a click on the shade;
/// if Windows still refuses, joining the input queue of the window in front lets it through.
pub fn focus(h: isize) {
    unsafe {
        if SetForegroundWindow(hwnd(h)).as_bool() {
            return;
        }
        let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let me = GetCurrentThreadId();
        let joined = front != 0 && front != me && AttachThreadInput(me, front, true).as_bool();
        let _ = BringWindowToTop(hwnd(h));
        let _ = SetForegroundWindow(hwnd(h));
        if joined {
            let _ = AttachThreadInput(me, front, false);
        }
    }
}
