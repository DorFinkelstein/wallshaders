//! Invisible rendering: a window that is never shown provides the OpenGL
//! context, and frames are drawn into an offscreen buffer.

use crate::glctx::{Frame, GlContext, Pipeline};
use crate::win::wide;
use std::fs;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct Hidden {
    hwnd: HWND,
    ctx: Option<GlContext>,
}

impl Hidden {
    unsafe fn new() -> Result<Self, String> {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide("WallshadersHeadless");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.style = CS_OWNDC;
        wc.lpfnWndProc = Some(DefWindowProcW);
        wc.hInstance = hinst;
        wc.lpszClassName = class.as_ptr();
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
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
            return Err("Could not create the offscreen window".into());
        }
        let ctx = GlContext::create(hwnd)?;
        Ok(Hidden { hwnd, ctx: Some(ctx) })
    }
}

impl Drop for Hidden {
    fn drop(&mut self) {
        self.ctx.take();
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

fn read_script(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Cannot read {path}: {e}"))
}

pub fn check(path: &str) -> Result<(), String> {
    let src = read_script(path)?;
    unsafe {
        let hidden = Hidden::new()?;
        let gl = &hidden.ctx.as_ref().unwrap().gl;
        let mut pipe = Pipeline::new(gl)?;
        pipe.compile(gl, &src)
    }
}

pub fn render_png(path: &str, out: &str, w: i32, h: i32, time: f32) -> Result<(), String> {
    let src = read_script(path)?;
    let pixels = unsafe {
        let hidden = Hidden::new()?;
        let gl = &hidden.ctx.as_ref().unwrap().gl;
        let mut pipe = Pipeline::new(gl)?;
        pipe.compile(gl, &src)?;
        pipe.resize(gl, w, h);
        let frame = Frame { time, delta: 1.0 / 30.0, rate: 30.0, index: (time * 30.0) as i32, ..Frame::default() };
        pipe.render(gl, &frame);
        pipe.read_rgba(gl)
    };
    let file = fs::File::create(out).map_err(|e| format!("Cannot write {out}: {e}"))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w.max(1) as u32, h.max(1) as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&pixels).map_err(|e| e.to_string())
}
