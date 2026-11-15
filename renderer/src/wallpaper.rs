//! The live wallpaper: a borderless window placed behind the desktop icons,
//! a render loop with a frame-rate cap, and the rules that pause drawing.

use crate::config::{self, log, Config};
use crate::control::{self, CONTROL_CLASS, WM_WS_RELOAD};
use crate::glctx::{Frame, GlContext, Pipeline};
use crate::shader;
use crate::win::{class_name, wide};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::time::{Duration, Instant};
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::*;
use windows_sys::Win32::System::RemoteDesktop::WTSRegisterSessionNotification;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WALL_CLASS: &str = "WallshadersWallpaper";
const WM_WS_TRAY: u32 = WM_APP + 2;
const MENU_OPEN: usize = 1;
const MENU_PAUSE: usize = 2;
const MENU_QUIT: usize = 3;

/// GUID_CONSOLE_DISPLAY_STATE: tells us when the screen turns off.
const DISPLAY_STATE: GUID = GUID {
    data1: 0x6fe69556,
    data2: 0x704a,
    data3: 0x47a0,
    data4: [0x8f, 0x24, 0xc2, 0x8d, 0x93, 0x6f, 0xda, 0x47],
};

static RELOAD: AtomicBool = AtomicBool::new(false);
static QUIT: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static DISPLAY_OFF: AtomicBool = AtomicBool::new(false);
static USER_PAUSED: AtomicBool = AtomicBool::new(false);
static LAYOUT_DIRTY: AtomicBool = AtomicBool::new(false);
static REATTACH: AtomicBool = AtomicBool::new(false);
static REDRAW: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn run(debug: bool) -> i32 {
    unsafe { run_inner(debug) }
}

// ---------------------------------------------------------------- tray icon

unsafe fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid
}

unsafe fn add_tray(hwnd: HWND) {
    let hinst = GetModuleHandleW(std::ptr::null());
    // The app's icon is embedded as resource 32512 (or 1); fall back to the system default.
    let mut icon = LoadIconW(hinst, 32512 as *const u16);
    if icon.is_null() {
        icon = LoadIconW(hinst, 1 as *const u16);
    }
    if icon.is_null() {
        icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    }
    let mut nid = tray_data(hwnd);
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_WS_TRAY;
    nid.hIcon = icon;
    for (i, c) in "Wallshaders".encode_utf16().enumerate() {
        nid.szTip[i] = c;
    }
    Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let open = wide("Open Wallshaders");
    let pause = wide(if USER_PAUSED.load(Relaxed) { "Resume wallpaper" } else { "Pause wallpaper" });
    let quit = wide("Stop wallpaper and quit");
    AppendMenuW(menu, MF_STRING, MENU_OPEN, open.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_PAUSE, pause.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, MENU_QUIT, quit.as_ptr());
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON, p.x, p.y, 0, hwnd, std::ptr::null());
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}

fn open_app() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}

// ------------------------------------------------------------ window procs

