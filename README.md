# BabySmash

Cross-platform desktop play app for babies: every keyboard smash (and screen poke) bursts colorful shapes and playful tones. Built with **Tauri 2**, **SvelteKit**, and **TypeScript**.

## Features (first slice)

- **Smash feedback** — keys and pointer taps spawn animated shapes/letters with Web Audio tones
- **Fullscreen** — one click (or Baby Lock) fills the display
- **Baby Lock** — exclusive play mode that:
  - enters fullscreen
  - keeps the window always on top
  - disables webview/browser shortcuts (reload, find, print, DevTools chords, etc.) via `tauri-plugin-prevent-default`
  - on **macOS**, applies AppKit kiosk presentation options and an active `CGEventTap` to block OS shortcuts (Spotlight, Mission Control, Cmd+Q/Tab, screenshots, …) — requires **Accessibility** permission
  - swallows key events in the UI while locked
  - hides parent controls (unlock by holding `Esc`)
- **Parent unlock** — hold `Esc` for **2 seconds**

## Requirements

- Node.js 20+ and npm
- Rust **1.90+** (stable) — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- OS-specific Tauri deps (WebView2 on Windows, WebKitGTK on Linux, Xcode CLT on macOS)

## Run locally

```bash
npm install

# UI only (Vite) — useful for layout/sound iteration in a browser
npm run dev

# Full desktop app (recommended)
npm run tauri:dev
```

Vite defaults to Tauri’s `http://localhost:1420` when launched through `tauri:dev`.

Production build:

```bash
npm run tauri:build
```

Typecheck the frontend:

```bash
npm run check
```

## Trying Baby Lock + fullscreen

1. Start with `npm run tauri:dev`
2. Smash keys / poke the window — shapes and sounds should appear immediately
3. Click **Fullscreen** to toggle display mode without locking
4. Click **Baby Lock** — UI controls hide; window goes fullscreen + always-on-top
5. Hold **Esc** for ~2 seconds to unlock (progress bar appears)

Browser-only `npm run dev` can preview smash graphics/sound and the Fullscreen API, but Baby Lock’s always-on-top / webview-shortcut blocking need the Tauri shell.

## Platform caveats (shortcut capture)

Baby Lock **does** block in-app / webview shortcuts (for example `Ctrl/Cmd+R`, `Ctrl/Cmd+W`, `Ctrl/Cmd+F`, `F5`, `F12`, context menu) so accidental baby chords don’t reload or close the webview UI.

### macOS (implemented)

When Baby Lock arms on macOS it:

1. Sets AppKit kiosk `presentationOptions` (hides Dock/menu bar; disables Cmd+Tab / Exposé, Force Quit panel, session-termination UI, Apple menu, Hide).
2. Installs an **active** `CGEventTap` that drops Cmd/Ctrl/Opt chords and function keys before system handlers see them, while still passing plain smash keys and Esc for unlock.

**You must grant Accessibility** (System Settings → Privacy & Security → Accessibility) for the tap. Without it, Baby Lock fails closed and the UI offers **Open System Settings**. After rebuilding/re-signing, you may need to toggle the permission again.

**Distribution:** ship as a notarized Developer ID app **without App Sandbox**. Active filter taps are not reliable in the sandbox / Mac App Store.

**Still not blockable:** hardware power-button hold; Secure Event Input (password fields in other apps — the tap goes deaf; status warns when detected). Mouse stays usable for recovery.

See `docs/macos-keyboard-lock.md` and `docs/macos-keyboard-lock-plan.md`.

### Windows / Linux

OS-reserved shortcuts are **not** fully suppressed yet:

| Platform | Typically still available to the OS |
| -------- | ----------------------------------- |
| Windows  | `Alt+Tab`, `Ctrl+Alt+Del`, `Win` key, some `Alt+F4` paths |
| Linux    | Desktop environment shortcuts (`Alt+Tab`, system overlays); Wayland especially limits global grabs |

## Project layout

- `src/` — SvelteKit UI (smash canvas, audio, lock controls)
- `src-tauri/` — Tauri/Rust shell (`set_baby_lock`, `set_fullscreen`, `macos_keyboard_lock`, prevent-default plugin)
- `docs/` — macOS keyboard-lock research and implementation plan

## License

MIT
