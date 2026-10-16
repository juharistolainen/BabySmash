<script lang="ts">
  import { onMount } from "svelte";
  import { unlockAudio, playSmashTone, playUnlockChime } from "$lib/audio";
  import { burstFromKey, burstFromPointer, keySeed, type SmashBurst } from "$lib/smash";
  import { applyBabyLock, applyFullscreen, isTauri } from "$lib/babyLock";

  const MAX_BURSTS = 40;
  const UNLOCK_HOLD_MS = 2000;

  let bursts = $state<SmashBurst[]>([]);
  let babyLocked = $state(false);
  let fullscreen = $state(false);
  let status = $state("Press any key — or poke the screen!");
  let unlockProgress = $state(0);
  let error = $state("");
  let inTauri = $state(false);

  let unlockTimer: ReturnType<typeof setTimeout> | null = null;
  let unlockStartedAt = 0;
  let unlockRaf = 0;

  onMount(() => {
    inTauri = isTauri();

    const onKeyDown = (event: KeyboardEvent) => {
      unlockAudio();

      // Parent unlock: hold Escape for 2 seconds while locked.
      if (event.key === "Escape") {
        if (babyLocked) {
          event.preventDefault();
          event.stopPropagation();
          beginUnlockHold();
          return;
        }
        // Unlocked: Esc toggles fullscreen off if on.
        if (fullscreen) {
          event.preventDefault();
          void toggleFullscreen(false);
        }
        return;
      }

      if (babyLocked) {
        // Swallow chords that would otherwise leave the app / trigger chrome.
        event.preventDefault();
        event.stopPropagation();
      }

      // Ignore lone modifier keys for visuals.
      if (["Shift", "Control", "Alt", "Meta", "CapsLock"].includes(event.key)) {
        return;
      }

      spawnBurst(burstFromKey(event.key));
      playSmashTone(keySeed(event.key));
      status = babyLocked ? "Baby Lock on — hold Esc to unlock" : "Smash!";
    };

    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        cancelUnlockHold();
      }
    };

    const onPointer = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("[data-controls]")) return;
      unlockAudio();
      spawnBurst(burstFromPointer(event.clientX, event.clientY));
      playSmashTone(Math.floor(event.clientX + event.clientY));
      status = "Poke!";
    };

    const onContext = (event: Event) => {
      if (babyLocked) event.preventDefault();
    };

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    window.addEventListener("pointerdown", onPointer);
    window.addEventListener("contextmenu", onContext);

    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      window.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("contextmenu", onContext);
      cancelUnlockHold();
    };
  });

  function spawnBurst(burst: SmashBurst) {
    bursts = [...bursts.slice(-(MAX_BURSTS - 1)), burst];
    window.setTimeout(() => {
      bursts = bursts.filter((b) => b.id !== burst.id);
    }, 2200);
  }

  function beginUnlockHold() {
    if (unlockTimer) return;
    unlockStartedAt = performance.now();
    status = "Keep holding Esc…";
    const tick = () => {
      const elapsed = performance.now() - unlockStartedAt;
      unlockProgress = Math.min(1, elapsed / UNLOCK_HOLD_MS);
      if (unlockProgress < 1) {
        unlockRaf = requestAnimationFrame(tick);
      }
    };
    unlockRaf = requestAnimationFrame(tick);
    unlockTimer = setTimeout(() => {
      void setLocked(false);
      playUnlockChime();
      cancelUnlockHold();
      status = "Unlocked — parent controls are back";
    }, UNLOCK_HOLD_MS);
  }

  function cancelUnlockHold() {
    if (unlockTimer) {
      clearTimeout(unlockTimer);
      unlockTimer = null;
    }
    if (unlockRaf) {
      cancelAnimationFrame(unlockRaf);
      unlockRaf = 0;
    }
    unlockProgress = 0;
  }

  async function setLocked(next: boolean) {
    error = "";
    try {
      await applyBabyLock(next);
      babyLocked = next;
      fullscreen = next;
      status = next
        ? "Baby Lock on — hold Esc for 2s to unlock"
        : "Unlocked — parent controls are back";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function toggleFullscreen(next = !fullscreen) {
    error = "";
    try {
      await applyFullscreen(next);
      fullscreen = next;
      status = next ? "Fullscreen" : "Windowed";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="stage" class:locked={babyLocked}>
  <header class="brand" class:dim={babyLocked}>
    <h1>BabySmash</h1>
    <p>{status}</p>
  </header>

  <div class="playfield" aria-label="Smash playfield">
    {#each bursts as burst (burst.id)}
      <div
        class="burst shape-{burst.shape}"
        style="--x:{burst.x}%; --y:{burst.y}%; --c:{burst.color}; --s:{burst.size}px; --r:{burst.rotate}deg;"
      >
        <span>{burst.label}</span>
      </div>
    {/each}
  </div>

  {#if unlockProgress > 0}
    <div class="unlock-bar" aria-hidden="true">
      <div class="unlock-fill" style="transform: scaleX({unlockProgress})"></div>
      <span>Unlocking…</span>
    </div>
  {/if}

  <aside class="controls" data-controls class:hidden={babyLocked}>
    <button type="button" class="primary" onclick={() => setLocked(true)}>
      Baby Lock
    </button>
    <button type="button" onclick={() => toggleFullscreen()}>
      {fullscreen ? "Exit Fullscreen" : "Fullscreen"}
    </button>
    <p class="hint">
      Baby Lock goes fullscreen, stays on top, hides window chrome, and blocks
      webview shortcuts. Hold <kbd>Esc</kbd> for 2 seconds to unlock.
      {#if !inTauri}
        <br />Browser preview: OS shortcut capture needs `npm run tauri:dev`.
      {/if}
    </p>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </aside>
</div>

<style>
  .stage {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }

  .brand {
    position: absolute;
    top: 1.25rem;
    left: 1.5rem;
    z-index: 3;
    pointer-events: none;
    transition: opacity 0.35s ease;
  }

  .brand.dim {
    opacity: 0.35;
  }

  .brand h1 {
    margin: 0;
    font-family: "Fredoka", "Nunito", sans-serif;
    font-size: clamp(2.4rem, 6vw, 4rem);
    font-weight: 700;
    letter-spacing: 0.02em;
    line-height: 1;
    color: var(--ink);
    text-shadow: 0 2px 0 rgba(255, 255, 255, 0.7);
  }

  .brand p {
    margin: 0.45rem 0 0;
    font-size: 1rem;
    font-weight: 700;
    color: rgba(20, 50, 79, 0.72);
  }

  .playfield {
    position: absolute;
    inset: 0;
    z-index: 1;
  }

  .burst {
    position: absolute;
    left: var(--x);
    top: var(--y);
    width: var(--s);
    height: var(--s);
    margin-left: calc(var(--s) / -2);
    margin-top: calc(var(--s) / -2);
    display: grid;
    place-items: center;
    color: white;
    font-family: "Fredoka", "Nunito", sans-serif;
    font-size: calc(var(--s) * 0.48);
    font-weight: 700;
    transform: rotate(var(--r)) scale(0.2);
    animation: pop-in 0.45s cubic-bezier(0.2, 1.4, 0.4, 1) forwards,
      drift-out 1.7s ease-in 0.45s forwards;
    filter: drop-shadow(0 8px 14px rgba(20, 50, 79, 0.22));
    pointer-events: none;
  }

  .burst span {
    line-height: 1;
  }

  .shape-circle {
    background: var(--c);
    border-radius: 50%;
  }

  .shape-square {
    background: var(--c);
    border-radius: 18%;
  }

  .shape-triangle {
    background: transparent;
    color: var(--c);
    filter: drop-shadow(0 8px 10px rgba(20, 50, 79, 0.18));
  }

  .shape-triangle::before {
    content: "";
    position: absolute;
    inset: 8%;
    background: var(--c);
    clip-path: polygon(50% 8%, 92% 88%, 8% 88%);
    border-radius: 8px;
  }

  .shape-triangle span {
    position: relative;
    z-index: 1;
    color: white;
    transform: translateY(12%);
  }

  .shape-star {
    background: var(--c);
    clip-path: polygon(
      50% 4%,
      61% 35%,
      94% 35%,
      67% 57%,
      78% 91%,
      50% 70%,
      22% 91%,
      33% 57%,
      6% 35%,
      39% 35%
    );
  }

  .shape-letter {
    background: var(--c);
    border-radius: 28%;
    box-shadow: inset 0 -6px 0 rgba(0, 0, 0, 0.12);
  }

  @keyframes pop-in {
    to {
      transform: rotate(var(--r)) scale(1);
    }
  }

  @keyframes drift-out {
    to {
      opacity: 0;
      transform: rotate(calc(var(--r) + 18deg)) scale(1.15) translateY(-28px);
    }
  }

  .controls {
    position: absolute;
    right: 1.25rem;
    bottom: 1.25rem;
    z-index: 4;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    width: min(320px, calc(100vw - 2rem));
    padding: 1rem;
    background: var(--panel);
    border: 2px solid rgba(20, 50, 79, 0.08);
    border-radius: 18px;
    box-shadow: var(--shadow);
    transition: opacity 0.3s ease, transform 0.3s ease;
  }

  .controls.hidden {
    opacity: 0;
    pointer-events: none;
    transform: translateY(12px);
  }

  .controls button {
    border: none;
    border-radius: 12px;
    padding: 0.75rem 1rem;
    font-weight: 800;
    color: var(--ink);
    background: #e7f5ff;
    transition: transform 0.15s ease, background 0.15s ease;
  }

  .controls button:hover {
    transform: translateY(-1px);
    background: #d0ebff;
  }

  .controls button.primary {
    color: white;
    background: linear-gradient(180deg, var(--accent) 0%, var(--accent-deep) 100%);
  }

  .controls button.primary:hover {
    background: linear-gradient(180deg, #ff6b88 0%, var(--accent-deep) 100%);
  }

  .hint {
    margin: 0;
    font-size: 0.82rem;
    line-height: 1.35;
    color: rgba(20, 50, 79, 0.72);
  }

  .hint kbd {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 6px;
    background: #fff;
    border: 1px solid rgba(20, 50, 79, 0.15);
    font-family: inherit;
    font-weight: 800;
  }

  .error {
    margin: 0;
    color: var(--accent-deep);
    font-size: 0.85rem;
    font-weight: 700;
  }

  .unlock-bar {
    position: absolute;
    left: 50%;
    bottom: 2rem;
    z-index: 5;
    transform: translateX(-50%);
    width: min(280px, 70vw);
    padding: 0.75rem 1rem 0.9rem;
    border-radius: 999px;
    background: rgba(20, 50, 79, 0.82);
    color: white;
    text-align: center;
    font-weight: 800;
    overflow: hidden;
  }

  .unlock-fill {
    position: absolute;
    inset: 0;
    transform-origin: left center;
    background: linear-gradient(90deg, var(--mint), var(--sky));
    opacity: 0.85;
  }

  .unlock-bar span {
    position: relative;
    z-index: 1;
  }

  @media (max-width: 640px) {
    .brand {
      left: 1rem;
      right: 1rem;
    }

    .controls {
      left: 1rem;
      right: 1rem;
      width: auto;
    }
  }
</style>
