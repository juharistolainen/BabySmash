export type ThemeId = "default" | "animals" | "ocean" | "space" | "fruit" | "vehicles";

export interface Theme {
  id: ThemeId;
  name: string;
  /** Short sample shown on the theme card in Preferences. */
  preview: string;
  /** Burst colors (shape fill, or the halo behind an emoji). */
  colors: string[];
  /** Rides the leading edge of the hold-Esc unlock bar. */
  unlockIcon: string;
  /** When set, bursts show one of these instead of a shape with a letter. */
  emoji: string[] | null;
}

// Backgrounds live in app.css under [data-theme="…"], with dark variants.
// Avoid ZWJ sequences and very new emoji: Linux fonts may lack the glyphs.
export const THEMES: Theme[] = [
  {
    id: "default",
    name: "Classic",
    unlockIcon: "🔓",
    preview: "A ● ■ ★",
    colors: [
      "#ff6b6b",
      "#ffa94d",
      "#ffd43b",
      "#69db7c",
      "#38d9a9",
      "#4dabf7",
      "#748ffc",
      "#da77f2",
      "#f783ac",
    ],
    emoji: null,
  },
  {
    id: "animals",
    name: "Animals",
    unlockIcon: "🐾",
    preview: "🐶🐱🦁🐸",
    colors: ["#ffb347", "#ffd166", "#a3d977", "#8ecae6", "#f4a261", "#90be6d", "#f28482"],
    emoji: [..."🐶🐱🐭🐰🦊🐻🐼🐨🐯🦁🐮🐷🐸🐵🐔🐧🐤🦆🦉🐴🦄🐝🦋🐞🐘🦒🦓🐢"],
  },
  {
    id: "ocean",
    name: "Ocean",
    unlockIcon: "🐠",
    preview: "🐠🐙🐳🦀",
    colors: ["#4cc9f0", "#4895ef", "#56cfe1", "#72efdd", "#80ffdb", "#48bfe3", "#ff9e7a"],
    emoji: [..."🐠🐟🐡🐙🦑🦀🦞🦐🐳🐋🐬🦈🦭🐚🐢🌊"],
  },
  {
    id: "space",
    name: "Space",
    unlockIcon: "🚀",
    preview: "🚀🪐🌙👽",
    colors: ["#9d4edd", "#c77dff", "#4361ee", "#4cc9f0", "#f72585", "#ffd60a", "#e0aaff"],
    emoji: ["🚀", "🛸", "🪐", "🌍", "🌎", "🌏", "🌙", "🌛", "⭐", "🌟", "✨", "☄️", "🌞", "👽", "🛰️", "🔭"],
  },
  {
    id: "fruit",
    name: "Yummy",
    unlockIcon: "🍓",
    preview: "🍎🍌🍓🧁",
    colors: ["#ff595e", "#ff924c", "#ffca3a", "#8ac926", "#52a675", "#ff70a6", "#c77dff"],
    emoji: [..."🍎🍐🍊🍋🍌🍉🍇🍓🫐🍒🍑🥭🍍🥥🥝🥕🌽🧁🍪🍩🍦🍰🧀🥨"],
  },
  {
    id: "vehicles",
    name: "Vroom",
    unlockIcon: "🚗",
    preview: "🚗🚒🚜✈️",
    colors: ["#e63946", "#f4a261", "#ffb703", "#219ebc", "#2a9d8f", "#fb8500", "#8ecae6"],
    emoji: [
      "🚗", "🚕", "🚙", "🚌", "🚎", "🏎️", "🚓", "🚑", "🚒", "🚐", "🛻", "🚚",
      "🚜", "🚲", "🛵", "🚂", "🚆", "✈️", "🚁", "⛵", "🚤", "🛳️",
    ],
  },
];

export function isThemeId(value: unknown): value is ThemeId {
  return THEMES.some((t) => t.id === value);
}

export function getTheme(id: ThemeId): Theme {
  return THEMES.find((t) => t.id === id) ?? THEMES[0]!;
}
