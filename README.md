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

### Settings

Top-level constants marked with a comment get a slider or colour picker under the preview:

```glsl
float speed = 1.0;                      // @slider 0.0 3.0 Speed
int count = 12;                         // @slider 1 40 Number of lights
vec3 skyColor = vec3(0.10, 0.12, 0.20); // @color Sky
```

The file stays ordinary GLSL, so it runs anywhere. Moving a control rewrites the number in the code (undoable, saved with the file). The preview turns these lines into uniforms so dragging does not recompile; if a constant is used where only constants are allowed, such as an array size, the preview recompiles on each change instead. Parsing lives in `src/lib/params.ts`.

## Screens

Each screen gets its own window behind the icons and draws the shader at its own size and shape. `config.json` holds `active_shader` (every screen) and `screens` (per-screen choices by monitor id; `""` means the normal Windows wallpaper on that screen). Screens with the same shader and size share one render. A screen covered by a maximized window stops drawing on its own.

## Develop

```bash
npm install
npm run app:dev      # the full app with hot reload
npm run dev          # interface only, in a browser, with a stand-in backend
npm run app:build    # installer in target/release/bundle/nsis/
```

## Release

Installed copies update themselves from GitHub releases. Each release needs the installer and a `latest.json`, both made by:

```bash
.\scripts\release.ps1 -Notes "What changed"            # build and sign; add -Publish to create the GitHub release
```

Bump the version in `src-tauri/tauri.conf.json`, both `Cargo.toml` files and `package.json` first. Updates are signed with the private key at `%USERPROFILE%\.tauri\wallshaders.key`; the matching public key is in `tauri.conf.json`. Keep a backup of that key: without it no installed copy will accept a new version.

## Test and debug

```bash
npm test             # Rust unit tests and the settings parser tests
npm run check        # type-check the interface
```

The renderer can be driven without showing anything:

```bash
# Draw one frame to a PNG
cargo run -p wallshaders-renderer -- --headless "shaders/Aurora.glsl" --out frame.png --size 1280x720 --time 5

# Compile a script; errors go to stderr with script line numbers, exit code 1
cargo run -p wallshaders-renderer -- --check my-shader.glsl
```

`--renderer --debug` writes the measured frame rate, screens and pause state to `wallshaders.log` every five seconds. `--monitors` prints the screens and their ids as JSON.
