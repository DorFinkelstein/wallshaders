//! Turns a Shadertoy-style script (`void mainImage(out vec4, in vec2)`) into a
//! complete fragment shader, and maps compiler error lines back to the script.

/// Number of lines placed before the user's script. Keep in sync with `src/lib/glsl.ts`.
pub const HEADER_LINES: u32 = 2;

const UNIFORMS: &str = "uniform vec3 iResolution; uniform float iTime; uniform float iTimeDelta; \
uniform float iFrameRate; uniform int iFrame; uniform vec4 iMouse; uniform vec4 iDate; \
out vec4 ws_fragColor;";

const FOOTER: &str = "\nvoid main() { vec4 c = vec4(0.0, 0.0, 0.0, 1.0); \
mainImage(c, gl_FragCoord.xy); ws_fragColor = vec4(c.rgb, 1.0); }\n";

pub const VERTEX: &str = "#version 330 core\n\
void main() { vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); \
gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0); }\n";

pub fn wrap_fragment(user: &str) -> String {
    format!("#version 330 core\n{UNIFORMS}\n{user}{FOOTER}")
}

/// A shader that reads none of the time-varying inputs only needs to be drawn once.
pub fn is_animated(user: &str) -> bool {
    ["iTime", "iFrame", "iMouse", "iDate"].iter().any(|u| user.contains(u))
}

pub fn uses_mouse(user: &str) -> bool {
    user.contains("iMouse")
}

/// Rewrite line numbers in a driver's compile log so they point into the user's script.
/// Handles the `ERROR: 0:12:` (Intel, AMD, Mesa) and `0(12) :` (NVIDIA) formats.
pub fn fix_error_lines(log: &str) -> String {
    log.lines().map(fix_line).collect::<Vec<_>>().join("\n")
}

fn fix_line(line: &str) -> String {
    for (open, close) in [("0:", ':'), ("0(", ')')] {
        if let Some(start) = line.find(open) {
            // Only accept the marker at the start or right after a space.
            if start > 0 && !line[..start].ends_with(' ') {
                continue;
            }
            let digits_at = start + open.len();
            let rest = &line[digits_at..];
            let n = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if n > 0 && rest[n..].starts_with(close) {
                if let Ok(num) = rest[..n].parse::<u32>() {
                    let fixed = num.saturating_sub(HEADER_LINES).max(1);
                    return format!("{}{}{}", &line[..digits_at], fixed, &rest[n..]);
                }
            }
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_code_starts_after_header() {
        let wrapped = wrap_fragment("USER");
        let line = wrapped.lines().position(|l| l.starts_with("USER")).unwrap() as u32;
        assert_eq!(line, HEADER_LINES);
    }

    #[test]
    fn fixes_both_log_formats() {
        assert_eq!(fix_error_lines("ERROR: 0:12: 'x' : undeclared"), "ERROR: 0:10: 'x' : undeclared");
        assert_eq!(fix_error_lines("0(7) : error C1008: undefined"), "0(5) : error C1008: undefined");
        assert_eq!(fix_error_lines("link failed"), "link failed");
    }

    #[test]
    fn detects_static_shaders() {
        assert!(!is_animated("void mainImage(out vec4 c, in vec2 p){ c = vec4(p / iResolution.xy, 0, 1); }"));
        assert!(is_animated("float t = iTime;"));
    }
}
