# macOS Baby Lock — manual QA checklist

Use a **signed** `tauri:dev` or `tauri:build` binary on a real Mac. Unsigned builds may not get reliable TCC prompts.

Prereq: System Settings → Privacy & Security → **Accessibility** → enable BabySmash (re-toggle after rebuilds).

| # | Case | Expect |
|---|------|--------|
| 1 | Arm Baby Lock **without** Accessibility | UI fails closed; error + **Open System Settings**; not left locked |
| 2 | Grant Accessibility, relaunch, arm | Lock succeeds; Dock/menu hidden |
| 3 | ⌘Space, ⌘Tab, ⌘Q, ⌘⇧4, Ctrl+↑ | No Spotlight / Mission Control / quit / screenshot UI |
| 4 | Letters / Space / digits | Smash bursts + tones still work |
| 5 | Hold Esc ~2s | Unlocks; chrome returns; shortcuts work again |
| 6 | Mouse move / click while locked | Works (pointer not captured) |
| 7 | Revoke Accessibility while locked | Within a few seconds: unlock + error (input not wedged) |
| 8 | Focus a password field in another app (Secure Input) | Status warns; mouse still usable to unlock via Force Quit if needed |
| 9 | Linux / Windows smoke | Baby Lock still does fullscreen + always-on-top + webview shortcut block only |

See also [macos-keyboard-lock.md](./macos-keyboard-lock.md) and [macos-keyboard-lock-plan.md](./macos-keyboard-lock-plan.md).
