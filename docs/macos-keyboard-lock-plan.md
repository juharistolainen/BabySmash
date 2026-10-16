# Implementation plan: macOS Baby Lock keyboard capture

Based on [macos-keyboard-lock.md](./macos-keyboard-lock.md). Goal: when Baby Lock is on, babies cannot fire Spotlight, Mission Control, Cmd+Q/Tab, screenshots, etc., while smash feedback and parent unlock (hold Esc 2s) still work.

**Out of scope for this slice:** Windows/Linux OS capture, Mac App Store sandbox build, MDM.

**Prerequisite product decision (lock this before coding):** ship macOS as **Developer ID + notarized, App Sandbox off**. Full key swallowing is not reliable in the sandbox; do not plan App Store for this feature yet.

---

## Success criteria

While Baby Lock is armed on a signed macOS build with Accessibility granted:

1. These do **not** activate: Spotlight (⌘Space), Mission Control / Spaces chords, ⌘Tab, ⌘Q, ⌘H, ⌘⇧3/4/5, Force Quit panel (⌘⌥⎋).
2. Smash keys still produce visual + audio feedback.
3. Hold Esc 2s still unlocks (existing UX).
4. Mouse remains usable (no pointer tap).
5. If Accessibility is missing, UI does **not** claim full lock; shows grant steps + “Open System Settings.”
6. Disarm restores normal presentation options and removes the tap.

Known remaining escapes (document in UI/README, do not “fix”): power-button hold, Secure Event Input (tap goes deaf).

---

## Current hooks (do not reinvent)

| Layer | File | Role today |
|-------|------|------------|
| UI lock toggle / Esc hold | `src/routes/+page.svelte` | Smash + unlock UX |
| Invoke wrapper | `src/lib/babyLock.ts` | `invoke("set_baby_lock")` |
| Native shell | `src-tauri/src/lib.rs` | always-on-top + fullscreen only |
| Webview chords | `tauri-plugin-prevent-default` | Keep; still useful |

All new macOS capture goes behind `set_baby_lock` / small sibling commands. Keep non-macOS behavior unchanged (`#[cfg(target_os = "macos")]`).

---

## Architecture

```
Baby Lock ON
  ├─ (existing) window always-on-top + fullscreen
  ├─ Layer 1: NSApp.presentationOptions (kiosk)     ← no TCC
  └─ Layer 2: active CGEventTap (filter/swallow)  ← Accessibility
        policy: drop system chords; pass smash/Esc
        OR: drop-all + emit events to webview
```

**Recommended event policy for v1 (simpler, keeps current Svelte handlers):**

| Event | Tap action |
|-------|------------|
| Esc (no other modifiers) | **Pass** (frontend unlock hold) |
| Key with ⌘ / ⌃ / ⌥ (any) | **Drop** |
| Function / media / sys-defined (F-keys, brightness, etc.) | **Drop** |
| Plain letters, digits, Space, arrows, etc. | **Pass** (smash in webview) |
| Mouse / scroll | **Ignore** (do not tap) |

If QA finds plain-key system bindings still firing (rare custom shortcuts), escalate to **drop-all + Tauri event bridge** (`baby-lock-key`) and move Esc-hold timing into Rust or listen for the event in Svelte. Do not start with the bridge unless Pass policy fails.

---

## Phases

### Phase 0 — Scaffolding & packaging (½ day engineering)

1. Confirm `src-tauri` has **no** App Sandbox entitlement for macOS (add explicit note in README + `tauri.conf` / entitlements file if introduced later).
2. Add macOS-only deps (pick one path and stick to it):
   - **Preferred:** `cgevents` for the tap; `objc2` + `objc2-app-kit` for `NSApplication.presentationOptions`.
   - **Fallback:** raw `core-graphics` / CoreFoundation FFI with callback returning `std::ptr::null_mut()` (verify Spotlight is actually blocked — `Option::None` wrappers have lied before).
3. New module: `src-tauri/src/macos_keyboard_lock.rs` (+ `mod` from `lib.rs`, cfg-gated).
4. Managed state: `Mutex<KeyboardLockState>` on the Tauri app (tap handle, previous presentation options, armed flag).
5. Extend invoke surface (serde-friendly):
   - `set_baby_lock` — orchestrates window + macOS layers; returns structured error.
   - `macos_keyboard_lock_status` — `{ armed, accessibilityGranted, secureInput }` for UI.
   - `open_accessibility_settings` — open Privacy → Accessibility pane.

**Exit:** compiles on macOS; Linux/Windows still build with stubs.

---

### Phase 1 — Presentation options only (shippable partial win)

No Accessibility prompt. Wire into `set_baby_lock(true/false)`:

**On arm:** set valid combo, e.g.

- `hideDock | hideMenuBar`
- `disableProcessSwitching | disableForceQuit | disableSessionTermination`
- `disableAppleMenu | disableHideApplication`

(Use try/catch / Result — invalid combos throw.)

**On disarm / app exit:** restore previous options (store before arm).

**Also:** re-apply options on become-active if still locked (focus theft).

**QA (Phase 1):** ⌘Tab / Exposé disabled; Force Quit panel disabled; Dock/menu hidden. Expect Spotlight ⌘Space and ⌘Q **still** to work — that is Phase 2.

**Exit:** mergeable improvement even if tap work slips.

---

### Phase 2 — Active CGEventTap

