import { isThemeId, type ThemeId } from "./themes";

export type ColorMode = "system" | "light" | "dark";
export type ResolvedMode = "light" | "dark";

export interface Preferences {
  mode: ColorMode;
  theme: ThemeId;
}

// Keep in sync with the inline pre-paint script in src/app.html.
const PREFS_KEY = "babysmash-preferences";

export const DEFAULT_PREFERENCES: Preferences = { mode: "system", theme: "default" };

function isColorMode(value: unknown): value is ColorMode {
  return value === "system" || value === "light" || value === "dark";
}

/** localStorage (not sessionStorage like lock intent): preferences should survive relaunches. */
export function loadPreferences(): Preferences {
  try {
    const raw = JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}");
    return {
      mode: isColorMode(raw?.mode) ? raw.mode : DEFAULT_PREFERENCES.mode,
      theme: isThemeId(raw?.theme) ? raw.theme : DEFAULT_PREFERENCES.theme,
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
