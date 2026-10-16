import { invoke } from "@tauri-apps/api/core";

/** True when running inside a Tauri webview (not plain Vite browser). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function applyBabyLock(locked: boolean): Promise<void> {
  if (!isTauri()) {
    // Browser fallback: Fullscreen API only (no OS shortcut capture).
    if (locked) {
      await document.documentElement.requestFullscreen?.().catch(() => undefined);
    } else if (document.fullscreenElement) {
      await document.exitFullscreen?.().catch(() => undefined);
    }
    return;
  }
  await invoke("set_baby_lock", { locked });
}

export async function applyFullscreen(fullscreen: boolean): Promise<void> {
  if (!isTauri()) {
    if (fullscreen) {
      await document.documentElement.requestFullscreen?.().catch(() => undefined);
    } else if (document.fullscreenElement) {
      await document.exitFullscreen?.().catch(() => undefined);
    }
    return;
  }
  await invoke("set_fullscreen", { fullscreen });
}
