//! The Wallshaders editor app: shader library, settings, and control of the
//! wallpaper renderer (which is this same executable started with `--renderer`).

mod library;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;
use tauri::Manager;
use wallshaders_renderer::config::{self, Config};
use wallshaders_renderer::{control, monitors};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const GPU_KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";

fn exe_path() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct ScreenStatus {
    id: String,
    name: String,
    width: i32,
    height: i32,
    primary: bool,
    /// What this screen shows right now (`None`: the normal Windows wallpaper).
    shader: Option<String>,
}

#[derive(Serialize)]
struct Status {
    running: bool,
    /// The shader shared by screens without their own choice.
    active: Option<String>,
    screens: Vec<ScreenStatus>,
}

#[tauri::command]
fn list_shaders() -> Result<Vec<String>, String> {
    library::list()
}

#[tauri::command]
fn read_shader(name: String) -> Result<String, String> {
    library::read(&name)
}

#[tauri::command]
fn save_shader(name: String, source: String) -> Result<(), String> {
    library::write(&name, &source)?;
    // Saving a script that is on the desktop updates the desktop.
    if Config::load().uses_shader(&name) {
        control::notify_reload();
    }
    Ok(())
}

/// Save settings and tell the renderer, or stop it when no screen has a shader left.
fn save_and_notify(cfg: &Config) -> Result<(), String> {
    cfg.save()?;
    if cfg.has_any_shader() {
        control::notify_reload();
    } else {
        control::quit();
    }
    Ok(())
}

#[tauri::command]
fn delete_shader(name: String) -> Result<(), String> {
    library::delete(&name)?;
    let mut cfg = Config::load();
    if cfg.forget_shader(&name) {
        save_and_notify(&cfg)?;
    }
    Ok(())
}

#[tauri::command]
fn rename_shader(name: String, new_name: String) -> Result<String, String> {
    let new_name = library::rename(&name, &new_name)?;
    let mut cfg = Config::load();
    if cfg.rename_shader(&name, &new_name) {
        save_and_notify(&cfg)?;
    }
    Ok(new_name)
}

#[tauri::command]
fn import_shader(path: String) -> Result<String, String> {
    library::import(&path)
}

#[tauri::command]
fn export_shader(name: String, path: String) -> Result<(), String> {
    std::fs::write(&path, library::read(&name)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_builtins() -> Result<(), String> {
    library::write_builtins(true)
}

#[tauri::command]
fn get_config() -> Config {
    Config::load()
}

#[tauri::command]
fn set_config(config: Config) -> Result<Config, String> {
    // The UI does not own which shader is on which screen; keep whatever is on disk.
    let mut next = config.sanitized();
    let current = Config::load();
    next.active_shader = current.active_shader;
    next.screens = current.screens;
    next.save()?;
    control::notify_reload();
    Ok(next)
}

/// Ask Windows to run us on the power-saving GPU on dual-GPU laptops, unless
/// the user already made their own choice in Settings > Display > Graphics.
fn prefer_power_saving_gpu(exe: &str) {
    if let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(GPU_KEY) {
        if key.get_value::<String, _>(exe).is_err() {
            let _ = key.set_value(exe, &"GpuPreference=1;");
        }
    }
}

/// Show a shader on every screen (`screen` = `None`) or on one screen.
#[tauri::command(async)]
fn apply_wallpaper(name: String, screen: Option<String>) -> Result<(), String> {
    let path = config::shader_path(&library::valid_name(&name)?);
    let exe = exe_path()?;
    // Compile with the real renderer first so a broken script never reaches the desktop.
    let check = Command::new(&exe)
        .arg("--check")
        .arg(&path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Could not start the renderer: {e}"))?;
    if !check.status.success() {
        let log = String::from_utf8_lossy(&check.stderr).trim().to_string();
        return Err(if log.is_empty() { "The shader failed to compile.".into() } else { log });
    }

    let mut cfg = Config::load();
    match screen {
        Some(id) => {
            cfg.screens.insert(id, name);
        }
        None => {
            cfg.active_shader = Some(name);
            cfg.screens.clear();
        }
    }
    cfg.save()?;
    if !control::notify_reload() {
        start_renderer(&exe)?;
    }
    Ok(())
}

fn start_renderer(exe: &str) -> Result<(), String> {
    prefer_power_saving_gpu(exe);
    Command::new(exe)
        .arg("--renderer")
        .creation_flags(DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start the wallpaper: {e}"))
}

/// Left in the data folder while an update installs, so the wallpaper that was
/// stopped for it comes back when the updated app opens.
fn resume_marker() -> std::path::PathBuf {
    config::data_dir().join("resume-after-update")
}

/// Before installing an update: the installer replaces this program's file, so
/// the wallpaper (which runs from the same file) has to stop first.
#[tauri::command(async)]
fn prepare_update() -> Result<(), String> {
    if !control::is_running() {
        return Ok(());
    }
    std::fs::write(resume_marker(), "").map_err(|e| e.to_string())?;
    if control::quit_and_wait(8000) {
        Ok(())
    } else {
        let _ = std::fs::remove_file(resume_marker());
        Err("The wallpaper did not stop, so the update cannot be installed right now. Try again.".into())
    }
}

/// Bring back a wallpaper that was stopped for an update (after the update, or
/// when installing it failed).
#[tauri::command]
fn resume_wallpaper() -> Result<(), String> {
    let marker = resume_marker();
    if !marker.exists() {
        return Ok(());
    }
    let _ = std::fs::remove_file(&marker);
    if Config::load().has_any_shader() && !control::is_running() {
        start_renderer(&exe_path()?)?;
    }
    Ok(())
}

/// Stop the wallpaper everywhere (`screen` = `None`), or give one screen back
/// its normal Windows wallpaper.
#[tauri::command]
fn stop_wallpaper(screen: Option<String>) -> Result<(), String> {
    let mut cfg = Config::load();
    match screen {
        Some(id) => {
            cfg.screens.insert(id, String::new());
        }
        None => {
            cfg.active_shader = None;
            cfg.screens.clear();
        }
    }
    save_and_notify(&cfg)
}

#[tauri::command]
fn wallpaper_status() -> Status {
    let running = control::is_running();
    let cfg = Config::load();
    let screens = monitors::list()
        .into_iter()
        .map(|m| ScreenStatus {
            shader: if running { cfg.shader_for(&m.id).map(str::to_string) } else { None },
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            primary: m.primary,
        })
        .collect();
    Status { running, active: if running { cfg.active_shader.clone() } else { None }, screens }
}

#[tauri::command]
fn get_autostart() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>("Wallshaders"))
        .is_ok()
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if enabled {
        key.set_value("Wallshaders", &format!("\"{}\" --renderer", exe_path()?)).map_err(|e| e.to_string())
    } else {
        match key.delete_value("Wallshaders") {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

pub fn run() {
    if let Err(e) = library::ensure() {
        config::log(&format!("could not prepare the shader library: {e}"));
    }
    if let Err(e) = resume_wallpaper() {
        config::log(&format!("could not restart the wallpaper after an update: {e}"));
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            list_shaders,
            read_shader,
            save_shader,
            delete_shader,
            rename_shader,
            import_shader,
            export_shader,
            restore_builtins,
            get_config,
            set_config,
            apply_wallpaper,
            stop_wallpaper,
            wallpaper_status,
            get_autostart,
            set_autostart,
            prepare_update,
            resume_wallpaper,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wallshaders");
}
