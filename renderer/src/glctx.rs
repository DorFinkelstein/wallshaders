//! OpenGL 3.3 context creation through raw WGL, and the shader pipeline.

use crate::shader;
use glow::HasContext;
use std::ffi::{c_void, CString};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};
use windows_sys::Win32::Graphics::OpenGL::*;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

pub struct GlContext {
    hwnd: HWND,
    hdc: HDC,
    hglrc: HGLRC,
    pub gl: glow::Context,
}

type CreateContextAttribs = unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;
type SwapInterval = unsafe extern "system" fn(i32) -> i32;

unsafe fn wgl_proc(name: &[u8]) -> *const c_void {
    match wglGetProcAddress(name.as_ptr()) {
        Some(f) => {
            let p = f as usize;
            // Some drivers return small sentinel values instead of null.
            if p <= 3 || p == usize::MAX { std::ptr::null() } else { p as *const c_void }
        }
        None => std::ptr::null(),
    }
}

impl GlContext {
    pub unsafe fn create(hwnd: HWND) -> Result<Self, String> {
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return Err("GetDC failed".into());
        }
        let mut pfd: PIXELFORMATDESCRIPTOR = std::mem::zeroed();
        pfd.nSize = std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16;
        pfd.nVersion = 1;
        // PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER; pixel type 0 = RGBA.
        pfd.dwFlags = (0x4 | 0x20 | 0x1) as _;
        pfd.cColorBits = 32;
        let format = ChoosePixelFormat(hdc, &pfd);
        if format == 0 || SetPixelFormat(hdc, format, &pfd) == 0 {
            ReleaseDC(hwnd, hdc);
            return Err("No usable OpenGL pixel format".into());
        }

        // A legacy context is needed first to reach wglCreateContextAttribsARB.
        let legacy = wglCreateContext(hdc);
        if legacy.is_null() || wglMakeCurrent(hdc, legacy) == 0 {
            ReleaseDC(hwnd, hdc);
            return Err("Could not create an OpenGL context".into());
        }
        let create = wgl_proc(b"wglCreateContextAttribsARB\0");
        if create.is_null() {
            wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
            wglDeleteContext(legacy);
            ReleaseDC(hwnd, hdc);
            return Err("This graphics driver does not support OpenGL 3.3".into());
        }
        let create: CreateContextAttribs = std::mem::transmute(create);
        // MAJOR 3, MINOR 3, PROFILE_MASK = CORE.
        let attribs = [0x2091, 3, 0x2092, 3, 0x9126, 0x1, 0];
        let hglrc = create(hdc, std::ptr::null_mut(), attribs.as_ptr());
        wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
        wglDeleteContext(legacy);
        if hglrc.is_null() || wglMakeCurrent(hdc, hglrc) == 0 {
            ReleaseDC(hwnd, hdc);
            return Err("This graphics driver does not support OpenGL 3.3".into());
        }

        // Frame pacing is done by the render loop, so never block in SwapBuffers.
        let swap = wgl_proc(b"wglSwapIntervalEXT\0");
        if !swap.is_null() {
            let swap: SwapInterval = std::mem::transmute(swap);
            swap(0);
        }

        let opengl32 = GetModuleHandleA(b"opengl32.dll\0".as_ptr());
        let gl = glow::Context::from_loader_function(|name| {
            let cname = CString::new(name).unwrap();
            let p = wgl_proc(cname.as_bytes_with_nul());
            if !p.is_null() {
                return p;
            }
            match GetProcAddress(opengl32, cname.as_ptr() as *const u8) {
                Some(f) => f as usize as *const c_void,
                None => std::ptr::null(),
            }
        });
        Ok(GlContext { hwnd, hdc, hglrc, gl })
    }

    /// Draw with this context (into its own window) from now on. Needed when
    /// a process has several contexts, one per screen.
    pub unsafe fn make_own_current(&self) -> bool {
        wglMakeCurrent(self.hdc, self.hglrc) != 0
    }

    /// Show what was drawn into the window's back buffer.
    pub fn swap(&self) {
        unsafe {
            SwapBuffers(self.hdc);
        }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
            wglDeleteContext(self.hglrc);
            ReleaseDC(self.hwnd, self.hdc);
        }
    }
}

/// Per-frame shader inputs.
#[derive(Default, Clone, Copy)]
pub struct Frame {
    pub time: f32,
    pub delta: f32,
    pub rate: f32,
    pub index: i32,
    pub mouse: [f32; 4],
    pub date: [f32; 4],
}

struct Uniforms {
    resolution: Option<glow::UniformLocation>,
    time: Option<glow::UniformLocation>,
    delta: Option<glow::UniformLocation>,
    rate: Option<glow::UniformLocation>,
    frame: Option<glow::UniformLocation>,
    mouse: Option<glow::UniformLocation>,
    date: Option<glow::UniformLocation>,
}

