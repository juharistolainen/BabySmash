# macOS keyboard lock for BabySmash Baby Lock

Research for suppressing OS-reserved and app shortcuts while Baby Lock is active (Spotlight, Mission Control, Cmd+Q, Cmd+Tab, etc.). Current first slice only blocks webview chords via `tauri-plugin-prevent-default` + DOM `preventDefault`; it cannot stop macOS system handlers.

**Audience:** engineers implementing the next macOS Baby Lock slice.  
**Scope:** modern macOS (10.15 Catalina onward; TCC model still applies on macOS 14/15/26).  
**Claim bar:** verified against Apple docs / WWDC / Apple DTS forum posts where possible; community sources noted as secondary.

---

## Top-line recommendation

Use a **two-layer** approach when Baby Lock arms:

1. **AppKit presentation options** (kiosk mode) — no special TCC; disables Cmd+Tab / Exposé, Force Quit panel, session-termination UI, Dock/menu bar, Apple menu, Hide.
2. **Active `CGEventTap` filter** (`kCGEventTapOptionDefault`, prefer `kCGHIDEventTap` + `kCGHeadInsertEventTap`, swallow by returning `NULL`) — required to stop Spotlight, Mission Control chords, Cmd+Q, screenshots, media keys, etc.

**Swallowing keys requires Accessibility-class permission** (modifying tap), not Input Monitoring alone. Prefer shipping **notarized outside the Mac App Store without App Sandbox** for this feature; sandbox historically blocks active filter taps. Keep a parent unlock chord and leave the mouse usable so the user can Force Quit via UI if the tap fails.

You cannot honestly promise “100% of all keyboard shortcuts forever.” Power hardware and Secure Input are hard limits.

---

## 1. What is and is not possible

### Possible (with correct APIs + user consent)

| Goal | Mechanism | Notes |
|------|-----------|--------|
| Block Cmd+Tab / Exposé / Mission Control UI via process switcher | `NSApplication.presentationOptions` + `disableProcessSwitching` (+ hide/autoHide dock) | Official kiosk API; only while app is active |
| Disable Force Quit panel (⌘⌥⎋) | `disableForceQuit` | Does **not** remove Apple menu Force Quit if menu is still reachable |
| Disable Restart / Shut Down / Log Out power-key panel | `disableSessionTermination` | Power **button hold** still works |
| Hide Dock / menu bar; disable Apple menu / Hide | matching presentation flags | Spotlight **menu** disabled when dock hidden |
| Swallow almost all keyDown/keyUp/flagsChanged before system shortcuts | Active `CGEventTap` returning `NULL` | Needs Accessibility (see §3) |
| Keep receiving smash keys inside BabySmash | Tap runs in-process; forward selected events into your app logic, or let non-chord keys through / re-post | Design choice |

### Not possible / not reliable as a consumer app

