//! The few Win32 calls the overlay needs. Window handles travel as `isize` so they can cross threads.

use windows::core::{BOOL, PCWSTR, PWSTR};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmFlush, DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowPlacement, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, SetForegroundWindow, SetWindowPlacement, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, GW_OWNER, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_MINIMIZE,
    SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, SWP_NOZORDER, WINDOWPLACEMENT, WS_EX_TOOLWINDOW,
};

/// Windows of the shell itself (the desktop and the taskbars), which belong to explorer.exe but are never an app.
const SHELL_CLASSES: &[&str] = &["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

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

/// Visible, titled, unowned top-level windows with their process ids, front to back: the windows a program opens for the user.
pub fn top_windows() -> Vec<(isize, u32)> {
    windows_where(true)
}

/// Titled, unowned top-level windows that are hidden, such as an app closed to the tray. Most belong to helpers, so
/// callers match them by exe and window class.
pub fn hidden_windows() -> Vec<(isize, u32)> {
    windows_where(false)
}

fn windows_where(visible: bool) -> Vec<(isize, u32)> {
    struct Walk {
        visible: bool,
        out: Vec<(isize, u32)>,
    }
    unsafe extern "system" fn each(h: HWND, walk: LPARAM) -> BOOL {
        let walk = &mut *(walk.0 as *mut Walk);
        let owned = GetWindow(h, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        let tool = GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0;
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        let shell = SHELL_CLASSES.contains(&class_of(h.0 as isize).as_str());
        // Cloaked windows are "visible" but not drawn: other virtual desktops and suspended Store apps.
        let shown = IsWindowVisible(h).as_bool() && cloaked == 0;
        if shown == walk.visible && !owned && !tool && !shell && GetWindowTextLengthW(h) > 0 {
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            walk.out.push((h.0 as isize, pid));
        }
        true.into()
    }
    let mut walk = Walk { visible, out: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut walk as *mut _ as isize));
    }
    walk.out
}

pub fn class_of(h: isize) -> String {
    let mut class = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd(h), &mut class) };
    String::from_utf16_lossy(&class[..n.max(0) as usize])
}

pub fn visible(h: isize) -> bool {
    unsafe { IsWindowVisible(hwnd(h)).as_bool() }
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

/// Moves an app window to `r` without changing its place in the stack, so it stays under the shade (whose panel
/// is solid) while it redraws at the new size. A maximised or minimised window is made normal through its
/// placement, which skips Windows' restore animation; a hidden window is shown.
pub fn place(h: isize, r: Rect) {
    unsafe {
        if IsZoomed(hwnd(h)).as_bool() || IsIconic(hwnd(h)).as_bool() {
            let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
            let _ = GetWindowPlacement(hwnd(h), &mut wp);
            wp.showCmd = SW_SHOWNOACTIVATE.0 as u32;
            wp.rcNormalPosition = RECT { left: r.x, top: r.y, right: r.x + r.w, bottom: r.y + r.h };
            let _ = SetWindowPlacement(hwnd(h), &wp);
        }
        let _ = SetWindowPos(hwnd(h), None, r.x, r.y, r.w, r.h, SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

/// Lifts a window `place` put under the shade above it, and gives it the keyboard.
pub fn raise(h: isize) {
    unsafe {
        let _ = SetWindowPos(hwnd(h), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    focus(h);
}

/// Waits for the screen to be redrawn `n` times, giving an app time to paint after a resize.
pub fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe {
            let _ = DwmFlush();
        }
    }
}

/// Where a window sits on the desktop: position, size, whether it is maximised or minimised, and whether it was
/// hidden (an app closed to the tray).
#[derive(Clone, Copy)]
pub struct Placement {
    wp: WINDOWPLACEMENT,
    shown: bool,
}

pub fn placement(h: isize) -> Placement {
    let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
    unsafe {
        let _ = GetWindowPlacement(hwnd(h), &mut wp);
    }
    Placement { wp, shown: visible(h) }
}

/// Puts a window back where `p` says it was, as a normal desktop window just below `below` (the game), or at the
/// top of the normal windows when there is no game; a window that was hidden is hidden again. It drops behind the
/// game before it moves, so the move happens out of sight, and it is not activated, so the game keeps the keyboard.
pub fn restore(h: isize, p: Placement, below: isize) {
    let mut wp = p.wp;
    wp.showCmd = if !p.shown {
        SW_HIDE.0 as u32
    } else if wp.showCmd == SW_SHOWMAXIMIZED.0 as u32 {
        SW_SHOWMAXIMIZED.0 as u32
    } else if [SW_SHOWMINIMIZED.0, SW_MINIMIZE.0, SW_SHOWMINNOACTIVE.0].contains(&(wp.showCmd as i32)) {
        SW_SHOWMINNOACTIVE.0 as u32
    } else {
        SW_SHOWNOACTIVATE.0 as u32
    };
    let drop = || unsafe {
        if alive(below) {
            let _ = SetWindowPos(hwnd(h), Some(hwnd(below)), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    };
    unsafe {
        // Off the top layer first: it lands under the shade's solid panel, then behind the game.
        let _ = SetWindowPos(hwnd(h), Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        drop();
        let _ = SetWindowPlacement(hwnd(h), &wp);
    }
    // Maximising always activates the window, which lifts it above the game; put it back behind.
    drop();
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
