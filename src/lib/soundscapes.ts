import type { ThemeId } from "./themes";

/**
 * How a voice turns one smash into sound:
 * - tone: one oscillator with an optional pitch sweep
 * - chirp: two quick rising tones ("tweet-tweet")
 * - pluck: fast attack plus a quieter octave overtone (marimba / music box)
 * - horn: two held, low-passed beeps ("beep-beep")
 */
export type VoiceKind = "tone" | "chirp" | "pluck" | "horn";

export interface VoiceSpec {
  kind: VoiceKind;
  /** Notes in Hz; a hit's seed picks one. */
  scale: number[];
  /** Oscillator types; a hit's seed picks one. */
  waves: OscillatorType[];
  /** Peak gain of a single hit, before the master volume. */
  level: number;
  /** Seconds until one note has faded out. */
  decay: number;
  /** Pitch multiplier reached by `sweepTime` (1 = steady pitch). */
  sweep: number;
  sweepTime: number;
  /** Shared feedback delay the hit is also sent to. */
  echo?: { time: number; feedback: number };
}

/** A quiet background bed. `level` is its gain before the master volume. */
export type AmbientSpec =
  | { kind: "pad"; level: number; freqs: number[]; wave: OscillatorType; cutoff: number }
  | { kind: "waves"; level: number; period: number; cutoff: number }
  | { kind: "rumble"; level: number; freq: number; cutoff: number }
  | { kind: "events"; level: number; voice: VoiceSpec; minGap: number; maxGap: number };

export interface Soundscape {
  voice: VoiceSpec;
  ambient: AmbientSpec | null;
}

const C_PENTATONIC = [261.63, 293.66, 329.63, 392.0, 440.0, 523.25, 587.33];
const C_PENTATONIC_HIGH = [523.25, 587.33, 659.25, 783.99, 880.0, 1046.5];

// Record (not Partial) so adding a ThemeId without a soundscape fails `npm run check`.
export const SOUNDSCAPES: Record<ThemeId, Soundscape> = {
  default: {
    voice: {
      kind: "tone",
      scale: C_PENTATONIC,
      waves: ["triangle", "sine"],
      level: 0.28,
      decay: 0.35,
      sweep: 1.35,
      sweepTime: 0.12,
    },
    ambient: { kind: "pad", level: 0.05, freqs: [130.81, 196.0, 261.63], wave: "sine", cutoff: 900 },
  },
  animals: {
    voice: {
      kind: "chirp",
      scale: C_PENTATONIC_HIGH,
      waves: ["sine", "triangle"],
      level: 0.22,
      decay: 0.09,
      sweep: 1.6,
      sweepTime: 0.07,
    },
    ambient: {
      kind: "events",
      level: 0.35,
      minGap: 2,
      maxGap: 6,
      voice: {
        kind: "chirp",
        scale: [1318.51, 1567.98, 1760.0, 2093.0],
        waves: ["sine"],
        level: 0.2,
        decay: 0.06,
        sweep: 1.3,
        sweepTime: 0.05,
      },
    },
  },
  ocean: {
    voice: {
      kind: "tone",
      scale: [196.0, 220.0, 261.63, 293.66, 329.63, 392.0],
      waves: ["sine"],
      level: 0.3,
      decay: 0.22,
      sweep: 2.4,
      sweepTime: 0.08,
    },
    ambient: { kind: "waves", level: 0.09, period: 9, cutoff: 500 },
  },
  space: {
    voice: {
      kind: "tone",
      scale: C_PENTATONIC_HIGH,
      waves: ["sine"],
      level: 0.22,
      decay: 0.6,
      sweep: 1.02,
      sweepTime: 0.3,
      echo: { time: 0.28, feedback: 0.45 },
    },
    ambient: { kind: "pad", level: 0.05, freqs: [65.41, 98.0, 130.81], wave: "sawtooth", cutoff: 380 },
  },
  fruit: {
    voice: {
      kind: "pluck",
      scale: C_PENTATONIC_HIGH,
      waves: ["triangle"],
      level: 0.3,
      decay: 0.25,
      sweep: 1,
      sweepTime: 0.01,
    },
    ambient: {
      kind: "events",
      level: 0.3,
      minGap: 0.6,
      maxGap: 2.2,
      voice: {
        kind: "pluck",
        scale: [1046.5, 1174.66, 1318.51, 1567.98, 1760.0],
        waves: ["sine"],
        level: 0.15,
        decay: 0.6,
        sweep: 1,
        sweepTime: 0.01,
      },
    },
  },
  vehicles: {
    voice: {
      kind: "horn",
      scale: [261.63, 293.66, 329.63, 392.0, 440.0],
      waves: ["square", "sawtooth"],
      level: 0.14,
      decay: 0.12,
      sweep: 1,
      sweepTime: 0.01,
    },
    ambient: { kind: "rumble", level: 0.08, freq: 48, cutoff: 160 },
  },
};
