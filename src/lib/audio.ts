/** Tiny Web Audio synth — no asset files required. */

import type { AmbientSpec, VoiceSpec } from "./soundscapes";

// Everything routes through master -> compressor -> destination,
// so volume is one gain and mashing many keys can't clip.
let ctx: AudioContext | null = null;
let master: GainNode | null = null;
let volume = 0.8;
let unlocked = false;

function getContext(): AudioContext {
  if (!ctx) {
    ctx = new AudioContext();
    const compressor = ctx.createDynamicsCompressor();
    compressor.connect(ctx.destination);
    master = ctx.createGain();
    master.gain.value = volume;
    master.connect(compressor);
  }
  if (ctx.state === "suspended") {
    void ctx.resume();
  }
  return ctx;
}

function getMaster(): GainNode {
  getContext();
  return master!;
}

/** Resume audio after a user gesture (browser autoplay policy). */
export function unlockAudio(): void {
  getContext();
  if (!unlocked) {
    unlocked = true;
    syncAmbient();
  }
}

/** 0..1; applies to smash voices, ambience and chimes alike. */
export function setVolume(next: number): void {
  volume = next;
  if (ctx && master) {
    master.gain.setTargetAtTime(next, ctx.currentTime, 0.05);
  }
}

// ---------------------------------------------------------------------------
// Shared building blocks

let noiseBuffer: AudioBuffer | null = null;

function getNoise(audio: AudioContext): AudioBuffer {
  if (!noiseBuffer) {
    noiseBuffer = audio.createBuffer(1, audio.sampleRate * 2, audio.sampleRate);
    const data = noiseBuffer.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
  }
  return noiseBuffer;
}

// One delay line per echo setting, created lazily and kept for the app's lifetime.
const echoes = new Map<string, AudioNode>();

function getEcho(audio: AudioContext, echo: NonNullable<VoiceSpec["echo"]>): AudioNode {
  const key = `${echo.time}:${echo.feedback}`;
  let input = echoes.get(key);
  if (!input) {
    const delay = audio.createDelay(1);
    delay.delayTime.value = echo.time;
    const feedback = audio.createGain();
    feedback.gain.value = echo.feedback;
    const wet = audio.createGain();
    wet.gain.value = 0.5;
    delay.connect(feedback);
    feedback.connect(delay);
    delay.connect(wet);
    wet.connect(getMaster());
    input = delay;
    echoes.set(key, input);
  }
  return input;
}

interface Blip {
  wave: OscillatorType;
  freq: number;
  level: number;
  attack: number;
  /** Seconds at full level before the decay starts. */
  hold: number;
  decay: number;
  sweep: number;
  sweepTime: number;
}

/** One enveloped oscillator note. Returns the oscillator so callers can track it. */
function blip(audio: AudioContext, dest: AudioNode, when: number, b: Blip): OscillatorNode {
  const osc = audio.createOscillator();
  const gain = audio.createGain();
  osc.type = b.wave;
  osc.frequency.setValueAtTime(b.freq, when);
  if (b.sweep !== 1) {
    osc.frequency.exponentialRampToValueAtTime(b.freq * b.sweep, when + b.sweepTime);
  }

  const peak = when + b.attack;
  gain.gain.setValueAtTime(0.0001, when);
  gain.gain.exponentialRampToValueAtTime(b.level, peak);
  gain.gain.setValueAtTime(b.level, peak + b.hold);
  gain.gain.exponentialRampToValueAtTime(0.0001, peak + b.hold + b.decay);

  osc.connect(gain);
  gain.connect(dest);
  osc.start(when);
  osc.stop(peak + b.hold + b.decay + 0.03);
  return osc;
}

