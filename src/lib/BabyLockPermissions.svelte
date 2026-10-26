<script lang="ts">
  import {
    fetchMacosKeyboardLockStatus,
    isTauri,
    lockErrorMessage,
    openAccessibilitySettings,
    type MacosKeyboardLockStatus,
  } from "./babyLock";

  /** Poll only while the containing dialog is visible. */
  let { active }: { active: boolean } = $props();

  const POLL_MS = 1500;

  const inTauri = isTauri();
  let status = $state<MacosKeyboardLockStatus | null>(null);
  let checked = $state(false);
  let error = $state("");

  async function refresh() {
    status = await fetchMacosKeyboardLockStatus();
    checked = true;
  }

  // The parent toggles the switch in System Settings and comes back; poll and
  // re-check on focus so the status flips without reopening Preferences.
  $effect(() => {
    if (!active || !inTauri) return;
    void refresh();
    const timer = setInterval(refresh, POLL_MS);
    window.addEventListener("focus", refresh);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", refresh);
    };
  });

  async function openSettings() {
    error = "";
    try {
      await openAccessibilitySettings();
    } catch (e) {
      error = lockErrorMessage(e);
    }
  }

  const granted = $derived(status?.accessibilityGranted ?? false);
</script>

<section>
  <h3>Baby Lock permissions</h3>

  {#if !inTauri}
    <p class="info">
      Browser preview can only go fullscreen. Run the desktop app to see which permissions Baby
      Lock needs.
    </p>
  {:else if !checked}
    <p class="info">Checking…</p>
  {:else if !status?.supported}
    <p class="info">
      No extra permissions are needed on this system. Baby Lock goes fullscreen, stays on top and
      blocks in-app shortcuts. System-reserved keys (like Ctrl+Alt+Del or the Windows key) still
      work.
    </p>
  {:else}
    <div class="status" class:granted role="status">
      <span class="icon" aria-hidden="true">{granted ? "✅" : "⚠️"}</span>
      <div>
        <p class="headline">
          {granted ? "Accessibility access is on" : "Accessibility access is needed"}
        </p>
        <p class="detail">
          {granted
            ? "Baby Lock can block Cmd-Tab, Cmd-Q, Spotlight and Mission Control."
            : "Without it, Baby Lock still goes fullscreen, but Cmd-Tab, Cmd-Q, Spotlight and Mission Control get through."}
        </p>
      </div>
    </div>

    {#if !granted}
      <ol class="steps">
        <li>
          Click <strong>Open Accessibility Settings</strong>. It opens
          <span class="path">System Settings › Privacy &amp; Security › Accessibility</span>.
        </li>
        <li>
          Find <strong>BabySmash</strong> in the list and turn its switch <strong>on</strong>. macOS
          asks for your password or Touch ID.
        </li>
        <li>Come back here. This section updates by itself.</li>
      </ol>
      <p class="tip">
        <strong>Switched on but still shows as needed?</strong> After an app update, macOS can keep
        a stale entry. Select BabySmash, click <strong>−</strong> to remove it, then click
        <strong>Open Accessibility Settings</strong> again and turn the new entry on.
      </p>
    {/if}

    <button type="button" class="open-settings" onclick={openSettings}>
      {granted ? "Review in System Settings" : "Open Accessibility Settings"}
    </button>
    {#if error}<p class="error">{error}</p>{/if}
  {/if}
</section>

<style>
  p {
    margin: 0;
  }

  .info,
  .tip {
    font-size: 0.85rem;
    line-height: 1.4;
    color: var(--ink-soft);
  }

  .status {
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
    padding: 0.75rem 0.9rem;
    border-radius: 14px;
    background: var(--warn-bg);
  }

  .status.granted {
    background: var(--button-bg);
  }

  .icon {
    font-size: 1.3rem;
    line-height: 1.2;
  }

  .headline {
    font-weight: 800;
  }

  .detail {
    font-size: 0.85rem;
    line-height: 1.35;
    color: var(--ink-soft);
  }

  .steps {
    margin: 0.75rem 0 0.6rem;
    padding-left: 1.3rem;
    font-size: 0.9rem;
    line-height: 1.45;
  }

  .steps li + li {
    margin-top: 0.3rem;
  }

  .path {
    font-weight: 800;
  }

  .open-settings {
    margin-top: 0.75rem;
    padding: 0.7rem 1.1rem;
    border: none;
    border-radius: 12px;
    color: white;
    font-weight: 800;
    background: linear-gradient(180deg, var(--accent) 0%, var(--accent-deep) 100%);
  }

  .open-settings:hover {
    background: linear-gradient(180deg, var(--accent-hover) 0%, var(--accent-deep) 100%);
  }

  .status.granted + .open-settings {
    color: var(--ink);
    background: var(--button-bg);
  }

  .status.granted + .open-settings:hover {
    background: var(--button-bg-hover);
  }

  .error {
    margin-top: 0.5rem;
    color: var(--danger);
    font-size: 0.85rem;
    font-weight: 700;
  }
</style>
