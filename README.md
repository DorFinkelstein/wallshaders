# Wallshaders

Shader wallpapers for Windows 10 and 11. Write or import a GLSL script, preview it live, and put it behind your desktop icons.

## Layout

| Path | What it is |
|---|---|
| `renderer/` | Native wallpaper renderer (Rust, OpenGL 3.3, raw Win32). Also builds `ws-renderer.exe` for testing. |
| `src-tauri/` | The app shell (Tauri 2): shader library, settings, control of the renderer. |
| `src/` | The interface (Svelte 5, CodeMirror 6, WebGL2 preview). |
| `shaders/` | Starter shaders, copied into the user's library on first run. |

The app and the wallpaper are one executable. `wallshaders.exe` opens the editor; `wallshaders.exe --renderer` is the wallpaper and loads none of the UI.

User data lives in `%APPDATA%\Wallshaders` (`shaders\*.glsl`, `config.json`, `wallshaders.log`). Set `WALLSHADERS_DATA_DIR` to use a different folder, which is how tests avoid touching real data.

## Shader format

Scripts use the Shadertoy convention:

```glsl
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}
```

Available inputs: `iResolution`, `iTime`, `iTimeDelta`, `iFrameRate`, `iFrame`, `iMouse`, `iDate`. Textures, audio and multi-pass buffers are not supported.

## Develop

```bash
npm install
npm run app:dev      # the full app with hot reload
npm run dev          # interface only, in a browser, with a stand-in backend
npm run app:build    # installer in target/release/bundle/nsis/
```

## Test and debug

```bash
npm test             # Rust unit tests
npm run check        # type-check the interface
```

The renderer can be driven without showing anything:

```bash
# Draw one frame to a PNG
cargo run -p wallshaders-renderer -- --headless "shaders/Aurora.glsl" --out frame.png --size 1280x720 --time 5

# Compile a script; errors go to stderr with script line numbers, exit code 1
cargo run -p wallshaders-renderer -- --check my-shader.glsl
```

`--renderer --debug` writes the measured frame rate and pause state to `wallshaders.log` every five seconds.
