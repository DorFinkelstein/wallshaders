//! The shader library: plain `.glsl` files in `%APPDATA%\Wallshaders\shaders`.

use std::fs;
use wallshaders_renderer::config::{shader_path, shaders_dir};

const BUILTINS: &[(&str, &str)] = &[
    ("Terracotta Dusk", include_str!("../../shaders/Terracotta Dusk.glsl")),
    ("Soft Hills", include_str!("../../shaders/Soft Hills.glsl")),
    ("Starfield", include_str!("../../shaders/Starfield.glsl")),
    ("Aurora", include_str!("../../shaders/Aurora.glsl")),
    ("Lava Lamp", include_str!("../../shaders/Lava Lamp.glsl")),
    ("Fireflies", include_str!("../../shaders/Fireflies.glsl")),
    ("Warm Plasma", include_str!("../../shaders/Warm Plasma.glsl")),
    ("Slow Ripples", include_str!("../../shaders/Slow Ripples.glsl")),
    ("Paper Gradient", include_str!("../../shaders/Paper Gradient.glsl")),
];

/// Starter shaders as earlier versions shipped them (FNV-1a 64 of the text with
/// LF line endings). A library copy that still matches one was never edited, so
/// it is safe to replace with the current version.
const PREVIOUS_BUILTINS: &[(&str, u64)] = &[
    // 0.1.0
    ("Aurora", 0xc5095777df097397),
    ("Fireflies", 0xd58f83560ae75b0c),
    ("Lava Lamp", 0x437e5399a1d1c225),
    ("Paper Gradient", 0x6a03787b876a1a01),
    ("Slow Ripples", 0x79fb07590c4c8064),
    ("Soft Hills", 0xacfaf5fce7c180c6),
    ("Starfield", 0xa7ecbae89bbba255),
    ("Terracotta Dusk", 0xce56f0ea5c822ee3),
    ("Warm Plasma", 0x067f52915091d00f),
];

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;

fn text_hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.replace("\r\n", "\n").bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn allowed(c: char) -> bool {
    c.is_alphanumeric() || " -_()".contains(c)
}

/// Names become file names, so only a safe set of characters is accepted.
pub fn valid_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("The name cannot be empty.".into());
    }
    if n.chars().count() > 60 {
        return Err("The name is too long (60 characters at most).".into());
    }
    if !n.chars().all(allowed) {
        return Err("Use only letters, numbers, spaces, - _ ( ) in the name.".into());
    }
    Ok(n.to_string())
}

/// Turn any text into an acceptable name.
fn clean_name(raw: &str) -> String {
    let cleaned: String = raw.chars().map(|c| if allowed(c) { c } else { ' ' }).collect();
    let cleaned: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned: String = cleaned.chars().take(50).collect();
    if cleaned.is_empty() { "Imported".into() } else { cleaned }
}

fn unique_name(base: &str) -> String {
    if !shader_path(base).exists() {
        return base.to_string();
    }
    (2..).map(|i| format!("{base} {i}")).find(|n| !shader_path(n).exists()).unwrap()
}

pub fn write_builtins(overwrite: bool) -> Result<(), String> {
    fs::create_dir_all(shaders_dir()).map_err(|e| e.to_string())?;
    for (name, source) in BUILTINS {
        let path = shader_path(name);
        if overwrite || !path.exists() {
            fs::write(path, source).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// First run: create the library and fill it with the starter shaders.
/// Later runs: bring unedited starter shaders up to date.
pub fn ensure() -> Result<(), String> {
    if !shaders_dir().exists() {
        return write_builtins(false);
    }
    for (name, source) in BUILTINS {
        let path = shader_path(name);
        let Ok(current) = fs::read_to_string(&path) else { continue };
        let hash = text_hash(&current);
        if hash != text_hash(source) && PREVIOUS_BUILTINS.iter().any(|(n, h)| n == name && *h == hash) {
            fs::write(&path, source).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn list() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let entries = match fs::read_dir(shaders_dir()) {
        Ok(e) => e,
        Err(_) => return Ok(names),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("glsl") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    Ok(names)
}

pub fn read(name: &str) -> Result<String, String> {
    fs::read_to_string(shader_path(&valid_name(name)?)).map_err(|e| format!("Could not open \"{name}\": {e}"))
}

pub fn write(name: &str, source: &str) -> Result<(), String> {
    let name = valid_name(name)?;
    fs::create_dir_all(shaders_dir()).map_err(|e| e.to_string())?;
    fs::write(shader_path(&name), source).map_err(|e| format!("Could not save \"{name}\": {e}"))
}

pub fn delete(name: &str) -> Result<(), String> {
    fs::remove_file(shader_path(&valid_name(name)?)).map_err(|e| e.to_string())
}

pub fn rename(name: &str, new_name: &str) -> Result<String, String> {
    let (old, new) = (valid_name(name)?, valid_name(new_name)?);
    if old == new {
        return Ok(new);
    }
    // A change of letter case only is the same file on Windows, so allow it.
    if shader_path(&new).exists() && old.to_lowercase() != new.to_lowercase() {
        return Err(format!("A shader named \"{new}\" already exists."));
    }
    fs::rename(shader_path(&old), shader_path(&new)).map_err(|e| e.to_string())?;
    Ok(new)
}

/// Copy a shader file from anywhere on disk into the library.
pub fn import(path: &str) -> Result<String, String> {
    let src = std::path::Path::new(path);
    let size = fs::metadata(src).map_err(|e| e.to_string())?.len();
    if size > MAX_SOURCE_BYTES {
        return Err("That file is too large to be a shader script (1 MB at most).".into());
    }
    let source = fs::read_to_string(src).map_err(|_| "That file is not readable text.".to_string())?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported");
    let name = unique_name(&clean_name(stem));
    write(&name, &source)?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_tricks() {
        assert!(valid_name("..\\evil").is_err());
        assert!(valid_name("a/b").is_err());
        assert!(valid_name("  ").is_err());
        assert_eq!(valid_name(" Soft Hills ").unwrap(), "Soft Hills");
    }

    #[test]
    fn cleans_imported_names() {
        assert_eq!(clean_name("my.cool/shader!!"), "my cool shader");
        assert_eq!(clean_name("***"), "Imported");
    }

    #[test]
    fn text_hash_ignores_line_endings() {
        assert_eq!(text_hash("a\r\nb\n"), text_hash("a\nb\n"));
        assert_ne!(text_hash("a"), text_hash("b"));
        // A current starter is never mistaken for an old one.
        for (name, source) in BUILTINS {
            assert!(!PREVIOUS_BUILTINS.iter().any(|(n, h)| n == name && *h == text_hash(source)));
        }
    }

    #[test]
    fn builtins_use_valid_names() {
        for (name, source) in BUILTINS {
            assert_eq!(&valid_name(name).unwrap(), name);
            assert!(source.contains("mainImage"));
        }
    }
}
