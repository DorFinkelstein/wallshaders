//! Settings and file locations shared by the app and the wallpaper renderer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Name (file stem) of the shader shown on every screen without its own choice.
    pub active_shader: Option<String>,
    /// Per-screen choices, keyed by monitor id (see `monitors`). An empty name
    /// means "no shader here": that screen keeps the normal Windows wallpaper.
    pub screens: BTreeMap<String, String>,
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
    /// Look for a new version of the app when it starts (the app's setting; the renderer ignores it).
    pub check_updates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            active_shader: None,
            screens: BTreeMap::new(),
            fps: 30,
            render_scale: 0.75,
            pause_fullscreen: true,
            pause_maximized: true,
            battery_mode: "half".into(),
            mouse_input: false,
            check_updates: true,
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

    /// The shader a screen shows, or `None` for the normal Windows wallpaper.
    pub fn shader_for(&self, screen: &str) -> Option<&str> {
        match self.screens.get(screen) {
            Some(name) => (!name.is_empty()).then_some(name.as_str()),
            None => self.active_shader.as_deref(),
        }
    }

    /// Is any screen set to show a shader?
    pub fn has_any_shader(&self) -> bool {
        self.active_shader.is_some() || self.screens.values().any(|s| !s.is_empty())
    }

    /// Is this shader on any screen?
    pub fn uses_shader(&self, name: &str) -> bool {
        self.active_shader.as_deref() == Some(name) || self.screens.values().any(|s| s == name)
    }

    /// Follow a renamed shader. Returns true if anything changed.
    pub fn rename_shader(&mut self, from: &str, to: &str) -> bool {
        let mut changed = false;
        if self.active_shader.as_deref() == Some(from) {
            self.active_shader = Some(to.to_string());
            changed = true;
        }
        for v in self.screens.values_mut() {
            if v == from {
                *v = to.to_string();
                changed = true;
            }
        }
        changed
    }

    /// Take a deleted shader off every screen. Returns true if anything changed.
    pub fn forget_shader(&mut self, name: &str) -> bool {
        let mut changed = false;
        if self.active_shader.as_deref() == Some(name) {
            self.active_shader = None;
            changed = true;
        }
        let before = self.screens.len();
        self.screens.retain(|_, v| v != name);
        changed || self.screens.len() != before
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
    fn screens_override_the_shared_shader() {
        let mut c = Config { active_shader: Some("A".into()), ..Config::default() };
        c.screens.insert("left".into(), "B".into());
        c.screens.insert("right".into(), String::new());
        assert_eq!(c.shader_for("left"), Some("B"));
        assert_eq!(c.shader_for("right"), None);
        assert_eq!(c.shader_for("middle"), Some("A"));
        assert!(c.rename_shader("B", "C"));
        assert_eq!(c.shader_for("left"), Some("C"));
        assert!(c.forget_shader("A"));
        assert_eq!(c.shader_for("middle"), None);
        assert!(c.has_any_shader());
        assert!(c.forget_shader("C"));
        assert!(!c.has_any_shader());
        // Old config files without "screens" still load.
        let old: Config = serde_json::from_str(r#"{"active_shader": "A"}"#).unwrap();
        assert!(old.screens.is_empty());
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
