// Turns a Shadertoy-style script into a complete WebGL2 fragment shader for the
// preview. Mirrors renderer/src/shader.rs, which does the same for the wallpaper.

/** Lines placed before the user's script. Keep in sync with shader.rs. */
export const HEADER_LINES = 2;

const UNIFORMS =
  "uniform vec3 iResolution; uniform float iTime; uniform float iTimeDelta; " +
  "uniform float iFrameRate; uniform int iFrame; uniform vec4 iMouse; uniform vec4 iDate; " +
  "out vec4 ws_fragColor;";

const FOOTER =
  "\nvoid main() { vec4 c = vec4(0.0, 0.0, 0.0, 1.0); " +
  "mainImage(c, gl_FragCoord.xy); ws_fragColor = vec4(c.rgb, 1.0); }\n";

export const VERTEX =
  "#version 300 es\n" +
  "void main() { vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); " +
  "gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0); }\n";

export function wrapFragment(user: string): string {
  return `#version 300 es\nprecision highp float; precision highp int; ${UNIFORMS}\n${user}${FOOTER}`;
}

export interface ShaderError {
  /** 1-based line in the user's script. */
  line: number;
  message: string;
}

/** Parse a compile log from WebGL ("ERROR: 0:12: ...") or from the native
 *  renderer, whose lines are already script-relative ("0(12) : error ..."). */
export function parseErrors(log: string, alreadyRelative = false): ShaderError[] {
  const errors: ShaderError[] = [];
  const offset = alreadyRelative ? 0 : HEADER_LINES;
  for (const raw of log.split("\n")) {
    const text = raw.trim();
    if (!text) continue;
    const m = /^(?:ERROR|WARNING):\s*\d+:(\d+):\s*(.*)$/.exec(text) ?? /^\d+\((\d+)\)\s*:\s*(.*)$/.exec(text);
    if (m) {
      errors.push({ line: Math.max(1, Number(m[1]) - offset), message: m[2] });
    } else {
      errors.push({ line: 1, message: text });
    }
  }
  return errors;
}

export const NEW_SHADER_TEMPLATE = `// A new shader. Every pixel runs mainImage once per frame.
//   fragCoord    this pixel's position, in pixels
//   iResolution  size of the picture, in pixels
//   iTime        seconds since the shader started

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;

    vec3 gray = vec3(0.20, 0.19, 0.19);
    vec3 terracotta = vec3(0.85, 0.56, 0.45);
    float wave = 0.5 + 0.5 * sin(uv.x * 6.0 + iTime * 0.5);

    fragColor = vec4(mix(gray, terracotta, uv.y * wave), 1.0);
}
`;
