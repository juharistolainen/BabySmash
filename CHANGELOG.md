# Changelog

## 1.0.1 — 2026-10-27

Patch release since 1.0.0.

- Per-theme soundscapes: smash voices plus optional ambient beds (opt-in ambience preference)
- Baby Lock permissions UI / Preferences polish for Accessibility
- Minimise confirmation for the on-screen controls panel (reopen via small button)

## 1.0.0 — 2026-10-16

First public release of BabySmash.

- SvelteKit 5 + Tauri 2 desktop app: smash keys / poke the screen for shapes and tones
- Themes, About dialog, Preferences
- Baby Lock kiosk mode (macOS): fullscreen, always-on-top, webview shortcut blocking, Accessibility-backed keyboard capture via `CGEventTap`
- Unlock by holding Esc for 2 seconds; fails open if Accessibility is revoked while locked
- CI builds for macOS (aarch64 + x86_64), Linux (deb/AppImage), and Windows (NSIS/MSI)
