//! How the app talks to a running wallpaper renderer: window messages sent to
//! the renderer's hidden control window. Settings travel through `config.json`.

use crate::win::wide;
use windows_sys::Win32::Foundation::{CloseHandle, HWND, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId, PostMessageW, WM_APP, WM_CLOSE};

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

/// Ask the renderer to exit and wait until its process is gone (so its program
/// file can be replaced). Returns true when no renderer is left running.
pub fn quit_and_wait(timeout_ms: u32) -> bool {
    let hwnd = control_window();
    if hwnd.is_null() {
        return true;
    }
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = if pid != 0 { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) } else { std::ptr::null_mut() };
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
        if process.is_null() {
            // Cannot watch the process; fall back to watching its window.
            for _ in 0..timeout_ms / 100 {
                if !is_running() {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            return !is_running();
        }
        let done = WaitForSingleObject(process, timeout_ms) == WAIT_OBJECT_0;
        CloseHandle(process);
        done
    }
}
