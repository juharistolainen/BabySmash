import { invoke } from "@tauri-apps/api/core";

const LOCK_KEY = "babysmash-baby-lock";

/** True when running inside a Tauri webview (not plain Vite browser). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Persist lock intent so a webview remount after fullscreen/decoration changes keeps UI in sync. */
export function readStoredBabyLock(): boolean {
  try {
    return sessionStorage.getItem(LOCK_KEY) === "1";
  } catch {
    return false;
  }
}

export function writeStoredBabyLock(locked: boolean): void {
  try {
    sessionStorage.setItem(LOCK_KEY, locked ? "1" : "0");
  } catch {
    // ignore quota / private-mode failures
  }
}

export async function applyBabyLock(locked: boolean): Promise<void> {
  writeStoredBabyLock(locked);
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
