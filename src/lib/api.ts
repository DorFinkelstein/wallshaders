// Everything the UI asks of the backend goes through here. Inside the app it
// calls the Rust commands; in a plain browser (used for UI testing) it falls
// back to an in-memory stand-in so the whole interface still works.

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export interface Config {
  active_shader: string | null;
  /** Per-screen choices by screen id; "" means the normal Windows wallpaper. */
  screens: Record<string, string>;
  fps: number;
  render_scale: number;
  pause_fullscreen: boolean;
  pause_maximized: boolean;
  battery_mode: "none" | "half" | "pause";
  mouse_input: boolean;
  /** Look for a new version when the app starts. */
  check_updates: boolean;
}

export interface ScreenStatus {
  id: string;
  name: string;
  width: number;
  height: number;
  primary: boolean;
  /** What the screen shows now; null means the normal Windows wallpaper. */
  shader: string | null;
}

export interface Status {
  running: boolean;
  /** The shader shared by screens without their own choice. */
  active: string | null;
  /** Left to right. */
  screens: ScreenStatus[];
}

export const inApp = "__TAURI_INTERNALS__" in window;

// ---- stand-in backend ------------------------------------------------------

const builtins = import.meta.glob("../../shaders/*.glsl", { query: "?raw", import: "default", eager: true }) as Record<
  string,
  string
>;

const mock = {
  shaders: new Map<string, string>(
    Object.entries(builtins).map(([path, src]) => [path.split("/").pop()!.replace(/\.glsl$/, ""), src]),
  ),
  // Two pretend screens so the screen picker can be tried in a browser.
  monitors: [
    { id: "side", name: "Side screen", width: 2560, height: 1080, primary: false },
    {
      id: "main",
      name: "Main screen",
      width: window.screen.width > 0 ? window.screen.width : 1920,
      height: window.screen.height > 0 ? window.screen.height : 1080,
      primary: true,
    },
  ],
  config: {
    active_shader: null,
    screens: {},
    fps: 30,
    render_scale: 0.75,
    pause_fullscreen: true,
    pause_maximized: true,
    battery_mode: "half",
    mouse_input: false,
    check_updates: true,
  } as Config,
  running: false,
  autostart: false,
};

function mockShaderFor(id: string): string | null {
  const own = mock.config.screens[id];
  if (own !== undefined) return own || null;
  return mock.config.active_shader;
}

function mockAnyShader(): boolean {
  return mock.config.active_shader !== null || Object.values(mock.config.screens).some((s) => s);
}

function mockForget(name: string) {
  if (mock.config.active_shader === name) mock.config.active_shader = null;
  for (const [id, s] of Object.entries(mock.config.screens)) if (s === name) delete mock.config.screens[id];
  if (!mockAnyShader()) mock.running = false;
}

function checkName(name: string): string {
  const n = name.trim();
  if (!n) throw "The name cannot be empty.";
  if (!/^[\p{L}\p{N} \-_()]+$/u.test(n)) throw "Use only letters, numbers, spaces, - _ ( ) in the name.";
  return n;
}