unsafe extern "system" fn ctrl_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WS_RELOAD => {
            RELOAD.store(true, Relaxed);
            0
        }
        WM_WS_TRAY => {
            match (lp as u32) & 0xFFFF {
                WM_LBUTTONUP => open_app(),
                WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            match wp & 0xFFFF {
                MENU_OPEN => open_app(),
                MENU_PAUSE => {
                    USER_PAUSED.fetch_xor(true, Relaxed);
                }
                MENU_QUIT => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_WTSSESSION_CHANGE => {
            match wp {
                7 => LOCKED.store(true, Relaxed),  // WTS_SESSION_LOCK
                8 => LOCKED.store(false, Relaxed), // WTS_SESSION_UNLOCK
                _ => {}
            }
            0
        }
        WM_POWERBROADCAST => {
            if wp as u32 == PBT_POWERSETTINGCHANGE && lp != 0 {
                let s = lp as *const POWERBROADCAST_SETTING;
                if (*s).PowerSetting.data1 == DISPLAY_STATE.data1 {
                    DISPLAY_OFF.store((*s).Data[0] == 0, Relaxed);
                }
            }
            1
        }
        WM_DISPLAYCHANGE => {
            LAYOUT_DIRTY.store(true, Relaxed);
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            Shell_NotifyIconW(NIM_DELETE, &tray_data(hwnd));
            QUIT.store(true, Relaxed);
            PostQuitMessage(0);
            0
        }
        m if m != 0 && m == TASKBAR_CREATED.load(Relaxed) => {
            // Explorer restarted: the tray icon and the desktop host are gone.
            add_tray(hwnd);
            REATTACH.store(true, Relaxed);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn wall_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            REDRAW.store(true, Relaxed);
            ValidateRect(hwnd, std::ptr::null());
            0
        }
        WM_ERASEBKGND => 1,
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// --------------------------------------------------- behind-the-icons host

struct Host {
    parent: HWND,
    /// Windows 11 24H2 and later: icons and wallpaper share Progman, so we must
    /// slot in between them. `None` on older systems.
    raised: Option<(HWND, HWND)>,
}

unsafe extern "system" fn find_classic_host(hwnd: HWND, lp: LPARAM) -> BOOL {
    let defview = wide("SHELLDLL_DefView");
    if !FindWindowExW(hwnd, std::ptr::null_mut(), defview.as_ptr(), std::ptr::null()).is_null() {
        // The wallpaper WorkerW is the sibling right after the window holding the icons.
        let workerw = wide("WorkerW");
        let target = FindWindowExW(std::ptr::null_mut(), hwnd, workerw.as_ptr(), std::ptr::null());
        *(lp as *mut HWND) = target;
        return 0;
    }
    1
}

unsafe fn find_host() -> Option<Host> {
    let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
    if progman.is_null() {
        return None;
    }
    // Undocumented but long-standing: asks Progman to split the wallpaper into its own WorkerW.
    let mut result = 0usize;
    SendMessageTimeoutW(progman, 0x052C, 0xD, 0x1, SMTO_NORMAL, 1000, &mut result);
    let defview_class = wide("SHELLDLL_DefView");
    let workerw_class = wide("WorkerW");
    for _ in 0..20 {
        let defview = FindWindowExW(progman, std::ptr::null_mut(), defview_class.as_ptr(), std::ptr::null());
        if !defview.is_null() {
            let workerw = FindWindowExW(progman, std::ptr::null_mut(), workerw_class.as_ptr(), std::ptr::null());
            if !workerw.is_null() {
                return Some(Host { parent: progman, raised: Some((defview, workerw)) });
            }
        }
        let mut classic: HWND = std::ptr::null_mut();
        EnumWindows(Some(find_classic_host), &mut classic as *mut HWND as LPARAM);
        if !classic.is_null() {
            return Some(Host { parent: classic, raised: None });
        }
        Sleep(100);
    }
    None
}

struct MonitorScan {
    origin: (i32, i32),
    height: i32,
    targets: Vec<[i32; 4]>,
    primary: (i32, i32),
}

unsafe extern "system" fn scan_monitor(mon: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
    let scan = &mut *(lp as *mut MonitorScan);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(mon, &mut info) != 0 {
        let r = info.rcMonitor;
        let (x0, x1) = (r.left - scan.origin.0, r.right - scan.origin.0);
        let (top, bottom) = (r.top - scan.origin.1, r.bottom - scan.origin.1);
        scan.targets.push([x0, scan.height - bottom, x1, scan.height - top]);
        if info.dwFlags & 1 != 0 {
            scan.primary = (r.right - r.left, r.bottom - r.top);
        }
    }
    1
}

struct Surface {
    hwnd: HWND,
    ctx: Option<GlContext>,
    pipe: Pipeline,
    /// Where each monitor sits inside the window, in OpenGL coordinates.
    targets: Vec<[i32; 4]>,
    primary: (i32, i32),
}

impl Surface {
    unsafe fn create(hinst: HINSTANCE) -> Result<Self, String> {
        let host = find_host().ok_or("The desktop window was not found")?;
        let layered = host.raised.is_some() && std::env::var_os("WALLSHADERS_NO_LAYERED").is_none();
        let class = wide(WALL_CLASS);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | if layered { WS_EX_LAYERED } else { 0 },
            class.as_ptr(),
            class.as_ptr(),
            WS_POPUP,
            0,
            0,
            16,
            16,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err("Could not create the wallpaper window".into());
        }
        if layered {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        }
        let ctx = match GlContext::create(hwnd) {
            Ok(c) => c,
            Err(e) => {
                DestroyWindow(hwnd);
                return Err(e);
            }
        };
        let pipe = Pipeline::new(&ctx.gl)?;
        SetParent(hwnd, host.parent);
        let mut surface = Surface { hwnd, ctx: Some(ctx), pipe, targets: Vec::new(), primary: (1, 1) };
        surface.layout();
        match host.raised {
            Some((defview, workerw)) => {
                // Order inside Progman, top to bottom: icons, us, the static wallpaper.
                let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
                SetWindowPos(hwnd, defview, 0, 0, 0, 0, flags | SWP_SHOWWINDOW);
                SetWindowPos(workerw, hwnd, 0, 0, 0, 0, flags);
            }
            None => {
                ShowWindow(hwnd, SW_SHOWNA);
            }
        }
        log(&format!("attached to desktop (raised layout: {}, layered: {layered})", host.raised.is_some()));
        Ok(surface)
    }

    /// Cover the whole virtual screen and note where each monitor is.
    unsafe fn layout(&mut self) {
        let origin = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
        let (w, h) = (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
        SetWindowPos(self.hwnd, std::ptr::null_mut(), 0, 0, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
        let mut scan = MonitorScan { origin, height: h, targets: Vec::new(), primary: (w, h) };
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(scan_monitor), &mut scan as *mut MonitorScan as LPARAM);
        if scan.targets.is_empty() {
            scan.targets.push([0, 0, w, h]);
        }
        self.targets = scan.targets;
        self.primary = scan.primary;
    }

    unsafe fn apply_scale(&mut self, scale: f32) {
        let w = (self.primary.0 as f32 * scale).round() as i32;
        let h = (self.primary.1 as f32 * scale).round() as i32;
        self.pipe.resize(&self.ctx.as_ref().unwrap().gl, w, h);
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.ctx.take();
        unsafe {
            if IsWindow(self.hwnd) != 0 {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

// ----------------------------------------------------------- pause rules

struct Coverage {
    covered: Vec<HMONITOR>,
}

unsafe fn monitor_rect(mon: HMONITOR) -> Option<RECT> {
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    (GetMonitorInfoW(mon, &mut info) != 0).then_some(info.rcMonitor)
}

fn is_shell_class(class: &str) -> bool {
    matches!(class, "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | WALL_CLASS | CONTROL_CLASS)
}

/// Does this window fill its monitor? Returns the monitor and whether the window is maximized.
unsafe fn fills_monitor(hwnd: HWND) -> Option<(HMONITOR, bool)> {
    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 || is_shell_class(&class_name(hwnd)) {
        return None;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if ex & WS_EX_TOOLWINDOW != 0 || ex & WS_EX_NOACTIVATE != 0 || ex & WS_EX_TRANSPARENT != 0 {
        return None; // overlays and helper windows
    }
    let mut cloaked = 0u32;
    DwmGetWindowAttribute(hwnd, 14, &mut cloaked as *mut u32 as *mut c_void, 4); // DWMWA_CLOAKED
    if cloaked != 0 {
        return None; // on another virtual desktop, or a suspended store app
    }
    let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
    if mon.is_null() {
        return None;
    }
    let m = monitor_rect(mon)?;
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut r);
    let maximized = IsZoomed(hwnd) != 0;
    let covers = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
    (maximized || covers).then_some((mon, maximized))
}

unsafe extern "system" fn scan_coverage(hwnd: HWND, lp: LPARAM) -> BOOL {
    let cov = &mut *(lp as *mut Coverage);
    if let Some((mon, _)) = fills_monitor(hwnd) {
        if !cov.covered.contains(&mon) {
            cov.covered.push(mon);
        }
    }
    1
}

/// Returns (a fullscreen app is in front, every monitor is hidden behind a window).
unsafe fn probe_desktop(monitor_count: usize) -> (bool, bool) {
    let fg = GetForegroundWindow();
    let fullscreen = !fg.is_null() && matches!(fills_monitor(fg), Some((_, false)));
    let mut cov = Coverage { covered: Vec::new() };
    EnumWindows(Some(scan_coverage), &mut cov as *mut Coverage as LPARAM);
    (fullscreen, cov.covered.len() >= monitor_count.max(1))
}

unsafe fn on_battery() -> bool {
    let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
    GetSystemPowerStatus(&mut status) != 0 && status.ACLineStatus == 0
}

// ------------------------------------------------------------ render loop

/// Sleep until `ms` have passed or a window message arrives.
unsafe fn wait_ms(timer: HANDLE, ms: f64) {
    if ms <= 0.0 {
        return;
    }
    if !timer.is_null() {
        let due: i64 = -((ms * 10_000.0) as i64).max(1);
        SetWaitableTimer(timer, &due, 0, None, std::ptr::null(), 0);
        MsgWaitForMultipleObjects(1, &timer, 0, INFINITE, QS_ALLINPUT);
    } else {
        MsgWaitForMultipleObjects(0, std::ptr::null(), 0, ms.ceil() as u32, QS_ALLINPUT);
    }
}

unsafe fn local_date() -> [f32; 4] {
    let mut t: SYSTEMTIME = std::mem::zeroed();
    GetLocalTime(&mut t);
    let secs = t.wHour as f32 * 3600.0 + t.wMinute as f32 * 60.0 + t.wSecond as f32 + t.wMilliseconds as f32 / 1000.0;
    [t.wYear as f32, t.wMonth as f32 - 1.0, t.wDay as f32, secs]
}

unsafe fn register_class(hinst: HINSTANCE, name: &str, proc: WNDPROC) {
    let class = wide(name);
    let mut wc: WNDCLASSW = std::mem::zeroed();
    wc.style = CS_OWNDC;
    wc.lpfnWndProc = proc;
    wc.hInstance = hinst;
    wc.lpszClassName = class.as_ptr();
    RegisterClassW(&wc);
}

unsafe fn run_inner(debug: bool) -> i32 {
    // Only one wallpaper at a time: a second launch just nudges the first.
    if control::is_running() {
        control::notify_reload();
        return 0;
    }
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    // Never compete with whatever the user is actually doing.
    SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);

    let hinst = GetModuleHandleW(std::ptr::null());
    register_class(hinst, CONTROL_CLASS, Some(ctrl_proc));
    register_class(hinst, WALL_CLASS, Some(wall_proc));
    let ctrl_class = wide(CONTROL_CLASS);
    let ctrl = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        ctrl_class.as_ptr(),
        ctrl_class.as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        hinst,
        std::ptr::null(),
    );
    if ctrl.is_null() {
        log("could not create the control window");
        return 1;
    }
    TASKBAR_CREATED.store(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), Relaxed);
    add_tray(ctrl);
    WTSRegisterSessionNotification(ctrl, 0);
    RegisterPowerSettingNotification(ctrl as HANDLE, &DISPLAY_STATE, 0);
    // High-resolution timer (Windows 10 1803+); plain timer on older builds.
    let mut timer = CreateWaitableTimerExW(std::ptr::null(), std::ptr::null(), 0x2, 0x1F0003);
    if timer.is_null() {
        timer = CreateWaitableTimerExW(std::ptr::null(), std::ptr::null(), 0, 0x1F0003);
    }
    log("renderer started");

    let mut surface: Option<Surface> = None;
    let mut cfg = Config::load();
    let mut need_load = true;
    let mut animated = true;
    let mut wants_mouse = false;
    let mut dirty_frames = 3u32;
    let mut shader_time = 0f64;
    let mut frame_index = 0i32;
    let mut last_frame = Instant::now();
    let mut next_frame = Instant::now();
    let mut last_rules: Option<Instant> = None;
    let mut auto_paused = false;
    let mut battery = false;
    let mut stat_frames = 0u32;
    let mut stat_since = Instant::now();
    let mut msg: MSG = std::mem::zeroed();

    loop {
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT {
                QUIT.store(true, Relaxed);
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if QUIT.load(Relaxed) {
            break;
        }

        // (Re)create the wallpaper window. It dies with Explorer, so this also
        // recovers from an Explorer restart.
        let alive = surface.as_ref().map(|s| IsWindow(s.hwnd) != 0).unwrap_or(false);
        if !alive || REATTACH.swap(false, Relaxed) {
            surface = None;
            match Surface::create(hinst) {
                Ok(s) => {
                    surface = Some(s);
                    need_load = true;
                }
                Err(e) => {
                    log(&format!("attach failed: {e}"));
                    wait_ms(timer, 2000.0);
                    continue;
                }
            }
        }
        let s = surface.as_mut().unwrap();

        if RELOAD.swap(false, Relaxed) {
            need_load = true;
        }
        if need_load {
            need_load = false;
            cfg = Config::load();
            if cfg.active_shader.is_none() {
                log("no active shader, exiting");
                break;
            }
            let source = cfg.active_shader.as_ref().and_then(|n| std::fs::read_to_string(config::shader_path(n)).ok());
            match source {
                Some(src) => match s.pipe.compile(&s.ctx.as_ref().unwrap().gl, &src) {
                    Ok(()) => {
                        animated = shader::is_animated(&src);
                        wants_mouse = shader::uses_mouse(&src);
                        log(&format!("loaded shader {:?} (animated: {animated})", cfg.active_shader));
                    }
                    Err(e) => log(&format!("shader {:?} failed to compile:\n{e}", cfg.active_shader)),
                },
                None => log(&format!("no readable shader for {:?}", cfg.active_shader)),
            }
            s.apply_scale(cfg.render_scale);
            dirty_frames = 3;
            last_rules = None;
        }
        if LAYOUT_DIRTY.swap(false, Relaxed) {
            s.layout();
            s.apply_scale(cfg.render_scale);
            dirty_frames = 3;
        }
        if REDRAW.swap(false, Relaxed) {
            dirty_frames = dirty_frames.max(1);
        }

        if last_rules.map(|t| t.elapsed() >= Duration::from_millis(500)).unwrap_or(true) {
            last_rules = Some(Instant::now());
            let (fullscreen, covered) = probe_desktop(s.targets.len());
            battery = on_battery();
            auto_paused = (cfg.pause_fullscreen && fullscreen)
                || (cfg.pause_maximized && covered)
                || (battery && cfg.battery_mode == "pause");
        }
        let paused = auto_paused || LOCKED.load(Relaxed) || DISPLAY_OFF.load(Relaxed) || USER_PAUSED.load(Relaxed);
        if paused || (!animated && dirty_frames == 0) {
            if debug && stat_since.elapsed() >= Duration::from_secs(5) {
                log(&format!("idle (paused: {paused}, static shader: {})", !animated));
                stat_since = Instant::now();
                stat_frames = 0;
            }
            wait_ms(timer, 500.0);
            continue;
        }

        let now = Instant::now();
        if now < next_frame {
            wait_ms(timer, (next_frame - now).as_secs_f64() * 1000.0);
            continue;
        }
        let fps = if battery && cfg.battery_mode == "half" { (cfg.fps / 2).max(1) } else { cfg.fps };
        let interval = Duration::from_secs_f64(1.0 / fps as f64);
        // After a pause, continue from where the animation stopped instead of jumping.
        let delta = (now - last_frame).min(interval * 2).as_secs_f64();
        last_frame = now;
        shader_time += delta;
        next_frame += interval;
        if next_frame < now {
            next_frame = now + interval;
        }

        let gl = &s.ctx.as_ref().unwrap().gl;
        let mut frame = Frame {
            time: shader_time as f32,
            delta: delta as f32,
            rate: fps as f32,
            index: frame_index,
            mouse: [0.0; 4],
            date: local_date(),
        };
        if wants_mouse && cfg.mouse_input {
            let mut p = POINT { x: 0, y: 0 };
            GetCursorPos(&mut p);
            let (w, h) = s.pipe.size();
            let sx = w as f32 / s.primary.0 as f32;
            let sy = h as f32 / s.primary.1 as f32;
            frame.mouse = [p.x as f32 * sx, (s.primary.1 - p.y) as f32 * sy, 0.0, 0.0];
        }
        s.pipe.render(gl, &frame);
        s.pipe.present(gl, &s.targets);
        s.ctx.as_ref().unwrap().swap();
        frame_index = frame_index.wrapping_add(1);
        dirty_frames = dirty_frames.saturating_sub(1);

        if debug {
            stat_frames += 1;
            if stat_since.elapsed() >= Duration::from_secs(5) {
                let secs = stat_since.elapsed().as_secs_f32();
                log(&format!(
                    "{:.1} fps, target {fps}, buffer {:?}, monitors {}, has shader: {}",
                    stat_frames as f32 / secs,
                    s.pipe.size(),
                    s.targets.len(),
                    s.pipe.has_program()
                ));
                stat_frames = 0;
                stat_since = Instant::now();
            }
        }
    }

    drop(surface);
    log("renderer stopped");
    0
}
