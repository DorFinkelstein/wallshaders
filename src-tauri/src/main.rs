// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The same executable is both the editor and the wallpaper. Renderer modes
    // return before any of the app's UI (and its memory cost) is loaded.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = wallshaders_renderer::run_cli(&args) {
        std::process::exit(code);
    }
    wallshaders_lib::run();
}
