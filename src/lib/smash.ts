import type { Theme } from "./themes";

export type SmashShape = "circle" | "square" | "triangle" | "star" | "letter";

export interface SmashBurst {
  id: number;
  x: number;
  y: number;
  label: string;
  shape: SmashShape;
  /** Set for emoji themes; `label` then becomes a small letter badge (or ""). */
  emoji: string | null;
  color: string;
  size: number;
  rotate: number;
}

const SHAPES: SmashShape[] = ["circle", "square", "triangle", "star", "letter"];

let nextId = 1;

export function burstFromKey(
  key: string,
  theme: Theme,
  x?: number,
  y?: number,
  badge = true,
): SmashBurst {
  const printable =
    key.length === 1 && key.trim() !== "" ? key.toUpperCase() : key === " " ? "★" : "•";
  const seed = Math.abs(key.charCodeAt(0) || nextId);
  // Same key → same emoji, so the baby gets consistent cause and effect.
  const emoji = theme.emoji ? theme.emoji[seed % theme.emoji.length]! : null;
  return {
    id: nextId++,
    x: x ?? 12 + Math.random() * 76,
    y: y ?? 14 + Math.random() * 62,
    label: emoji ? (badge && /^[A-Z0-9]$/.test(printable) ? printable : "") : printable,
    shape: SHAPES[seed % SHAPES.length]!,
    emoji,
    color: theme.colors[(seed * 7) % theme.colors.length]!,
    size: (emoji ? 88 : 64) + (seed % 72),
    rotate: ((seed * 13) % 40) - 20,
  };
}

export function burstFromPointer(clientX: number, clientY: number, theme: Theme): SmashBurst {
  const vw = window.innerWidth || 1;
  const vh = window.innerHeight || 1;
  const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  const label = letters[Math.floor(Math.random() * letters.length)]!;
  // Random letter badges on pokes are noise; only show them for real keys.
  return burstFromKey(label, theme, (clientX / vw) * 100, (clientY / vh) * 100, false);
}

export function keySeed(key: string): number {
  return key.charCodeAt(0) || 1;
}
