/** Tiny Web Audio synth — no asset files required. */

let ctx: AudioContext | null = null;

function getContext(): AudioContext {
  if (!ctx) {
    ctx = new AudioContext();
  }
  if (ctx.state === "suspended") {
    void ctx.resume();
  }
  return ctx;
}

/** Resume audio after a user gesture (browser autoplay policy). */
export function unlockAudio(): void {
  getContext();
}

const PENTATONIC = [261.63, 293.66, 329.63, 392.0, 440.0, 523.25, 587.33];

export function playSmashTone(seed: number): void {
  const audio = getContext();
  const now = audio.currentTime;
  const freq = PENTATONIC[Math.abs(seed) % PENTATONIC.length]!;

  const osc = audio.createOscillator();
  const gain = audio.createGain();
  osc.type = seed % 2 === 0 ? "triangle" : "sine";
  osc.frequency.setValueAtTime(freq, now);
  osc.frequency.exponentialRampToValueAtTime(freq * 1.35, now + 0.12);

  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.exponentialRampToValueAtTime(0.28, now + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.35);

  osc.connect(gain);
  gain.connect(audio.destination);
  osc.start(now);
  osc.stop(now + 0.38);
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
    gain.connect(audio.destination);
    osc.start(t);
    osc.stop(t + 0.28);
  }
}
