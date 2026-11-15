//! Settings and file locations shared by the app and the wallpaper renderer.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Name (file stem) of the shader shown as wallpaper.
    pub active_shader: Option<String>,
    /// Frame rate cap.
    pub fps: u32,
    /// Fraction of the monitor resolution the shader is drawn at (0.25–1.0).
    pub render_scale: f32,
    /// Stop drawing while a fullscreen app or game is in front.
    pub pause_fullscreen: bool,
    /// Stop drawing while every monitor is covered by a maximized window.
    pub pause_maximized: bool,
    /// What to do on battery power: "none", "half" (half frame rate) or "pause".
    pub battery_mode: String,
    /// Feed the cursor position to shaders through `iMouse`.
    pub mouse_input: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            active_shader: None,
            fps: 30,
            render_scale: 0.75,
            pause_fullscreen: true,
            pause_maximized: true,
            battery_mode: "half".into(),
            mouse_input: false,
        }
    }
}

impl Config {
    pub fn sanitized(mut self) -> Self {
        self.fps = self.fps.clamp(1, 144);
        if !self.render_scale.is_finite() {
            self.render_scale = 0.75;
        }
        self.render_scale = self.render_scale.clamp(0.1, 1.0);
        if !matches!(self.battery_mode.as_str(), "none" | "half" | "pause") {
            self.battery_mode = "half".into();
        }
        self
    }

    pub fn load() -> Self {
        fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str::<Config>(&s).ok())
            .unwrap_or_default()
            .sanitized()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = data_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        // Write then rename so the renderer never reads a half-written file.
        let tmp = dir.join("config.json.tmp");
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp, config_path()).map_err(|e| e.to_string())
    }
}

/// `%APPDATA%\Wallshaders`, or `WALLSHADERS_DATA_DIR` when set (used by tests).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WALLSHADERS_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Wallshaders")
}

pub fn shaders_dir() -> PathBuf {
    data_dir().join("shaders")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn log_path() -> PathBuf {
    data_dir().join("wallshaders.log")
}

pub fn shader_path(name: &str) -> PathBuf {
    shaders_dir().join(format!("{name}.glsl"))
}

/// Append a line to the log file. The log is reset once it passes 512 KB.
pub fn log(msg: &str) {
    let path = log_path();
    let _ = fs::create_dir_all(data_dir());
    if fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
        let _ = fs::remove_file(&path);
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fill_missing_fields() {
        let c: Config = serde_json::from_str(r#"{"fps": 60}"#).unwrap();
        assert_eq!(c.fps, 60);
        assert_eq!(c.render_scale, 0.75);
        assert!(c.pause_fullscreen);
    }

    #[test]
    fn sanitize_clamps_bad_values() {
        let c = Config {
            fps: 0,
            render_scale: 9.0,
            battery_mode: "bogus".into(),
            ..Config::default()
        }
        .sanitized();
        assert_eq!(c.fps, 1);
        assert_eq!(c.render_scale, 1.0);
        assert_eq!(c.battery_mode, "half");
    }
}
