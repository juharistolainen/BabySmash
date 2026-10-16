export type SmashShape = "circle" | "square" | "triangle" | "star" | "letter";

export interface SmashBurst {
  id: number;
  x: number;
  y: number;
  label: string;
  shape: SmashShape;
  color: string;
  size: number;
  rotate: number;
}

const COLORS = [
  "#ff6b6b",
  "#ffa94d",
  "#ffd43b",
  "#69db7c",
  "#38d9a9",
  "#4dabf7",
  "#748ffc",
  "#da77f2",
  "#f783ac",
];

const SHAPES: SmashShape[] = ["circle", "square", "triangle", "star", "letter"];

let nextId = 1;

export function burstFromKey(key: string, x?: number, y?: number): SmashBurst {
  const printable =
    key.length === 1 && key.trim() !== "" ? key.toUpperCase() : key === " " ? "★" : "•";
  const seed = key.charCodeAt(0) || nextId;
  return {
    id: nextId++,
    x: x ?? 12 + Math.random() * 76,
    y: y ?? 14 + Math.random() * 62,
    label: printable,
    shape: SHAPES[Math.abs(seed) % SHAPES.length]!,
    color: COLORS[Math.abs(seed * 7) % COLORS.length]!,
    size: 64 + (Math.abs(seed) % 72),
    rotate: (Math.abs(seed * 13) % 40) - 20,
  };
}

export function burstFromPointer(clientX: number, clientY: number): SmashBurst {
  const vw = window.innerWidth || 1;
  const vh = window.innerHeight || 1;
  const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  const label = letters[Math.floor(Math.random() * letters.length)]!;
  return burstFromKey(label, (clientX / vw) * 100, (clientY / vh) * 100);
}

export function keySeed(key: string): number {
  return key.charCodeAt(0) || 1;
}