/// Draws a script into an offscreen buffer, which is then stretched onto the screen.
/// Drawing at a reduced size is what makes `render_scale` cheap.
pub struct Pipeline {
    program: Option<(glow::Program, Uniforms)>,
    vao: glow::VertexArray,
    fbo: glow::Framebuffer,
    rbo: glow::Renderbuffer,
    size: (i32, i32),
}

impl Pipeline {
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let vao = gl.create_vertex_array()?;
        let fbo = gl.create_framebuffer()?;
        let rbo = gl.create_renderbuffer()?;
        Ok(Pipeline { program: None, vao, fbo, rbo, size: (0, 0) })
    }

    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    /// Compile a script. On failure the previous program stays active and the
    /// driver's log (with script line numbers) is returned.
    pub unsafe fn compile(&mut self, gl: &glow::Context, user: &str) -> Result<(), String> {
        let vs = compile_stage(gl, glow::VERTEX_SHADER, shader::VERTEX)?;
        let fs = match compile_stage(gl, glow::FRAGMENT_SHADER, &shader::wrap_fragment(user)) {
            Ok(fs) => fs,
            Err(e) => {
                gl.delete_shader(vs);
                return Err(shader::fix_error_lines(&e));
            }
        };
        let program = gl.create_program()?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(shader::fix_error_lines(&log));
        }
        let u = |name: &str| gl.get_uniform_location(program, name);
        let uniforms = Uniforms {
            resolution: u("iResolution"),
            time: u("iTime"),
            delta: u("iTimeDelta"),
            rate: u("iFrameRate"),
            frame: u("iFrame"),
            mouse: u("iMouse"),
            date: u("iDate"),
        };
        if let Some((old, _)) = self.program.take() {
            gl.delete_program(old);
        }
        self.program = Some((program, uniforms));
        Ok(())
    }

    pub unsafe fn resize(&mut self, gl: &glow::Context, w: i32, h: i32) {
        let (w, h) = (w.max(1), h.max(1));
        if self.size == (w, h) {
            return;
        }
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(self.rbo));
        gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, w, h);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::RENDERBUFFER, Some(self.rbo));
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        self.size = (w, h);
    }

    /// Draw one frame into the offscreen buffer.
    pub unsafe fn render(&self, gl: &glow::Context, f: &Frame) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.viewport(0, 0, self.size.0, self.size.1);
        match &self.program {
            Some((program, u)) => {
                gl.use_program(Some(*program));
                gl.uniform_3_f32(u.resolution.as_ref(), self.size.0 as f32, self.size.1 as f32, 1.0);
                gl.uniform_1_f32(u.time.as_ref(), f.time);
                gl.uniform_1_f32(u.delta.as_ref(), f.delta);
                gl.uniform_1_f32(u.rate.as_ref(), f.rate);
                gl.uniform_1_i32(u.frame.as_ref(), f.index);
                gl.uniform_4_f32(u.mouse.as_ref(), f.mouse[0], f.mouse[1], f.mouse[2], f.mouse[3]);
                gl.uniform_4_f32(u.date.as_ref(), f.date[0], f.date[1], f.date[2], f.date[3]);
                gl.bind_vertex_array(Some(self.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
            }
            None => {
                // No working script: a plain dark gray desktop.
                gl.clear_color(0.16, 0.15, 0.15, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
        }
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    /// Stretch the offscreen buffer over the whole current window (`w` × `h` pixels).
    pub unsafe fn present(&self, gl: &glow::Context, w: i32, h: i32) {
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
        let filter = if self.size == (w, h) { glow::NEAREST } else { glow::LINEAR };
        gl.blit_framebuffer(0, 0, self.size.0, self.size.1, 0, 0, w, h, glow::COLOR_BUFFER_BIT, filter);
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
    }

    /// Free the GPU objects. The pipeline must not be used afterwards.
    pub unsafe fn destroy(&mut self, gl: &glow::Context) {
        if let Some((program, _)) = self.program.take() {
            gl.delete_program(program);
        }
        gl.delete_vertex_array(self.vao);
        gl.delete_framebuffer(self.fbo);
        gl.delete_renderbuffer(self.rbo);
    }

    /// Read the offscreen buffer back as top-down RGBA rows.
    pub unsafe fn read_rgba(&self, gl: &glow::Context) -> Vec<u8> {
        let (w, h) = (self.size.0 as usize, self.size.1 as usize);
        let mut raw = vec![0u8; w * h * 4];
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut raw)));
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        let mut out = Vec::with_capacity(raw.len());
        for row in raw.chunks_exact(w * 4).rev() {
            out.extend_from_slice(row);
        }
        out
    }
}

unsafe fn compile_stage(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, String> {
    let sh = gl.create_shader(kind)?;
    gl.shader_source(sh, src);
    gl.compile_shader(sh);
    if gl.get_shader_compile_status(sh) {
        Ok(sh)
    } else {
        let log = gl.get_shader_info_log(sh);
        gl.delete_shader(sh);
        Err(log)
    }
}
