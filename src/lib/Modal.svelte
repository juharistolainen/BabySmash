<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open,
    title,
    onclose,
    children,
    footer,
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  const titleId = `modal-title-${Math.random().toString(36).slice(2)}`;
  let dialog: HTMLDialogElement;

  // Native <dialog> gives us focus trapping, Esc-to-close and an inert backdrop.
  $effect(() => {
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialog) onclose();
  }
</script>

<!-- data-controls: the page's pointer handler won't spawn bursts for clicks in here. -->
<dialog
  bind:this={dialog}
  class="modal"
  data-controls
  aria-labelledby={titleId}
  {onclose}
  onclick={onBackdropClick}
>
  <div class="body">
    <header>
      <h2 id={titleId}>{title}</h2>
      <button type="button" class="close" aria-label="Close" onclick={onclose}>✕</button>
    </header>

    {@render children()}

    <footer>
      {@render footer?.()}
      <button type="button" class="done" onclick={onclose}>Done</button>
    </footer>
  </div>
</dialog>

<style>
  .modal {
    width: min(520px, calc(100vw - 2rem));
    max-height: calc(100vh - 2rem);
    overflow-y: auto;
    padding: 0;
    border: 2px solid var(--panel-border);
    border-radius: 22px;
    background: var(--panel-solid);
    color: var(--ink);
    box-shadow: var(--shadow);
  }

  .modal::backdrop {
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

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
  }

  /* Scoped: a :global(button) reset here would outrank children's own button rules. */
  button {
    border: none;
    color: var(--ink);
    font-weight: 800;
  }

  /* Section headings in dialog content share one style. */
  .modal :global(h3) {
    margin: 0 0 0.5rem;
    font-size: 0.8rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-soft);
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

  .modal .done {
    padding: 0.7rem 1.4rem;
    border-radius: 12px;
    color: white;
    background: linear-gradient(180deg, var(--accent) 0%, var(--accent-deep) 100%);
  }

  .modal .done:hover {
    background: linear-gradient(180deg, var(--accent-hover) 0%, var(--accent-deep) 100%);
  }
</style>
