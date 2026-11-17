// App updates from GitHub releases (Tauri updater). Each release carries a
// signed installer and a latest.json; the app only installs updates signed with
// the project's key.
//
// In a plain browser there is no updater; add ?fakeupdate to the address to try
// the update interface with a pretend release.

import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { inApp } from "./api";

export interface AvailableUpdate {
  version: string;
  notes: string;
  date: string | null;
  /** Download, stop the wallpaper, then run the installer. The app closes and
   *  reopens on its own once the installer finishes. */
  install: (onProgress: (fraction: number | null) => void) => Promise<void>;
}

export async function currentVersion(): Promise<string> {
  return inApp ? getVersion() : "dev";
}

function wrap(update: Update): AvailableUpdate {
  return {
    version: update.version,
    notes: update.body?.trim() ?? "",
    date: update.date ?? null,
    async install(onProgress) {
      let total = 0;
      let done = 0;
      await update.download((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          onProgress(total ? 0 : null);
        } else if (event.event === "Progress") {
          done += event.data.chunkLength;
          onProgress(total ? Math.min(1, done / total) : null);
        } else {
          onProgress(1);
        }
      });
      // The wallpaper runs from the same program file the installer replaces.
      await invoke("prepare_update");
      try {
        await update.install();
      } catch (e) {
        await invoke("resume_wallpaper").catch(() => {});
        throw e;
      }
    },
  };
}

function fake(): AvailableUpdate {
  return {
    version: "9.9.9",
    notes: "A pretend release for trying the update screen.\n\n- First change\n- Second change",
    date: new Date().toISOString(),
    async install(onProgress) {
      for (let i = 0; i <= 20; i++) {
        onProgress(i / 20);
        await new Promise((r) => setTimeout(r, 80));
      }
      throw "This is a pretend update, so nothing was installed.";
    },
  };
}

/** Look for a newer release. Returns null when this is the latest version. */
export async function findUpdate(): Promise<AvailableUpdate | null> {
  if (!inApp) return new URLSearchParams(location.search).has("fakeupdate") ? fake() : null;
  const update = await check({ timeout: 15000 });
  return update ? wrap(update) : null;
}