1. **Permission gate before claiming lock**
   - Probe: try create active tap (or `IOHIDCheckAccess(PostEvent)` / `AXIsProcessTrusted` — document which you trust; create-tap is authoritative for filtering).
   - If denied: call request prompt once; return error `needs_accessibility` to frontend; **do not** set `babyLocked` true in UI (frontend already rolls back on throw — keep that).
   - Offer button → `open_accessibility_settings`.
2. **Install tap** after presentation options:
   - Location: HID head-insert; fallback session head-insert.
   - Options: **default/filter**, not listen-only.
   - Mask: keyDown | keyUp | flagsChanged (+ sys-defined if media keys leak).
   - Run loop: dedicated thread with its own CFRunLoop **or** main run loop — match `cgevents` guidance; never leave a dead tap installed.
3. **Callback policy:** as in Architecture table; re-enable on `tapDisabledByTimeout` / `ByUserInput`.
4. **Safety**
   - On Accessibility revoke while armed: disarm tap immediately, emit `baby-lock-degraded`, unlock or keep presentation-only mode (prefer unlock + error — avoid wedged input).
   - Poll `IsSecureEventInputEnabled` every few seconds while armed; surface status string if true.
5. **Disarm:** invalidate tap, join thread, restore presentation options, then existing fullscreen/always-on-top off.

**Exit:** Phase 2 QA matrix green on a real Mac with Accessibility on.

---

### Phase 3 — Frontend / UX polish

1. `babyLock.ts`: type the invoke error (`needs_accessibility` vs generic).
2. `+page.svelte` when arm fails:
   - Short copy: why Accessibility is needed.
   - Button: Open System Settings.
   - Do not leave sessionStorage lock stuck on (already rolled back on catch — verify).
3. While locked + secure input: status line warning (“Another app is blocking keyboard lock”).
4. README: macOS setup — grant Accessibility, notarized build, known limits (power button, Secure Input), no App Store claim.
5. Optional: first-run tip the first time user enables Baby Lock on macOS.

---

### Phase 4 — Hardening (after core works)

1. If Pass policy leaks a system shortcut → switch that key class to Drop, or move to drop-all + event bridge.
2. Codesign identity note: after rebuild, user may need to re-toggle Accessibility — mention in README.
3. Manual QA checklist as CI-adjacent doc or PR description (no Mac in cloud CI assumed).
4. Consider `LSUIElement` only if product wants menubar-less helper — **not** required for BabySmash; skip unless needed.

---

## File-level change list

| Path | Change |
|------|--------|
| `src-tauri/Cargo.toml` | macOS deps (`cgevents` / `objc2*`) via target-specific deps |
| `src-tauri/src/macos_keyboard_lock.rs` | **new** — arm/disarm, permissions, tap, presentation |
| `src-tauri/src/lib.rs` | call macOS module from `set_baby_lock`; manage state; new commands |
| `src-tauri/capabilities/default.json` | allow new commands if capability allowlist requires it |
| `src/lib/babyLock.ts` | status helpers, open-settings invoke, typed errors |
| `src/routes/+page.svelte` | permission / secure-input UI |
| `README.md` | macOS Baby Lock permissions + limits |

No change to smash audio/visual modules unless adopting event bridge.

---

## Testing plan (manual, physical Mac)

Use a **signed** debug/release build (unsigned TCC behavior differs).

| # | Case | Expect |
|---|------|--------|
| 1 | Arm without Accessibility | Prompt / settings CTA; UI not locked |
| 2 | Grant Accessibility, relaunch, arm | Tap installs; status OK |
| 3 | ⌘Space, ⌘Tab, ⌘Q, ⌘⇧4, Ctrl+↑ | No system UI |
| 4 | Letters / Space smash | Bursts + tones |
| 5 | Hold Esc 2s | Unlock; chrome back; tap gone |
| 6 | Mouse move/click | Works |
| 7 | Revoke Accessibility while locked | Degrade safely; input not wedged |
| 8 | Secure Input (password field elsewhere) | Warning; unlock still possible via mouse if needed |
| 9 | Linux/Windows smoke | Unchanged Baby Lock (webview + fullscreen only) |

---

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Tap Drop breaks smash/Esc | v1 Pass non-chord keys; QA before widening Drop |
| `cgevents` Drop ≠ C NULL | Spike Spotlight block on day 1; fall back to raw FFI |
| Active tap + permission revoke hangs input | Watch permission; disarm immediately |
| Webview remount loses UI state | Keep existing sessionStorage restore |
| Parents confuse Input Monitoring vs Accessibility | Copy must say **Accessibility** for Baby Lock |
| Scope creep to App Store | Explicit non-goal this slice |

---

## Suggested work order (execution)

1. Phase 0 scaffold + dependency spike (tap creates, Drop blocks Spotlight) — **fail fast here**.
2. Phase 1 presentation options behind `set_baby_lock`.
3. Phase 2 tap + permission UX commands.
4. Phase 3 frontend copy + README.
5. Phase 4 only if QA finds leaks.

Do not implement Windows/Linux OS capture in the same PR.

---

## Open questions (resolve only if they block)

None blocking for Phase 0–1. For Phase 2, default answers:

- **Unlock chord:** keep hold Esc 2s in the webview (Pass Esc).
- **Distribution:** notarized direct download; sandbox off.
- **Degraded mode:** if tap fails, either fail arm entirely (preferred) or offer “partial lock” (presentation only) with explicit labeling — pick **fail arm** for honesty unless product wants partial.

---

## References

- Research: [macos-keyboard-lock.md](./macos-keyboard-lock.md)
- Product: [project-context.md](./project-context.md)
