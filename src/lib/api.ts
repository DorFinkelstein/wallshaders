// Everything the UI asks of the backend goes through here. Inside the app it
// calls the Rust commands; in a plain browser (used for UI testing) it falls
// back to an in-memory stand-in so the whole interface still works.

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export interface Config {
  active_shader: string | null;
  fps: number;
  render_scale: number;
  pause_fullscreen: boolean;
  pause_maximized: boolean;
  battery_mode: "none" | "half" | "pause";
  mouse_input: boolean;
}

export interface Status {
  running: boolean;
  active: string | null;
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
  config: {
    active_shader: null,
    fps: 30,
    render_scale: 0.75,
    pause_fullscreen: true,
    pause_maximized: true,
    battery_mode: "half",
    mouse_input: false,
  } as Config,
  running: false,
  autostart: false,
};

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
      if (m.config.active_shader === a.name) {
        m.config.active_shader = null;
        m.running = false;
      }
      return;
    case "rename_shader": {
      const next = checkName(a.newName);
      if (next !== a.name && m.shaders.has(next)) throw `A shader named "${next}" already exists.`;
      const src = m.shaders.get(a.name)!;
      m.shaders.delete(a.name);
      m.shaders.set(next, src);
      if (m.config.active_shader === a.name) m.config.active_shader = next;
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
      m.config = { ...a.config, active_shader: m.config.active_shader };
      return { ...m.config };
    case "apply_wallpaper":
      if (!m.shaders.get(a.name)?.includes("mainImage")) throw "0(1) : error: mainImage is not defined";
      m.config.active_shader = a.name;
      m.running = true;
      return;
    case "stop_wallpaper":
      m.running = false;
      m.config.active_shader = null;
      return;
    case "wallpaper_status":
      return { running: m.running, active: m.running ? m.config.active_shader : null };
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
export const applyWallpaper = (name: string) => call<void>("apply_wallpaper", { name });
export const stopWallpaper = () => call<void>("stop_wallpaper");
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
