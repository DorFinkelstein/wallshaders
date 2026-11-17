//! Wallshaders renderer: draws a GLSL script behind the desktop icons.
//!
//! The same code backs three modes, selected on the command line:
//! `--renderer` (the live wallpaper), `--check` (compile a script and report errors)
//! and `--headless` (draw one frame to a PNG without showing anything).

pub mod config;
pub mod control;
pub mod monitors;
pub mod shader;

mod glctx;
mod headless;
mod wallpaper;
mod win;

pub const USAGE: &str = "Wallshaders renderer
  --renderer [--debug]                 run as the desktop wallpaper
  --check <script.glsl>                compile a script; errors go to stderr, exit code 1
  --headless <script.glsl> --out <file.png> [--size WxH] [--time SECONDS]
                                       draw one frame to a PNG, invisibly
  --monitors                           list the screens as JSON";

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

/// Handle a renderer command line. Returns `None` when the arguments are not
/// for the renderer (the app then starts its normal window).
pub fn run_cli(args: &[String]) -> Option<i32> {
    if args.iter().any(|a| a == "--renderer") {
        let debug = args.iter().any(|a| a == "--debug");
        return Some(wallpaper::run(debug));
    }
    if args.iter().any(|a| a == "--monitors") {
        println!("{}", serde_json::to_string_pretty(&monitors::list()).unwrap_or_default());
        return Some(0);
    }
    if let Some(path) = value_after(args, "--check") {
        return Some(match headless::check(path) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    if let Some(path) = value_after(args, "--headless") {
        let out = value_after(args, "--out").unwrap_or("frame.png");
        let (w, h) = value_after(args, "--size")
            .and_then(|s| s.split_once('x'))
            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .unwrap_or((640, 360));
        let time = value_after(args, "--time").and_then(|t| t.parse().ok()).unwrap_or(1.0);
        return Some(match headless::render_png(path, out, w, h, time) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    None
}
