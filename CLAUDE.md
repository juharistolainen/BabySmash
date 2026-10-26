# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

BabySmash: a Tauri 2 desktop app with a SvelteKit 5 + TypeScript frontend. When a baby presses keys or taps the screen, the app draws shapes and plays tones. The interesting part is **Baby Lock**, a kiosk mode that tries to stop the baby from leaving the app.

## Commands

```bash
npm install
npm run tauri:dev     # full desktop app (Vite on fixed port 1420, strictPort)
npm run dev           # browser-only UI; no always-on-top, shortcut blocking, or macOS capture
npm run tauri:build   # production bundle
npm run check         # svelte-kit sync + svelte-check (the only frontend type/lint gate)

# Rust side (run from src-tauri/)
cargo check
cargo clippy
```

There is no test suite in either the JS or the Rust code. You can only verify macOS Baby Lock by hand, using `docs/macos-keyboard-lock-qa.md`. That check needs a signed build with Accessibility permission granted, and you may have to re-toggle the permission after each rebuild. Requirements: Node 20+ and Rust 1.90+.

## Architecture

**Frontend (`src/`)**: SvelteKit runs as an SPA. `adapter-static` serves `index.html` as the fallback, and `+layout.ts` sets `ssr = false`. Almost all UI state and key handling is in `src/routes/+page.svelte`, which uses Svelte 5 runes (`$state`). Key listeners are registered in the capture phase. `src/lib/` holds:
- `smash.ts`: generates bursts (shape, color, position)
- `audio.ts`: a Web Audio synth with no asset files. Everything routes through a master gain (the volume preference) and a compressor. `playSmash` caps concurrent oscillators so key mashing can't clip. The `AudioContext` is resumed on the first user gesture, and the ambient bed (`setAmbient`) only starts after that; it pauses while the document is hidden.
- `soundscapes.ts`: per-theme sound as data, `Record<ThemeId, Soundscape>` (a smash `VoiceSpec` plus an optional `AmbientSpec` bed). Adding a theme without a soundscape fails `npm run check`.
- `babyLock.ts`: the only bridge to Rust. Each call checks `isTauri()` (looks for `__TAURI_INTERNALS__`) and falls back to the browser Fullscreen API when Tauri is absent.

**Backend (`src-tauri/src/`)**:
- `lib.rs`: Tauri commands `set_baby_lock`, `set_fullscreen`, `macos_keyboard_lock_status`, `open_accessibility_settings`, plus the `tauri-plugin-prevent-default` setup. Debug builds keep DevTools and reload available; release builds block every webview shortcut.
- `macos_keyboard_lock.rs`: a `Mutex<KeyboardLockState>` held in Tauri managed state. All platform code sits in a `#[cfg(target_os = "macos")] mod platform` that calls CoreGraphics/CoreFoundation over hand-written FFI. On other OSes the public functions do nothing and report `supported: false`.

**Adding a command** takes three edits: write the `#[tauri::command]` function and register it in `generate_handler!` in `lib.rs`, add a `[[permission]]` and include it in the `babysmash-commands` set in `src-tauri/permissions/babysmash.toml`, and add an `invoke` wrapper in `babyLock.ts`. If you skip the permission, the call fails at runtime.

### Baby Lock invariants

- **Fail closed when locking.** `set_baby_lock(true)` arms macOS capture *before* it touches the window, and rolls the capture back if the window operations fail. Unlocking always tears down capture, even when the window operations error.
- **Main-thread requirement.** Changes to AppKit `presentationOptions` must happen on the main thread. `run_lock_on_main` sends the work through `app.run_on_main_thread` and waits on an mpsc channel for the result. Kiosk options are re-applied whenever the window regains focus (`reassert_if_armed`).
- **macOS has two layers:** (1) kiosk `presentationOptions`, which need no permission, and (2) an active `CGEventTap` running on its own thread with a CFRunLoop. The tap drops Cmd/Ctrl/Opt chords and function keys, and lets plain keys and bare Esc through. If you change `should_drop_key`, Esc without modifiers must still reach the webview, or the parent can no longer unlock. The tap callback re-enables the tap when macOS disables it after a timeout.
- **Accessibility errors.** When Accessibility is missing, the backend returns the string code `needs_accessibility`. The UI matches it with `isNeedsAccessibility()` and offers to open System Settings.
- **Revoke path.** While locked, the UI polls `macos_keyboard_lock_status` every 3s. If Accessibility has been revoked, the UI unlocks (fails open) so the keyboard doesn't get stuck. The status check uses the cheap IOHID check, not a probe tap.
- **Unlock** means holding Esc for 2s (`UNLOCK_HOLD_MS` in `+page.svelte`). While locked, the UI swallows other key events.
- **Lock intent survives remounts.** It is stored in `sessionStorage`, because fullscreen changes can remount the webview. Window locking sets always-on-top *before* fullscreen (this order matters on Linux) and never toggles decorations, because toggling them remounts the webview.
- **No native fullscreen on macOS while locked.** AppKit aborts the `toggleFullScreen:` Spaces transition when kiosk options (`DisableProcessSwitching`, `HideDock`) are set, so the window bounces back. Instead, `arm()` makes the `NSWindow` borderless and sizes it to the screen, and `disarm()` restores the saved frame and style mask. It restores the first responder after `setStyleMask`, because otherwise key events stop reaching the webview.
- Windows and Linux get only fullscreen, always-on-top, and webview-shortcut blocking. OS-reserved shortcuts still work there.

### macOS distribution constraint

Ship the app as a notarized Developer ID build **without App Sandbox**. Active event taps are unreliable inside the sandbox. Background and design reasoning are in `docs/macos-keyboard-lock.md` and `docs/macos-keyboard-lock-plan.md`.
