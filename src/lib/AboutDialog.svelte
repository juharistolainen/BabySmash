<script lang="ts">
  import Modal from "./Modal.svelte";
  import { fetchBuildInfo, type NativeBuildInfo } from "./babyLock";
  import { FRONTEND_VERSIONS } from "./buildInfo";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();

  let native = $state<NativeBuildInfo | null>(null);
  let loaded = $state(false);
  let copied = $state(false);

  // Fetch once, on first open.
  $effect(() => {
    if (open && !loaded) {
      loaded = true;
      void fetchBuildInfo().then((info) => (native = info));
    }
  });

  const WEBVIEW_NAMES: Record<string, string> = {
    macos: "WebKit",
    windows: "WebView2",
    linux: "WebKitGTK",
  };

  const OS_NAMES: Record<string, string> = { macos: "macOS", windows: "Windows", linux: "Linux" };

  type Row = [label: string, value: string];

  const sections = $derived.by((): { title: string; rows: Row[] }[] => {
    const fe = FRONTEND_VERSIONS;
    if (!native) {
      return [
        {
          title: "Frontend",
          rows: [
            ["Svelte", fe.svelte],
            ["SvelteKit", fe.kit],
            ["Vite", fe.vite],
          ],
        },
      ];
    }
    const built = new Date(native.buildTime * 1000).toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    });
    return [
      {
        title: "Build",
        rows: [
          ["Built", built],
          ["Profile", native.profile],
          ["Commit", native.commit ?? "unknown"],
          ["Target", native.target],
        ],
      },
      {
        title: "Frameworks",
        rows: [
          ["Tauri", native.tauri],
          ["Rust", native.rustc],
          ["Svelte", fe.svelte],
          ["SvelteKit", fe.kit],
          ["Vite", fe.vite],
          ["@tauri-apps/api", fe.tauriApi],
        ],
      },
      {
        title: "Running on",
        rows: [
          ["System", `${OS_NAMES[native.os] ?? native.os} (${native.arch})`],
          [WEBVIEW_NAMES[native.os] ?? "WebView", native.webview ?? "unknown"],
        ],
      },
    ];
  });

  const name = $derived(native?.name ?? "BabySmash");
  const version = $derived(native?.version ?? FRONTEND_VERSIONS.app);

  /** Plain-text dump for bug reports. */
  async function copyDetails() {
    const lines = [`${name} ${version}`];
    for (const section of sections) {
      lines.push("", `${section.title}:`);
      for (const [label, value] of section.rows) lines.push(`  ${label}: ${value}`);
    }
    try {
      await navigator.clipboard.writeText(lines.join("\n"));
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      // clipboard can be unavailable (permissions / insecure context); ignore
    }
  }
</script>

<Modal {open} title="About" {onclose}>
  <div class="hero">
    <img src="/favicon.png" alt="" width="72" height="72" />
    <div>
      <p class="name">{name}</p>
      <p class="version">Version {version}</p>
      <p class="tagline">Keyboard smash play for babies</p>
    </div>
  </div>

  {#each sections as section (section.title)}
    <section>
      <h3>{section.title}</h3>
      <dl>
        {#each section.rows as [label, value] (label)}
          <dt>{label}</dt>
          <dd>{value}</dd>
        {/each}
      </dl>
    </section>
  {/each}

  {#if !native}
    <p class="note">Build and runtime details are only shown in the desktop app.</p>
  {/if}

  {#snippet footer()}
    <button type="button" class="copy" onclick={copyDetails}>
      {copied ? "Copied!" : "Copy details"}
    </button>
  {/snippet}
</Modal>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .hero img {
    border-radius: 18px;
    box-shadow: var(--shadow);
  }

  .hero p {
    margin: 0;
  }

  .name {
    font-family: "Fredoka", "Nunito", sans-serif;
    font-size: 1.8rem;
    font-weight: 700;
    line-height: 1.1;
  }

  .version {
    font-weight: 800;
    color: var(--accent);
  }

  .tagline,
  .note {
    font-size: 0.85rem;
    color: var(--ink-soft);
  }

  .note {
    margin: 0;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.3rem 1rem;
    margin: 0;
    padding: 0.7rem 0.9rem;
    border-radius: 14px;
    background: var(--button-bg);
    font-size: 0.9rem;
  }

  dt {
    color: var(--ink-soft);
    font-weight: 700;
  }

  dd {
    margin: 0;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
    /* Let parents select version strings for bug reports, unlike the rest of the app. */
    user-select: text;
    -webkit-user-select: text;
  }

  .copy {
    padding: 0.7rem 1.2rem;
    border: none;
    border-radius: 12px;
    color: var(--ink);
    font-weight: 800;
    background: var(--button-bg);
  }

  .copy:hover {
    background: var(--button-bg-hover);
  }
</style>
