<script lang="ts">
  import Modal from "./Modal.svelte";
  import { THEMES, type ThemeId } from "./themes";
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