| Goal | Reality |
|------|---------|
| Silent, permissionless OS-wide shortcut kill | No. Catalina+ requires TCC user approval for listen/modify taps ([WWDC19-701](https://developer.apple.com/videos/play/wwdc2019/701/)) |
| Block power button / Touch ID hold to force power off | Hardware / firmware path; not interceptable as a CGEvent |
| Block Secure Event Input traffic | While Secure Input is on, keyboard taps go deaf ([TN2150](https://developer.apple.com/library/archive/technotes/tn2150/_index.html)) |
| iOS-style Guided Access / Single App Mode | **Not available on macOS** |
| MDM `allowKeyboardShortcuts: false` | Apple’s device-management schema marks this for **iOS**, `macOS: introduced: n/a` ([applicationaccess.yaml](https://github.com/apple/device-management/blob/release/mdm/profiles/com.apple.applicationaccess.yaml)) |
| Observe-only APIs that also suppress delivery | `NSEvent.addGlobalMonitor…` cannot modify or prevent delivery ([Apple Event Overview](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/MonitoringEvents/MonitoringEvents.html)) |
| Full Mac App Store + App Sandbox + active filter tap | Historically broken (`deny hid-control`); listen-only is the sandbox-friendly path (see §3) |

### Version notes

- **≤ 10.14:** Assistive Devices / Accessibility was the gate for keyboard taps; no separate Input Monitoring pane.
- **10.15+ (Catalina):** Split: listen-only tap → **Input Monitoring**; modifying tap → **Accessibility** ([WWDC19-701](https://developer.apple.com/videos/play/wwdc2019/701/?time=1461)).
- **Helpers:** `CGPreflightListenEventAccess` / `CGRequestListenEventAccess` (and PostEvent twins); also `IOHIDCheckAccess` / `IOHIDRequestAccess` with `kIOHIDRequestTypeListenEvent` / `PostEvent` ([IOHIDRequestAccess](https://developer.apple.com/documentation/iokit/3181574-iohidrequestaccess)).
- **Today (macOS 14/15+):** Same TCC model. Apps may need a restart after the user toggles permissions; code-signing / bundle ID changes invalidate prior grants.

---

## 2. APIs / approaches

### A. `NSApplication.presentationOptions` (kiosk / presentation mode) — **do this first**

Official AppKit API for games/kiosks. Documented in:

- [NSApplication.PresentationOptions](https://developer.apple.com/documentation/appkit/nsapplication/presentationoptions-swift.struct)
- Archived [Kiosk Mode Technical Note](https://developer.apple.com/library/archive/technotes/KioskMode/Introduction/Introduction.html)

Useful flags for Baby Lock (combine carefully; invalid combos raise `NSInvalidArgumentException`):

| Flag | Effect |
|------|--------|
| `hideDock` or `autoHideDock` | Required companion for several disable flags; Spotlight **menu** disabled |
| `hideMenuBar` or `autoHideMenuBar` | Remove chrome |
| `disableProcessSwitching` | Cmd+Tab + Exposé disabled |
| `disableForceQuit` | Cmd+Opt+Esc panel disabled |
| `disableSessionTermination` | Power-key Restart/Shut Down/Log Out panel disabled |
| `disableAppleMenu` | Apple menu items disabled |
| `disableHideApplication` | App Hide disabled |
| `fullScreen` | Mark fullscreen presentation |

**Gaps:** Does **not** reliably stop Cmd+Space (Spotlight search), many Keyboard Shortcuts prefs bindings, Cmd+Q (app quit is a menu key equivalent, not the Force Quit panel), screenshots (Cmd+Shift+3/4/5), Dictation / media keys, Control+arrows Spaces, etc. Treat as layer 1 only.

Options apply **while your app is active**. If focus is stolen, system UI can return. Pair with always-on-top + fullscreen (already in `set_baby_lock`) and optionally re-assert options on `NSApplicationDidBecomeActive`.

### B. Active `CGEventTap` (required for “block shortcuts”)

[`CGEvent.tapCreate`](https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:)) / `CGEventTapCreate`:

| Parameter | Recommended for Baby Lock |
|-----------|---------------------------|
| Location | Prefer `kCGHIDEventTap` (earliest); fall back to `kCGSessionEventTap` if create fails |
| Placement | `kCGHeadInsertEventTap` |
| Options | **`kCGEventTapOptionDefault`** (active filter). `listenOnly` cannot swallow |
| Mask | `keyDown` \| `keyUp` \| `flagsChanged` (and optionally `NX_SYSDEFINED` / media if needed) |
| Callback | Return `NULL` / `nil` to discard; return the event to pass through |

**Critical:** Returning `NULL` from an active tap is what stops Spotlight / Mission Control / quit chords. Listen-only taps are useless for Baby Lock’s “cannot activate” requirement.

Lifecycle:

1. Create tap → `CFMachPortCreateRunLoopSource` → add to run loop (main or dedicated thread with its own run loop).
2. Handle `kCGEventTapDisabledByTimeout` / `kCGEventTapDisabledByUserInput` by re-enabling (`CGEventTapEnable`).
3. On Baby Lock off: invalidate port, remove source, clear presentation options.
4. If Accessibility is revoked while an active tap is installed, input can hang system-wide until restart (Chromium and others specially handle this). **Disable the tap immediately** if permission disappears.

Older docs still say HID taps require root; in practice, with Accessibility granted, non-root apps create HID/session taps. Treat create-failure as the authoritative check.

### C. `NSEvent` monitors — **not sufficient**

- **Global monitor:** observe only; cannot prevent delivery.
- **Local monitor:** can rewrite/drop events **for your app**; system shortcuts that are handled outside your responder chain still fire.

Use local monitors only as a webview/AppKit supplement, not as OS lock.

### D. Carbon `RegisterEventHotKey` / `tauri-plugin-global-shortcut`

Registers hotkeys at lower priority than system shortcuts. System bindings win. Wrong tool for Baby Lock.

### E. IOHID / raw HID drivers

Overkill and sandbox-hostile. Apple DTS steers keyboard monitoring/filter use cases to `CGEventTap` + Input Monitoring / Accessibility, not custom HID clients ([forums example](https://developer.apple.com/forums/thread/724608)). Skip for v1.

### F. Secure Event Input

`EnableSecureEventInput` / `DisableSecureEventInput` / `IsSecureEventInputEnabled` ([TN2150](https://developer.apple.com/library/archive/technotes/tn2150/_index.html)). When any process enables Secure Input (password fields, some auth UI), **all** keyboard event taps stop receiving keys. BabySmash should:

- **Not** leave Secure Input on itself.
- Poll `IsSecureEventInputEnabled()` while locked and surface a parent-facing warning if the tap goes deaf.
- Rely on mouse + presentation options as partial backup.

### G. MDM / configuration profiles

Useful for school/lab fleets (restrict apps, Finder, Dock) but **not** a substitute for in-app Baby Lock, and **not** a portable consumer solution. `allowKeyboardShortcuts` is iOS-oriented. Do not block product design on MDM.

### H. Fullscreen presentation helpers

`NSView.enterFullScreenMode:withOptions:` + `NSFullScreenModeApplicationPresentationOptions` can apply kiosk flags for the fullscreen duration (see Kiosk TN). Tauri already toggles window fullscreen; you can still set `NSApp.presentationOptions` from Rust/ObjC when lock arms.

---

## 3. Permissions, entitlements, notarization, sandbox, App Store

### TCC services that matter

| TCC (approx.) | System Settings UI | Needed for |
|---------------|-------------------|------------|
| `ListenEvent` | Privacy & Security → **Input Monitoring** | Listen-only taps / observing keys |
| Accessibility / `PostEvent` | Privacy & Security → **Accessibility** | **Modifying/filtering taps** (swallow), posting synthetic events. UI label is “Accessibility”; TCC splits `Accessibility` vs `PostEvent` under the hood ([Quinn / DTS](https://developer.apple.com/forums/thread/789896)) |

WWDC 2019 explicit rule: *listen-only → Input Monitoring; modifying tap → Accessibility features* ([WWDC19-701](https://developer.apple.com/videos/play/wwdc2019/701/?time=1461)). Baby Lock needs the modifying path.

### How to prompt

1. Explain in-UI why Baby Lock needs Accessibility (“so Spotlight and quit shortcuts don’t fire”).
2. Call one of:
   - `AXIsProcessTrustedWithOptions` with prompt option, and/or
   - `IOHIDRequestAccess(kIOHIDRequestTypePostEvent)`, and/or
   - `CGRequestPostEventAccess()`, and/or
   - Attempt `CGEventTapCreate(…, kCGEventTapOptionDefault, …)` (first failure often triggers the system sheet).
3. Deep-link settings if still denied:  
   `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`  
   (Input Monitoring pane: `Privacy_ListenEvent` — secondary if you also use listen-only probes).
4. After the user toggles the switch, **relaunch** or recreate the tap; grants are tied to the signed binary’s identity.
5. Authoritative runtime check for filtering: try creating an active tap; `NULL` means not authorized ([forum technique](https://developer.apple.com/forums/thread/735204)). Do not trust preflight alone.

No special Info.plist “usage description” string exists for Accessibility the way iOS camera strings work; the system sheet + Settings toggle is the UX.

### Entitlements / Hardened Runtime

For a notarized Developer ID app (recommended distribution for this feature):

- Hardened Runtime: enable as required for notarization.
- If you use any Apple-event / automation later, add the matching entitlement; **not required** for CGEventTap alone.
- Do **not** assume a public `com.apple.security.accessibility` entitlement unlocks taps — user TCC grant does.

### App Sandbox

Apple documents that App Sandbox restricts accessibility APIs ([Protecting user data with App Sandbox](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox)).

- **Listen-only** `CGEventTap` + Input Monitoring: DTS states this works in sandboxed / Mac App Store apps ([forums](https://developer.apple.com/forums/thread/707680)).
- **Active filter** taps: longstanding sandbox denial (`deny hid-control`) ([OpenRadar 10961642](https://openradar.appspot.com/10961642)). Treat **sandbox ON ⇒ cannot reliably swallow keys**.

**Product implication:** for real Baby Lock on macOS, ship a **non-sandboxed**, notarized build (typical for Tauri desktop). Mac App Store would need a degraded Baby Lock (presentation options + webview blocking only) unless Apple’s policy/tech changes.

### Notarization

Notarization does not grant TCC rights. It only makes Gatekeeper happy. Users still approve Accessibility once per signing identity.

---

## 4. What remains unblockable (even with full permissions)

Assume active tap + kiosk options + Accessibility granted:

| Still works / escapes | Why |
|----------------------|-----|
| Hold power / Touch ID to force power off | Hardware |
| Secure Event Input sessions | Taps receive no keyboard events ([TN2150](https://developer.apple.com/library/archive/technotes/tn2150/_index.html), [Hammerspoon](https://www.hammerspoon.org/docs/hs.eventtap)) |
| Stuck Secure Input after another app misbehaves | Known ecosystem issue; logout may be required |
| Mouse / trackpad | Unless you also tap pointer events (don’t for recovery) |
| Force Quit via Apple menu | Blocked only if `disableAppleMenu` / menu hidden; if UI chrome returns, menu path remains |
| Loginwindow / firmware shortcuts | Outside app event stream |
| Accessibility revoked mid-session | Tap may die or wedge input; fail closed and unlock Baby Lock |

**Usually blockable** with an active HID-head tap (community-validated; still regression-test on target OS):

- Cmd+Space / Cmd+Opt+Space (Spotlight)
- Cmd+Tab (also covered by presentation options)
- Ctrl+↑ / Mission Control / Control+←→ Spaces
- Cmd+Q / Cmd+W / Cmd+H / Cmd+M
- Cmd+Shift+3/4/5 screenshots
- Media / dictation function keys (may need sys-defined event mask)
- App switcher / Exposé chords

Always keep a **parent unlock chord** (e.g. current “hold Esc 2s”) that the tap recognizes and does **not** swallow after the hold threshold, plus mouse access to quit if the chord fails.

---

## 5. Practical recommendation for Tauri (Rust)

### Stack choice

| Option | Verdict |
|--------|---------|
| **`cgevents` crate** ([docs.rs/cgevents](https://docs.rs/cgevents)) | Prefer. Safe `EventTap`, `TapAction::Drop` / `Pass`, auto-reenable on timeout, `request_*_access` helpers. macOS 10.15+. Verify Drop maps to C `NULL` in your pinned version (crate docs claim filtering support). |
| Raw `core-graphics` / `objc` FFI | Fine if `cgevents` packaging (Swift bridge) is awkward in CI; return `std::ptr::null_mut()` from the C callback. Some wrappers’ `Option::None` has historically **not** become C NULL — test with Spotlight. |
| `tauri-plugin-macos-input-monitor` | Exists for hotkey consume; useful reference for FFI pitfalls, but Baby Lock wants “swallow everything except unlock + smash routing,” not a hotkey table. Prefer first-party module. |
| `tauri-plugin-global-shortcut` | Insufficient (system shortcuts win). |
| Swift sidecar | Unnecessary if Rust/ObjC bridge works. |

For presentation options, call AppKit via `objc` / `objc2-app-kit` (`NSApp.setPresentationOptions_`) from the same lock command.

### Distribution

- macOS target: **no App Sandbox** entitlement for the Baby Lock build.
- Sign + notarize Developer ID.
- Document Accessibility grant in first-run / Baby Lock arming UX.

### Integration points in current BabySmash

Existing hooks:

- Frontend: `src/lib/babyLock.ts` → `invoke("set_baby_lock", { locked })`
- Backend: `src-tauri/src/lib.rs` → `set_baby_lock` (always-on-top + fullscreen only)
- Webview chord blocking: `tauri-plugin-prevent-default` (keep; necessary but not sufficient)

Extend `set_baby_lock(true)` to:

1. Request/verify Accessibility (and show settings if missing).
2. Apply kiosk `presentationOptions`.
3. Install/enable the active keyboard tap.
4. Keep existing window fullscreen / always-on-top.

On `false`: tear down tap, restore `presentationOptions` default, clear always-on-top/fullscreen as today.

Optional: emit a Tauri event `baby-lock-permission-denied` / `baby-lock-secure-input` for UI.

---

## 6. Minimal implementation sketch

Pseudo-Rust (illustrative — not drop-in):

```rust
// src-tauri/src/macos_keyboard_lock.rs  (cfg(target_os = "macos"))

struct KeyboardLock {
    // hold CFMachPort / EventTap + previous presentation options
}

impl KeyboardLock {
    fn arm(&mut self) -> Result<(), LockError> {
        // 1) Permission
        if !accessibility_filter_allowed() {
            request_accessibility_prompt();
            return Err(LockError::NeedsAccessibility);
        }

        // 2) Kiosk presentation (objc2 / objc)
        set_presentation_options(
            HIDE_DOCK | HIDE_MENU_BAR | DISABLE_PROCESS_SWITCHING
            | DISABLE_FORCE_QUIT | DISABLE_SESSION_TERMINATION
            | DISABLE_APPLE_MENU | DISABLE_HIDE_APPLICATION,
        )?;

        // 3) Active tap — swallow all keys except parent unlock bookkeeping
        self.tap = Some(EventTap::new_with_options(
            TapLocation::Hid,           // fall back Session
            TapPlacement::HeadInsert,
            CGEventTapOptions::Default, // NOT listen-only
            KEYBOARD_MASK,
            |ev| {
                if is_unlock_chord_progress(ev) {
                    TapAction::Pass // or Drop after you handled it in-app
                } else {
                    // Optionally forward keycode to smash audio/visual via channel
                    TapAction::Drop
                }
            },
        )?);
        Ok(())
    }

    fn disarm(&mut self) {
        self.tap.take();
        restore_presentation_options();
    }
}
```

Wire into existing command:

```rust
#[tauri::command]
fn set_baby_lock(app: AppHandle, locked: bool) -> Result<(), String> {
    let window = main_window(&app)?;
    window.set_always_on_top(locked).map_err(|e| e.to_string())?;
    window.set_fullscreen(locked).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    macos_keyboard_lock::set_locked(locked)?;
    Ok(())
}
```

**UX checklist when arming fails:** keep fullscreen optional; do not claim “locked” if the tap did not install; show one-shot instructions for Accessibility; offer “Open System Settings.”

**Test matrix (manual):** Spotlight, Mission Control, Cmd+Tab, Cmd+Q, Cmd+H, screenshots, Control+←/→, media keys, hold-Esc unlock, Accessibility deny path, revoke Accessibility while locked, password field in another app (Secure Input), mouse still moves.

---

## Hard blockers / product decisions

1. **No silent lock** — parent must grant Accessibility; without it, OS shortcuts remain.
2. **App Store + Sandbox vs real shortcut suppression** — pick distribution; full swallow conflicts with classic sandbox limits.
3. **Not 100%** — power button + Secure Input (and rare stuck Secure Input) remain escape hatches; document them.
4. **Safety** — never install an active tap without a tested unlock path; prefer not swallowing mouse events.

---

## Sources

### Apple

- [WWDC 2019 Session 701 — Advances in macOS Security](https://developer.apple.com/videos/play/wwdc2019/701/) (listen-only vs modifying tap permissions)
- [CGEvent.tapCreate](https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:))
- [CGPreflightListenEventAccess](https://developer.apple.com/documentation/coregraphics/cgpreflightlisteneventaccess())
- [CGRequestListenEventAccess](https://developer.apple.com/documentation/coregraphics/cgrequestlisteneventaccess())
- [IOHIDRequestAccess](https://developer.apple.com/documentation/iokit/3181574-iohidrequestaccess)
- [NSApplication.PresentationOptions](https://developer.apple.com/documentation/appkit/nsapplication/presentationoptions-swift.struct)
- [disableProcessSwitching](https://developer.apple.com/documentation/appkit/nsapplication/presentationoptions-swift.struct/disableprocessswitching)
- [Kiosk Mode Technical Note](https://developer.apple.com/library/archive/technotes/KioskMode/Introduction/Introduction.html)
- [TN2150 — Using Secure Event Input Fairly](https://developer.apple.com/library/archive/technotes/tn2150/_index.html)
- [Monitoring Events (NSEvent monitors)](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/MonitoringEvents/MonitoringEvents.html)
- [Protecting user data with App Sandbox](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox)
- [Allow accessibility apps — Mac User Guide](https://support.apple.com/guide/mac-help/allow-accessibility-apps-to-access-your-mac-mh43185/mac)
- [Force Quit — Apple Support](https://support.apple.com/en-us/102586)

### Apple Developer Forums (DTS / Quinn)

- [Accessibility permission in sandboxed app — use CGEventTap + Input Monitoring for listen](https://developer.apple.com/forums/thread/707680)
- [Sandboxed CGEventTap / Input Monitoring](https://developer.apple.com/forums/thread/724608)
- [ListenEvent vs PostEvent vs Accessibility](https://developer.apple.com/forums/thread/735204)
- [Input Monitoring vs Accessibility APIs in sandbox](https://developer.apple.com/forums/thread/789896)

### Secondary / confirmatory

- [Unshaky #157 — WWDC transcript: modifying tap needs Accessibility](https://github.com/aahung/Unshaky/issues/157)
- [OpenRadar — sandbox breaks active filter taps](https://openradar.appspot.com/10961642)
- [Apple device-management `allowKeyboardShortcuts` (iOS only)](https://github.com/apple/device-management/blob/release/mdm/profiles/com.apple.applicationaccess.yaml)
- [cgevents crate — filtering taps + permissions](https://docs.rs/cgevents/latest/cgevents/)
- [Hammerspoon `hs.eventtap.isSecureInputEnabled`](https://www.hammerspoon.org/docs/hs.eventtap)

---

## Suggested next engineering step

See the phased plan: [macos-keyboard-lock-plan.md](./macos-keyboard-lock-plan.md).

Implement `macos_keyboard_lock` behind `set_baby_lock`, gated on Accessibility, with presentation options + active tap, and add a short QA checklist for the shortcuts in §4. Defer Mac App Store until there is an accepted degraded mode.
