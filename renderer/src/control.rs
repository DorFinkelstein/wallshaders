//! How the app talks to a running wallpaper renderer: window messages sent to
//! the renderer's hidden control window. Settings travel through `config.json`.

use crate::win::wide;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_APP, WM_CLOSE};

pub const CONTROL_CLASS: &str = "WallshadersControl";
pub const WM_WS_RELOAD: u32 = WM_APP + 1;

fn control_window() -> HWND {
    unsafe { FindWindowW(wide(CONTROL_CLASS).as_ptr(), std::ptr::null()) }
}

pub fn is_running() -> bool {
    !control_window().is_null()
}

/// Ask the renderer to re-read `config.json` and the active script.
pub fn notify_reload() -> bool {
    let hwnd = control_window();
    !hwnd.is_null() && unsafe { PostMessageW(hwnd, WM_WS_RELOAD, 0, 0) != 0 }
}

/// Ask the renderer to exit, which restores the normal wallpaper.
pub fn quit() -> bool {
    let hwnd = control_window();
    !hwnd.is_null() && unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0 }
}