async function mockCall(cmd: string, a: Record<string, any>): Promise<any> {
  const m = mock;
  switch (cmd) {
    case "list_shaders":
      return [...m.shaders.keys()].sort((x, y) => x.toLowerCase().localeCompare(y.toLowerCase()));
    case "read_shader":
      if (!m.shaders.has(a.name)) throw `Could not open "${a.name}".`;
      return m.shaders.get(a.name);
    case "save_shader":
      m.shaders.set(checkName(a.name), a.source);
      return;
    case "delete_shader":
      m.shaders.delete(a.name);
      mockForget(a.name);
      return;
    case "rename_shader": {
      const next = checkName(a.newName);
      if (next !== a.name && m.shaders.has(next)) throw `A shader named "${next}" already exists.`;
      const src = m.shaders.get(a.name)!;
      m.shaders.delete(a.name);
      m.shaders.set(next, src);
      if (m.config.active_shader === a.name) m.config.active_shader = next;
      for (const [id, s] of Object.entries(m.config.screens)) if (s === a.name) m.config.screens[id] = next;
      return next;
    }
    case "restore_builtins":
      for (const [path, src] of Object.entries(builtins)) {
        m.shaders.set(path.split("/").pop()!.replace(/\.glsl$/, ""), src);
      }
      return;
    case "get_config":
      return { ...m.config };
    case "set_config":
      m.config = { ...a.config, active_shader: m.config.active_shader, screens: m.config.screens };
      return { ...m.config };
    case "apply_wallpaper":
      if (!m.shaders.get(a.name)?.includes("mainImage")) throw "0(1) : error: mainImage is not defined";
      if (a.screen) {
        m.config.screens[a.screen] = a.name;
      } else {
        m.config.active_shader = a.name;
        m.config.screens = {};
      }
      m.running = true;
      return;
    case "stop_wallpaper":
      if (a.screen) {
        m.config.screens[a.screen] = "";
      } else {
        m.config.active_shader = null;
        m.config.screens = {};
      }
      if (!mockAnyShader()) m.running = false;
      return;
    case "wallpaper_status":
      return {
        running: m.running,
        active: m.running ? m.config.active_shader : null,
        screens: m.monitors.map((s) => ({ ...s, shader: m.running ? mockShaderFor(s.id) : null })),
      };
    case "get_autostart":
      return m.autostart;
    case "set_autostart":
      m.autostart = a.enabled;
      return;
    default:
      throw `Unknown command ${cmd}`;
  }
}

function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return inApp ? invoke<T>(cmd, args) : mockCall(cmd, args);
}

// ---- commands --------------------------------------------------------------

export const listShaders = () => call<string[]>("list_shaders");
export const readShader = (name: string) => call<string>("read_shader", { name });
export const saveShader = (name: string, source: string) => call<void>("save_shader", { name, source });
export const deleteShader = (name: string) => call<void>("delete_shader", { name });
export const renameShader = (name: string, newName: string) => call<string>("rename_shader", { name, newName });
export const restoreBuiltins = () => call<void>("restore_builtins");
export const getConfig = () => call<Config>("get_config");
export const setConfig = (config: Config) => call<Config>("set_config", { config });
/** Show a shader on every screen (`screen` = null) or on one screen. */
export const applyWallpaper = (name: string, screen: string | null) => call<void>("apply_wallpaper", { name, screen });
/** Stop everywhere (`screen` = null), or give one screen back its normal wallpaper. */
export const stopWallpaper = (screen: string | null = null) => call<void>("stop_wallpaper", { screen });
export const wallpaperStatus = () => call<Status>("wallpaper_status");
export const getAutostart = () => call<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) => call<void>("set_autostart", { enabled });

const FILTERS = [{ name: "Shader scripts", extensions: ["glsl", "frag", "fs", "txt"] }];

/** Let the user pick shader files and copy them into the library. Returns the new names. */
export async function importShaders(): Promise<string[]> {
  if (!inApp) {
    const name = `Imported ${mock.shaders.size + 1}`;
    mock.shaders.set(name, mock.shaders.get("Warm Plasma") ?? "");
    return [name];
  }
  const picked = await open({ multiple: true, filters: FILTERS, title: "Import shaders" });
  if (!picked) return [];
  const names: string[] = [];
  for (const path of Array.isArray(picked) ? picked : [picked]) {
    names.push(await invoke<string>("import_shader", { path }));
  }
  return names;
}

/** Save a copy of a shader anywhere on disk. Returns false if the user cancelled. */
export async function exportShader(name: string): Promise<boolean> {
  if (!inApp) return true;
  const path = await save({ defaultPath: `${name}.glsl`, filters: FILTERS, title: "Export shader" });
  if (!path) return false;
  await invoke("export_shader", { name, path });
  return true;
}

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