function playVoice(
  audio: AudioContext,
  dest: AudioNode,
  voice: VoiceSpec,
  seed: number,
  when: number,
): OscillatorNode[] {
  const n = Math.abs(seed);
  const freq = voice.scale[n % voice.scale.length]!;
  const wave = voice.waves[n % voice.waves.length]!;
  const base: Blip = {
    wave,
    freq,
    level: voice.level,
    attack: 0.02,
    hold: 0,
    decay: voice.decay,
    sweep: voice.sweep,
    sweepTime: voice.sweepTime,
  };

  let out = dest;
  if (voice.echo) {
    const split = audio.createGain();
    split.connect(dest);
    split.connect(getEcho(audio, voice.echo));
    out = split;
  }

  switch (voice.kind) {
    case "tone":
      return [blip(audio, out, when, base)];
    case "chirp": {
      const gap = voice.decay + voice.sweepTime * 0.5;
      return [
        blip(audio, out, when, { ...base, attack: 0.01 }),
        blip(audio, out, when + gap, { ...base, attack: 0.01, freq: freq * 1.12 }),
      ];
    }
    case "pluck":
      return [
        blip(audio, out, when, { ...base, attack: 0.005 }),
        blip(audio, out, when, {
          ...base,
          wave: "sine",
          freq: freq * 2,
          attack: 0.005,
          level: voice.level * 0.4,
          decay: voice.decay * 0.6,
        }),
      ];
    case "horn": {
      // Square/saw are harsh on their own; round them off like a toy horn.
      const lowpass = audio.createBiquadFilter();
      lowpass.type = "lowpass";
      lowpass.frequency.value = 1800;
      lowpass.connect(out);
      const beep = { ...base, attack: 0.01, hold: 0.08, decay: voice.decay, sweep: 1 };
      return [
        blip(audio, lowpass, when, beep),
        blip(audio, lowpass, when + 0.08 + voice.decay + 0.05, beep),
      ];
    }
  }
}

// ---------------------------------------------------------------------------
// Smash hits

// Oscillators, not hits: a mashing baby can fire dozens of keys per second.
const MAX_OSCILLATORS = 16;
let activeOscillators = 0;

export function playSmash(seed: number, voice: VoiceSpec): void {
  if (activeOscillators >= MAX_OSCILLATORS) return;
  const audio = getContext();
  const oscs = playVoice(audio, getMaster(), voice, seed, audio.currentTime);
  activeOscillators += oscs.length;
  for (const osc of oscs) {
    osc.onended = () => {
      activeOscillators--;
    };
  }
}

export function playUnlockChime(): void {
  const audio = getContext();
  const now = audio.currentTime;
  for (let i = 0; i < 3; i++) {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.type = "sine";
    osc.frequency.value = 523.25 * (1 + i * 0.25);
    const t = now + i * 0.08;
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(0.18, t + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.25);
    osc.connect(gain);
    gain.connect(getMaster());
    osc.start(t);
    osc.stop(t + 0.28);
  }
}

// ---------------------------------------------------------------------------
// Ambient beds

const AMBIENT_FADE_IN = 1.5;
const AMBIENT_FADE_OUT = 0.5;

interface Bed {
  spec: AmbientSpec;
  stop(): void;
}

let wantedAmbient: AmbientSpec | null = null;
let ambientPaused = false;
let bed: Bed | null = null;

/** Wanted bed, or null for silence. Starts once audio is unlocked by a gesture. */
export function setAmbient(spec: AmbientSpec | null): void {
  wantedAmbient = spec;
  syncAmbient();
}

/** Stop the bed while the window is hidden; resume it when visible again. */
export function setAmbientPaused(paused: boolean): void {
  ambientPaused = paused;
  syncAmbient();
}

function syncAmbient(): void {
  const next = unlocked && !ambientPaused ? wantedAmbient : null;
  if (bed?.spec === next) return;
  // Stop first so two beds never stack; the fade-out overlaps the new fade-in.
  bed?.stop();
  bed = next ? startBed(next) : null;
}

