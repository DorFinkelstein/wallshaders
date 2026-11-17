//! The screens attached to this PC, with readable names and ids that stay the
//! same across restarts (so a per-screen choice sticks to the right screen).

use serde::Serialize;
use windows_sys::Win32::Devices::Display::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

#[derive(Serialize, Clone, Debug)]
pub struct Monitor {
    /// Stable id: the monitor's device path, or its GDI name when that is unknown.
    pub id: String,
    /// The monitor's own name, such as "LG ULTRAGEAR", or "Display 2".
    pub name: String,
    /// Position and size in physical pixels, in virtual-screen coordinates.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
    /// The HMONITOR, valid until the display layout changes.
    #[serde(skip)]
    pub handle: isize,
}

fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

/// GDI device name (`\\.\DISPLAY1`) → (friendly name, device path), for active displays.
unsafe fn display_names() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let (mut np, mut nm) = (0u32, 0u32);
    if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != 0 {
        return out;
    }
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = vec![std::mem::zeroed(); np as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![std::mem::zeroed(); nm as usize];
    if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), std::ptr::null_mut()) != 0 {
        return out;
    }
    for p in &paths[..np as usize] {
        let mut src: DISPLAYCONFIG_SOURCE_DEVICE_NAME = std::mem::zeroed();
        src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        src.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        src.header.adapterId = p.sourceInfo.adapterId;
        src.header.id = p.sourceInfo.id;
        if DisplayConfigGetDeviceInfo(&mut src.header) != 0 {
            continue;
        }
        let mut tgt: DISPLAYCONFIG_TARGET_DEVICE_NAME = std::mem::zeroed();
        tgt.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        tgt.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        tgt.header.adapterId = p.targetInfo.adapterId;
        tgt.header.id = p.targetInfo.id;
        let (friendly, path) = if DisplayConfigGetDeviceInfo(&mut tgt.header) == 0 {
            (from_wide(&tgt.monitorFriendlyDeviceName), from_wide(&tgt.monitorDevicePath))
        } else {
            (String::new(), String::new())
        };
        out.push((from_wide(&src.viewGdiDeviceName), friendly, path));
    }
    out
}

unsafe extern "system" fn collect(mon: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
    let list = &mut *(lp as *mut Vec<(HMONITOR, MONITORINFOEXW)>);
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(mon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) != 0 {
        list.push((mon, info));
    }
    1
}

/// All screens, left to right (then top to bottom), in physical pixels whatever
/// the calling process's DPI mode.
pub fn list() -> Vec<Monitor> {
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let out = list_inner();
        if !previous.is_null() {
            SetThreadDpiAwarenessContext(previous);
        }
        out
    }
}

unsafe fn list_inner() -> Vec<Monitor> {
    {
        let names = display_names();
        let mut raw: Vec<(HMONITOR, MONITORINFOEXW)> = Vec::new();
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect), &mut raw as *mut _ as LPARAM);
        let mut out: Vec<Monitor> = raw
            .iter()
            .map(|(handle, info)| {
                let gdi = from_wide(&info.szDevice);
                let found = names.iter().find(|(g, _, _)| g.eq_ignore_ascii_case(&gdi));
                let r = info.monitorInfo.rcMonitor;
                let number = gdi.trim_start_matches(|c: char| !c.is_ascii_digit()).to_string();
                let name = match found {
                    Some((_, friendly, _)) if !friendly.trim().is_empty() => friendly.trim().to_string(),
                    _ if !number.is_empty() => format!("Display {number}"),
                    _ => "Display".to_string(),
                };
                let id = match found {
                    Some((_, _, path)) if !path.is_empty() => path.to_lowercase(),
                    _ => gdi.to_lowercase(),
                };
                Monitor {
                    id,
                    name,
                    x: r.left,
                    y: r.top,
                    width: r.right - r.left,
                    height: r.bottom - r.top,
                    primary: info.monitorInfo.dwFlags & 1 != 0,
                    handle: *handle as isize,
                }
            })
            .collect();
        out.sort_by_key(|m| (m.x, m.y));
        out
    }
}
