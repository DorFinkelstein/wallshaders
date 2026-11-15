//! The Wallshaders editor app: shader library, settings, and control of the
//! wallpaper renderer (which is this same executable started with `--renderer`).

mod library;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;
use tauri::Manager;
use wallshaders_renderer::config::{self, Config};
use wallshaders_renderer::control;
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
struct Status {
    running: bool,
    active: Option<String>,
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
    // Saving the script that is on the desktop updates the desktop.
    if Config::load().active_shader.as_deref() == Some(name.as_str()) {
        control::notify_reload();
    }
    Ok(())
}

#[tauri::command]
fn delete_shader(name: String) -> Result<(), String> {
    library::delete(&name)?;
    let mut cfg = Config::load();
    if cfg.active_shader.as_deref() == Some(name.as_str()) {
        cfg.active_shader = None;
        cfg.save()?;
        control::quit();
    }
    Ok(())
}

#[tauri::command]
fn rename_shader(name: String, new_name: String) -> Result<String, String> {
    let new_name = library::rename(&name, &new_name)?;
    let mut cfg = Config::load();
    if cfg.active_shader.as_deref() == Some(name.as_str()) {
        cfg.active_shader = Some(new_name.clone());
        cfg.save()?;
        control::notify_reload();
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
    // The UI does not own the active shader; keep whatever is on disk.
    let mut next = config.sanitized();
    next.active_shader = Config::load().active_shader;
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

#[tauri::command(async)]
fn apply_wallpaper(name: String) -> Result<(), String> {
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
    cfg.active_shader = Some(name);
    cfg.save()?;
    if !control::notify_reload() {
        prefer_power_saving_gpu(&exe);
        Command::new(&exe)
            .arg("--renderer")
            .creation_flags(DETACHED_PROCESS)
            .spawn()
            .map_err(|e| format!("Could not start the wallpaper: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
fn stop_wallpaper() -> Result<(), String> {
    control::quit();
    let mut cfg = Config::load();
    cfg.active_shader = None;
    cfg.save()
}

#[tauri::command]
fn wallpaper_status() -> Status {
    let running = control::is_running();
    Status { running, active: if running { Config::load().active_shader } else { None } }
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
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wallshaders");
}