/** Sine LFO wobbling `target` by ±depth around its current value. */
function lfo(
  audio: AudioContext,
  rate: number,
  depth: number,
  target: AudioParam,
): OscillatorNode {
  const osc = audio.createOscillator();
  const amount = audio.createGain();
  osc.frequency.value = rate;
  amount.gain.value = depth;
  osc.connect(amount);
  amount.connect(target);
  return osc;
}

function noiseSource(audio: AudioContext): AudioBufferSourceNode {
  const src = audio.createBufferSource();
  src.buffer = getNoise(audio);
  src.loop = true;
  return src;
}

function lowpass(audio: AudioContext, cutoff: number): BiquadFilterNode {
  const filter = audio.createBiquadFilter();
  filter.type = "lowpass";
  filter.frequency.value = cutoff;
  return filter;
}

function startBed(spec: AmbientSpec): Bed {
  const audio = getContext();
  const now = audio.currentTime;
  const out = audio.createGain();
  out.gain.setValueAtTime(0, now);
  out.gain.linearRampToValueAtTime(spec.level, now + AMBIENT_FADE_IN);
  out.connect(getMaster());

  const sources: AudioScheduledSourceNode[] = [];
  let timer: ReturnType<typeof setTimeout> | null = null;

  switch (spec.kind) {
    case "pad": {
      const filter = lowpass(audio, spec.cutoff);
      // Normalize so spec.level means the same regardless of how many notes the pad has.
      const mix = audio.createGain();
      mix.gain.value = 1 / (spec.freqs.length * 2);
      filter.connect(mix);
      mix.connect(out);
      sources.push(lfo(audio, 0.07, spec.cutoff * 0.5, filter.frequency));
      // Two slightly detuned oscillators per note give the pad slow movement.
      for (const freq of spec.freqs) {
        for (const detune of [-6, 6]) {
          const osc = audio.createOscillator();
          osc.type = spec.wave;
          osc.frequency.value = freq;
          osc.detune.value = detune;
          osc.connect(filter);
          sources.push(osc);
        }
      }
      break;
    }
    case "waves": {
      const noise = noiseSource(audio);
      const filter = lowpass(audio, spec.cutoff);
      const swell = audio.createGain();
      swell.gain.value = 0.5;
      noise.connect(filter);
      filter.connect(swell);
      swell.connect(out);
      // One slow swell, both louder and brighter as each wave rolls in.
      const rate = 1 / spec.period;
      sources.push(
        noise,
        lfo(audio, rate, 0.45, swell.gain),
        lfo(audio, rate, spec.cutoff * 0.4, filter.frequency),
      );
      break;
    }
    case "rumble": {
      const noise = noiseSource(audio);
      const filter = lowpass(audio, spec.cutoff);
      noise.connect(filter);
      filter.connect(out);
      const engine = audio.createOscillator();
      const engineGain = audio.createGain();
      engine.frequency.value = spec.freq;
      engineGain.gain.value = 0.6;
      engine.connect(engineGain);
      engineGain.connect(out);
      sources.push(noise, engine, lfo(audio, 0.2, 3, engine.frequency));
      break;
    }
    case "events": {
      const schedule = () => {
        const gap = spec.minGap + Math.random() * (spec.maxGap - spec.minGap);
        timer = setTimeout(() => {
          const seed = Math.floor(Math.random() * 1000);
          playVoice(audio, out, spec.voice, seed, audio.currentTime + 0.05);
          schedule();
        }, gap * 1000);
      };
      schedule();
      break;
    }
  }

  for (const src of sources) src.start(now);

  return {
    spec,
    stop() {
      if (timer) clearTimeout(timer);
      const t = audio.currentTime;
      out.gain.cancelScheduledValues(t);
      out.gain.setValueAtTime(out.gain.value, t);
      out.gain.linearRampToValueAtTime(0, t + AMBIENT_FADE_OUT);
      for (const src of sources) src.stop(t + AMBIENT_FADE_OUT + 0.05);
      setTimeout(() => out.disconnect(), (AMBIENT_FADE_OUT + 0.1) * 1000);
    },
  };
}
