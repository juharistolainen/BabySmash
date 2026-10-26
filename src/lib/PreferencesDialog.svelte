<script lang="ts">
  import Modal from "./Modal.svelte";
  import BabyLockPermissions from "./BabyLockPermissions.svelte";
  import { THEMES, type ThemeId } from "./themes";
  import { SOUNDSCAPES } from "./soundscapes";
  import { playSmash } from "./audio";
  import type { ColorMode, Preferences, ResolvedMode } from "./preferences";

  let {
    open,
    preferences,
    resolvedMode,
    onchange,
    onclose,
  }: {
    open: boolean;
    preferences: Preferences;
    resolvedMode: ResolvedMode;
    onchange: (next: Preferences) => void;
    onclose: () => void;
  } = $props();

  const MODES: { id: ColorMode; label: string; icon: string }[] = [
    { id: "system", label: "System", icon: "🖥️" },
    { id: "light", label: "Light", icon: "☀️" },
    { id: "dark", label: "Dark", icon: "🌙" },
  ];

  function setMode(mode: ColorMode) {
    onchange({ ...preferences, mode });
  }

  function setTheme(theme: ThemeId) {
    onchange({ ...preferences, theme });
    previewSound(theme);
  }

  function setAmbience(ambience: boolean) {
    onchange({ ...preferences, ambience });
  }

  function setVolume(event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value) / 100;
    onchange({ ...preferences, volume: value });
  }

  // Lets the parent hear a theme (or a new volume) without leaving the dialog.
  function previewSound(theme: ThemeId = preferences.theme) {
    playSmash(Math.floor(Math.random() * 1000), SOUNDSCAPES[theme].voice);
  }
</script>

<Modal {open} title="Preferences" {onclose}>
  <section>
    <h3 id="mode-label">Appearance</h3>
    <div class="segmented" role="radiogroup" aria-labelledby="mode-label">
      {#each MODES as mode (mode.id)}
        <button
          type="button"
          role="radio"
          aria-checked={preferences.mode === mode.id}
          class:active={preferences.mode === mode.id}
          onclick={() => setMode(mode.id)}
        >
          <span aria-hidden="true">{mode.icon}</span>
          {mode.label}
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3 id="theme-label">Theme</h3>
    <div class="themes" role="radiogroup" aria-labelledby="theme-label">
      {#each THEMES as theme (theme.id)}
        <button
          type="button"
          role="radio"
          aria-checked={preferences.theme === theme.id}
          class="theme-card"
          class:active={preferences.theme === theme.id}
          data-theme={theme.id}
          data-mode={resolvedMode}
          onclick={() => setTheme(theme.id)}
        >
          <span class="preview" aria-hidden="true">{theme.preview}</span>
          <span class="name">{theme.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3 id="ambience-label">Sound</h3>
    <div class="segmented two" role="radiogroup" aria-labelledby="ambience-label">
      {#each [false, true] as on (on)}
        <button
          type="button"
          role="radio"
          aria-checked={preferences.ambience === on}
          class:active={preferences.ambience === on}
          onclick={() => setAmbience(on)}
        >
          <span aria-hidden="true">{on ? "🎶" : "🔕"}</span>
          {on ? "Background ambience" : "Smash sounds only"}
        </button>
      {/each}
    </div>
    <label class="volume">
      <span aria-hidden="true">🔈</span>
      <span class="visually-hidden">Volume</span>
      <input
        type="range"
        min="0"
        max="100"
        step="5"
        value={Math.round(preferences.volume * 100)}
        oninput={setVolume}
        onchange={() => previewSound()}
      />
      <span aria-hidden="true">🔊</span>
    </label>
  </section>

  <BabyLockPermissions active={open} />
</Modal>

<style>
  button {
    border: none;
    color: var(--ink);
    font-weight: 800;
  }

  .segmented {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.3rem;
    padding: 0.3rem;
    border-radius: 14px;
    background: var(--button-bg);
  }

  .segmented button {
    padding: 0.6rem 0.5rem;
    border-radius: 10px;
    background: transparent;
    transition: background 0.15s ease;
  }

  .segmented button.active {
    background: var(--panel-solid);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
  }

  .segmented.two {
    grid-template-columns: repeat(2, 1fr);
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-top: 0.7rem;
    padding: 0 0.3rem;
  }

  .volume input {
    flex: 1;
    accent-color: var(--accent);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.6rem;
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    padding: 0.9rem 0.5rem 0.7rem;
    border-radius: 16px;
    border: 3px solid transparent;
    background: var(--bg);
    background-size: cover;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .theme-card:hover {
    transform: translateY(-2px);
  }

  .theme-card.active {
    border-color: var(--accent);
  }

  .preview {
    font-size: 1.6rem;
    letter-spacing: 0.08em;
    font-family: "Fredoka", "Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif;
  }

  .name {
    padding: 0.1rem 0.6rem;
    border-radius: 999px;
    background: var(--panel);
    font-size: 0.85rem;
  }
</style>
