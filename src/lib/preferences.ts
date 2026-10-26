import { isThemeId, type ThemeId } from "./themes";

export type ColorMode = "system" | "light" | "dark";
export type ResolvedMode = "light" | "dark";

export interface Preferences {
  mode: ColorMode;
  theme: ThemeId;
  /** Background bed from the theme's soundscape. Off by default: continuous sound is opt-in. */
  ambience: boolean;
  /** Master volume, 0..1. */
  volume: number;
  /** Bottom-right controls panel minimised to a small reopen button. */
  panelCollapsed: boolean;
}

// Keep in sync with the inline pre-paint script in src/app.html.
const PREFS_KEY = "babysmash-preferences";

export const DEFAULT_PREFERENCES: Preferences = {
  mode: "system",
  theme: "default",
  ambience: false,
  volume: 0.8,
  panelCollapsed: false,
};

function isColorMode(value: unknown): value is ColorMode {
  return value === "system" || value === "light" || value === "dark";
}

function isVolume(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1;
}

/** localStorage (not sessionStorage like lock intent): preferences should survive relaunches. */
export function loadPreferences(): Preferences {
  try {
    const raw = JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}");
    return {
      mode: isColorMode(raw?.mode) ? raw.mode : DEFAULT_PREFERENCES.mode,
      theme: isThemeId(raw?.theme) ? raw.theme : DEFAULT_PREFERENCES.theme,
      ambience: typeof raw?.ambience === "boolean" ? raw.ambience : DEFAULT_PREFERENCES.ambience,
      volume: isVolume(raw?.volume) ? raw.volume : DEFAULT_PREFERENCES.volume,
      panelCollapsed:
        typeof raw?.panelCollapsed === "boolean"
          ? raw.panelCollapsed
          : DEFAULT_PREFERENCES.panelCollapsed,
    };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function savePreferences(prefs: Preferences): void {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // ignore quota / private-mode failures
  }
}

export function resolveMode(mode: ColorMode, systemDark: boolean): ResolvedMode {
  if (mode === "system") return systemDark ? "dark" : "light";
  return mode;
}

/** Theme + mode are driven by data attributes on <html>; app.css keys off them. */
export function applyPreferences(prefs: Preferences, systemDark: boolean): void {
  const root = document.documentElement;
  root.dataset.theme = prefs.theme;
  root.dataset.mode = resolveMode(prefs.mode, systemDark);
}
