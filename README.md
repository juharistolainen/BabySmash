# BabySmash

Cross-platform desktop play app for babies: every keyboard smash (and screen poke) bursts colorful shapes and playful tones. Built with **Tauri 2**, **SvelteKit**, and **TypeScript**.

## Features (first slice)

- **Smash feedback** — keys and pointer taps spawn animated shapes/letters with Web Audio tones
- **Fullscreen** — one click (or Baby Lock) fills the display
- **Baby Lock** — exclusive play mode that:
  - enters fullscreen
  - keeps the window always on top
  - hides window decorations/chrome
  - disables webview/browser shortcuts (reload, find, print, DevTools chords, etc.) via `tauri-plugin-prevent-default`
  - swallows key events in the UI while locked
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
4. Click **Baby Lock** — UI controls hide, window goes fullscreen + always-on-top, decorations drop
5. Hold **Esc** for ~2 seconds to unlock (progress bar appears)

Browser-only `npm run dev` can preview smash graphics/sound and the Fullscreen API, but Baby Lock’s always-on-top / decoration / webview-shortcut blocking need the Tauri shell.

## Platform caveats (shortcut capture)

Baby Lock **does** block in-app / webview shortcuts (for example `Ctrl/Cmd+R`, `Ctrl/Cmd+W`, `Ctrl/Cmd+F`, `F5`, `F12`, context menu) so accidental baby chords don’t reload or close the webview UI.

It **cannot fully suppress OS-reserved shortcuts**. Those are enforced by the operating system and often require accessibility / kiosk / MDM privileges beyond a normal desktop app:

| Platform | Typically still available to the OS |
| -------- | ----------------------------------- |
| macOS    | `Cmd+Tab`, `Cmd+Q`, `Cmd+Space`, Mission Control, Force Quit |
| Windows  | `Alt+Tab`, `Ctrl+Alt+Del`, `Win` key, some `Alt+F4` paths |
| Linux    | Desktop environment shortcuts (`Alt+Tab`, system overlays); Wayland especially limits global grabs |

Practical tip: enable OS guided-access / kiosk / “focus” modes when you need stronger lockdown, and keep Baby Lock on for the app-level protection.

## Project layout

- `src/` — SvelteKit UI (smash canvas, audio, lock controls)
- `src-tauri/` — Tauri/Rust shell (`set_baby_lock`, `set_fullscreen`, prevent-default plugin)

## License

MIT
