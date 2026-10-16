<script lang="ts">
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

  let dialog: HTMLDialogElement;

  // Native <dialog> gives us focus trapping, Esc-to-close and an inert backdrop.
  $effect(() => {
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  function setMode(mode: ColorMode) {
    onchange({ ...preferences, mode });
  }

  function setTheme(theme: ThemeId) {
    onchange({ ...preferences, theme });
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialog) onclose();
  }
</script>

<dialog
  bind:this={dialog}
  class="prefs"
  data-controls
  aria-labelledby="prefs-title"
  onclose={onclose}
  onclick={onBackdropClick}
>
  <div class="body">
    <header>
      <h2 id="prefs-title">Preferences</h2>
      <button type="button" class="close" aria-label="Close" onclick={onclose}>✕</button>
    </header>

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

    <footer>
      <button type="button" class="done" onclick={onclose}>Done</button>
    </footer>
  </div>
</dialog>

<style>
  .prefs {
    width: min(520px, calc(100vw - 2rem));
    max-height: calc(100vh - 2rem);
    padding: 0;
    border: 2px solid var(--panel-border);
    border-radius: 22px;
    background: var(--panel-solid);
    color: var(--ink);
    box-shadow: var(--shadow);
  }

  .prefs::backdrop {
    background: rgba(10, 20, 35, 0.4);
    backdrop-filter: blur(3px);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
    padding: 1.25rem 1.4rem 1.4rem;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font-family: "Fredoka", "Nunito", sans-serif;
    font-size: 1.6rem;
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.8rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  button {
    border: none;
    color: var(--ink);
    font-weight: 800;
  }

  .close {
    width: 2.2rem;
    height: 2.2rem;
    border-radius: 999px;
    background: var(--button-bg);
  }

  .close:hover {
    background: var(--button-bg-hover);
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

  footer {
    display: flex;
    justify-content: flex-end;
  }

  .done {
    padding: 0.7rem 1.4rem;
    border-radius: 12px;
    color: white;
    background: linear-gradient(180deg, var(--accent) 0%, var(--accent-deep) 100%);
  }
</style>
