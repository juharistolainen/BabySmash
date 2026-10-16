import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const LOCK_KEY = "babysmash-baby-lock";

export const ERR_NEEDS_ACCESSIBILITY = "needs_accessibility";

export type MacosKeyboardLockStatus = {
  armed: boolean;
  accessibilityGranted: boolean;
  secureInput: boolean;
  supported: boolean;
};

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

/** Normalize Tauri/invoke errors into a stable string code or message. */
export function lockErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

export function isNeedsAccessibility(error: unknown): boolean {
  return lockErrorMessage(error).includes(ERR_NEEDS_ACCESSIBILITY);
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

export async function fetchMacosKeyboardLockStatus(): Promise<MacosKeyboardLockStatus | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<MacosKeyboardLockStatus>("macos_keyboard_lock_status");
  } catch {
    return null;
  }
}

export async function openAccessibilitySettings(): Promise<void> {
  if (!isTauri()) {
    throw new Error("Accessibility settings are only available in the desktop app");
  }
  await invoke("open_accessibility_settings");
}

export type NativeBuildInfo = {
  name: string;
  version: string;
  /** Unix seconds. */
  buildTime: number;
  profile: string;
  commit: string | null;
  rustc: string;
  target: string;
  tauri: string;
  webview: string | null;
  os: string;
  arch: string;
};

export async function fetchBuildInfo(): Promise<NativeBuildInfo | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<NativeBuildInfo>("app_build_info");
  } catch {
    return null;
  }
}

/** App menu (macOS) or Help (Windows/Linux) > About. Returns an unsubscribe function. */
export async function onOpenAboutMenu(handler: () => void): Promise<() => void> {
  if (!isTauri()) return () => undefined;
  return listen("open-about", handler);
}

/** File > Preferences… in the native menu. Returns an unsubscribe function. */
export async function onOpenPreferencesMenu(handler: () => void): Promise<() => void> {
  if (!isTauri()) return () => undefined;
  return listen("open-preferences", handler);
}
