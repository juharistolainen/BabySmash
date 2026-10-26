//! macOS Baby Lock keyboard capture.
//!
//! Layer 1: AppKit kiosk `presentationOptions` (no TCC).
//! Layer 2: active `CGEventTap` filter (Accessibility / PostEvent).

use serde::Serialize;
use std::ffi::c_void;
use std::sync::Mutex;

/// Returned from `set_baby_lock` when Accessibility is required but missing.
pub const ERR_NEEDS_ACCESSIBILITY: &str = "needs_accessibility";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockStatus {
    pub armed: bool,
    pub accessibility_granted: bool,
    pub secure_input: bool,
    /// True when this build includes the macOS capture path.
    pub supported: bool,
}

#[derive(Default)]
pub struct KeyboardLockState {
    armed: bool,
    #[cfg(target_os = "macos")]
    session: Option<platform::ArmedSession>,
}

impl KeyboardLockState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub type SharedLockState = Mutex<KeyboardLockState>;

pub fn status(state: &SharedLockState) -> LockStatus {
    let guard = state.lock().unwrap_or_else(|e| e.into_inner());
    #[cfg(target_os = "macos")]
    {
        LockStatus {
            armed: guard.armed,
            // Prefer IOHID for polling — creating a probe tap every few seconds is heavy.
            accessibility_granted: platform::accessibility_granted_fast(),
            secure_input: platform::secure_input_enabled(),
            supported: true,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        LockStatus {
            armed: guard.armed,
            accessibility_granted: false,
            secure_input: false,
            supported: false,
        }
    }
}

/// Must be called on the AppKit main thread on macOS (presentation options).
///
/// `ns_window` is the main window's `NSWindow*` on macOS (ignored elsewhere and
/// when unlocking). Locking stretches it over the screen instead of using native
/// fullscreen, which AppKit refuses while kiosk presentation options are active.
pub fn set_locked(
    state: &SharedLockState,
    locked: bool,
    ns_window: *mut c_void,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    if locked == guard.armed {
        return Ok(());
    }

    if locked {
        #[cfg(target_os = "macos")]
        {
            let session = platform::arm(ns_window)?;
            guard.session = Some(session);
            guard.armed = true;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (ERR_NEEDS_ACCESSIBILITY, ns_window);
            guard.armed = true;
            Ok(())
        }
    } else {
        #[cfg(target_os = "macos")]
        {
            if let Some(session) = guard.session.take() {
                platform::disarm(session);
            }
        }
        guard.armed = false;
        Ok(())
    }
}

pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        platform::open_accessibility_settings()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility settings are only available on macOS".into())
    }
}

/// Re-apply kiosk presentation options after the app becomes active again.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn reassert_if_armed(state: &SharedLockState) {
    #[cfg(target_os = "macos")]
    {
        let guard = state.lock().unwrap_or_else(|e| e.into_inner());
        if guard.armed {
            let _ = platform::apply_kiosk_presentation();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::ERR_NEEDS_ACCESSIBILITY;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSApplicationPresentationOptions, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::NSRect;

    // --- CoreGraphics FFI (active filter tap; return NULL to swallow) ---

    type CFMachPortRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CGEventRef = *mut c_void;
    type CGEventTapProxy = *mut c_void;

    type CGEventTapCallBack = Option<
        unsafe extern "C" fn(
            proxy: CGEventTapProxy,
            event_type: u32,
            event: CGEventRef,
            user_info: *mut c_void,
        ) -> CGEventRef,
    >;

    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_SESSION_EVENT_TAP: u32 = 1;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

    const K_CG_EVENT_KEY_DOWN: u32 = 10;
    const K_CG_EVENT_KEY_UP: u32 = 11;
    const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
    const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
    const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

    const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

    const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x100_000;
    const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x08_0000;
    const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x04_0000;

    const K_VK_ESCAPE: i64 = 0x35;

    /// ANSI function-key keycodes (F1–F20) — drop so Mission Control / Dictation F-keys don't fire.
    const FUNCTION_KEYCODES: &[i64] = &[
        0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, // F1–F8
        0x65, 0x6D, 0x67, 0x6F, 0x69, 0x6B, 0x71, // F9–F15
        0x6A, 0x40, 0x4F, 0x50, 0x5A, // F16–F20
    ];

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(mach_port: CFMachPortRef, enable: bool);
        fn CGEventGetFlags(event: CGEventRef) -> u64;
        fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
        fn CFMachPortCreateRunLoopSource(
            allocator: *mut c_void,
            port: CFMachPortRef,
            order: isize,
        ) -> CFRunLoopSourceRef;
        fn CFMachPortInvalidate(port: CFMachPortRef);
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
        fn CFRunLoopRemoveSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
        fn CFRunLoopRun();
        fn CFRunLoopStop(rl: CFRunLoopRef);
        fn CFRelease(cf: *mut c_void);
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopCommonModes: *const c_void;
    }

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn IsSecureEventInputEnabled() -> u8;
    }

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOHIDCheckAccess(request_type: u32) -> u32;
        fn IOHIDRequestAccess(request_type: u32) -> bool;
    }

    /// PostEvent — appears under System Settings → Accessibility.
    const K_IOHID_REQUEST_TYPE_POST_EVENT: u32 = 1;
    const K_IOHID_ACCESS_TYPE_GRANTED: u32 = 0;

    static ACTIVE_TAP_PORT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static TAP_RUN_LOOP: Mutex<Option<usize>> = Mutex::new(None);

    #[derive(Clone, Copy)]
    struct SavedPresentation(NSApplicationPresentationOptions);

    struct TapThread {
        port: CFMachPortRef,
        source: CFRunLoopSourceRef,
        stop: Arc<AtomicBool>,
        join: Option<JoinHandle<()>>,
    }

    // Safety: port/source only touched from the tap thread or after join.
    unsafe impl Send for TapThread {}

    /// Window geometry captured before the kiosk frame is applied.
    struct SavedWindow {
        window: Retained<NSWindow>,
        frame: NSRect,
        style: NSWindowStyleMask,
        movable: bool,
    }

    // Safety: created in `arm` and consumed in `disarm`, both on the main thread.
    unsafe impl Send for SavedWindow {}

    pub struct ArmedSession {
        saved_presentation: SavedPresentation,
        saved_window: Option<SavedWindow>,
        tap: TapThread,
    }

    fn keyboard_mask() -> u64 {
        (1u64 << K_CG_EVENT_KEY_DOWN)
            | (1u64 << K_CG_EVENT_KEY_UP)
            | (1u64 << K_CG_EVENT_FLAGS_CHANGED)
    }

    /// Lightweight TCC read for status polling (Accessibility / PostEvent pane).
    pub fn accessibility_granted_fast() -> bool {
        unsafe {
            IOHIDCheckAccess(K_IOHID_REQUEST_TYPE_POST_EVENT) == K_IOHID_ACCESS_TYPE_GRANTED
        }
    }

    /// Authoritative check used when arming: can we create an active filter tap?
    pub fn accessibility_filter_allowed() -> bool {
        unsafe {
            let port = CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                keyboard_mask(),
                Some(probe_callback),
                ptr::null_mut(),
            );
            if port.is_null() {
                return accessibility_granted_fast();
            }
            CFMachPortInvalidate(port);
            CFRelease(port);
            true
        }
    }

    unsafe extern "C" fn probe_callback(
        _proxy: CGEventTapProxy,
        _event_type: u32,
        event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        event
    }

    fn request_accessibility_prompt() {
        unsafe {
            let _ = IOHIDRequestAccess(K_IOHID_REQUEST_TYPE_POST_EVENT);
        }
    }

    pub fn secure_input_enabled() -> bool {
        unsafe { IsSecureEventInputEnabled() != 0 }
    }

    pub fn open_accessibility_settings() -> Result<(), String> {
        // macOS only lists apps that have requested access, so register first;
        // otherwise the parent lands on a pane without BabySmash in it.
        if !accessibility_granted_fast() {
            request_accessibility_prompt();
        }
        let url =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        match std::process::Command::new("open").arg(url).status() {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!("open settings failed with {status}")),
            Err(e) => Err(e.to_string()),
        }
    }

    fn kiosk_options() -> NSApplicationPresentationOptions {
        NSApplicationPresentationOptions::HideDock
            | NSApplicationPresentationOptions::HideMenuBar
            | NSApplicationPresentationOptions::DisableProcessSwitching
            | NSApplicationPresentationOptions::DisableForceQuit
            | NSApplicationPresentationOptions::DisableSessionTermination
            | NSApplicationPresentationOptions::DisableAppleMenu
            | NSApplicationPresentationOptions::DisableHideApplication
    }

    pub fn apply_kiosk_presentation() -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "presentation options must be set on the main thread".to_string())?;
        NSApplication::sharedApplication(mtm).setPresentationOptions(kiosk_options());
        Ok(())
    }

    /// Re-apply a style mask without losing keyboard focus. AppKit rebuilds the
    /// frame view when `Titled` toggles, which drops the first responder.
    fn set_style_mask_keep_focus(window: &NSWindow, style: NSWindowStyleMask) {
        let responder = window.firstResponder();
        window.setStyleMask(style);
        window.makeFirstResponder(responder.as_deref());
    }

    /// Cover the window's screen with a borderless frame. Native fullscreen
    /// (`toggleFullScreen:`) needs a Spaces transition, which AppKit aborts
    /// while `DisableProcessSwitching` / `HideDock` are set — the window
    /// animates toward fullscreen and snaps back.
    fn enter_kiosk_window(window: Retained<NSWindow>) -> Option<SavedWindow> {
        let style = window.styleMask();
        // Already in native fullscreen: it covers the screen; leave it alone.
        if style.contains(NSWindowStyleMask::FullScreen) {
            return None;
        }
        let screen = window.screen()?;
        let saved = SavedWindow {
            frame: window.frame(),
            style,
            movable: window.isMovable(),
            window,
        };
        let kiosk_style = style
            & !(NSWindowStyleMask::Titled
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable);
        set_style_mask_keep_focus(&saved.window, kiosk_style);
        saved.window.setFrame_display(screen.frame(), true);
        saved.window.setMovable(false);
        Some(saved)
    }

    fn exit_kiosk_window(saved: SavedWindow) {
        set_style_mask_keep_focus(&saved.window, saved.style);
        saved.window.setFrame_display(saved.frame, true);
        saved.window.setMovable(saved.movable);
    }

    fn should_drop_key(event: CGEventRef) -> bool {
        unsafe {
            let flags = CGEventGetFlags(event);
            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let chord = (flags
                & (K_CG_EVENT_FLAG_MASK_COMMAND
                    | K_CG_EVENT_FLAG_MASK_CONTROL
                    | K_CG_EVENT_FLAG_MASK_ALTERNATE))
                != 0;

            // Esc with no Cmd/Ctrl/Opt — pass for parent unlock hold in the webview.
            if keycode == K_VK_ESCAPE && !chord {
                return false;
            }
            if chord {
                return true;
            }
            if FUNCTION_KEYCODES.contains(&keycode) {
                return true;
            }
            false
        }
    }

    unsafe extern "C" fn lock_tap_callback(
        _proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
            || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
        {
            let port = ACTIVE_TAP_PORT.load(Ordering::SeqCst);
            if !port.is_null() {
                CGEventTapEnable(port, true);
            }
            return ptr::null_mut();
        }

        if (event_type == K_CG_EVENT_KEY_DOWN
            || event_type == K_CG_EVENT_KEY_UP
            || event_type == K_CG_EVENT_FLAGS_CHANGED)
            && should_drop_key(event)
        {
            return ptr::null_mut();
        }

        event
    }

    fn create_filter_tap() -> Result<CFMachPortRef, String> {
        unsafe {
            let mut port = CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                keyboard_mask(),
                Some(lock_tap_callback),
                ptr::null_mut(),
            );
            if port.is_null() {
                port = CGEventTapCreate(
                    K_CG_SESSION_EVENT_TAP,
                    K_CG_HEAD_INSERT_EVENT_TAP,
                    K_CG_EVENT_TAP_OPTION_DEFAULT,
                    keyboard_mask(),
                    Some(lock_tap_callback),
                    ptr::null_mut(),
                );
            }
            if port.is_null() {
                request_accessibility_prompt();
                return Err(ERR_NEEDS_ACCESSIBILITY.into());
            }
            Ok(port)
        }
    }

    fn start_event_tap() -> Result<TapThread, String> {
        let port = create_filter_tap()?;
        unsafe {
            let source = CFMachPortCreateRunLoopSource(ptr::null_mut(), port, 0);
            if source.is_null() {
                CFMachPortInvalidate(port);
                CFRelease(port);
                return Err("failed to create run loop source for event tap".into());
            }

            ACTIVE_TAP_PORT.store(port, Ordering::SeqCst);

            let stop = Arc::new(AtomicBool::new(false));
            let stop_thread = Arc::clone(&stop);
            let port_addr = port as usize;
            let source_addr = source as usize;

            let join = thread::Builder::new()
                .name("babysmash-key-tap".into())
                .spawn(move || {
                    let port = port_addr as CFMachPortRef;
                    let source = source_addr as CFRunLoopSourceRef;
                    // Closure body inherits the enclosing `unsafe` context.
                    let rl = CFRunLoopGetCurrent();
                    {
                        let mut slot = TAP_RUN_LOOP.lock().unwrap_or_else(|e| e.into_inner());
                        *slot = Some(rl as usize);
                    }
                    CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
                    CGEventTapEnable(port, true);
                    // Run until stop; CFRunLoopStop wakes us.
                    while !stop_thread.load(Ordering::SeqCst) {
                        CFRunLoopRun();
                    }
                    CFRunLoopRemoveSource(rl, source, kCFRunLoopCommonModes);
                    let mut slot = TAP_RUN_LOOP.lock().unwrap_or_else(|e| e.into_inner());
                    *slot = None;
                })
                .map_err(|e| {
                    CFMachPortInvalidate(port);
                    CFRelease(source);
                    CFRelease(port);
                    ACTIVE_TAP_PORT.store(ptr::null_mut(), Ordering::SeqCst);
                    e.to_string()
                })?;

            // Allow the tap thread to register its run loop.
            thread::sleep(Duration::from_millis(30));

            Ok(TapThread {
                port,
                source,
                stop,
                join: Some(join),
            })
        }
    }

    fn stop_event_tap(mut tap: TapThread) {
        tap.stop.store(true, Ordering::SeqCst);
        unsafe {
            if !tap.port.is_null() {
                CGEventTapEnable(tap.port, false);
            }
            if let Some(rl_addr) = *TAP_RUN_LOOP.lock().unwrap_or_else(|e| e.into_inner()) {
                CFRunLoopStop(rl_addr as CFRunLoopRef);
            }
        }
        if let Some(join) = tap.join.take() {
            let _ = join.join();
        }
        unsafe {
            if !tap.source.is_null() {
                CFRelease(tap.source);
            }
            if !tap.port.is_null() {
                CFMachPortInvalidate(tap.port);
                CFRelease(tap.port);
            }
        }
        ACTIVE_TAP_PORT.store(ptr::null_mut(), Ordering::SeqCst);
    }

    pub fn arm(ns_window: *mut c_void) -> Result<ArmedSession, String> {
        // Fail closed: require a working active filter tap before claiming lock.
        if !accessibility_filter_allowed() {
            request_accessibility_prompt();
            return Err(ERR_NEEDS_ACCESSIBILITY.into());
        }

        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "Baby Lock must arm on the main thread".to_string())?;
        let app = NSApplication::sharedApplication(mtm);
        let saved = SavedPresentation(app.presentationOptions());
        app.setPresentationOptions(kiosk_options());

        let tap = match start_event_tap() {
            Ok(t) => t,
            Err(e) => {
                app.setPresentationOptions(saved.0);
                return Err(e);
            }
        };

        // Safety: `ns_window` comes from Tauri's `ns_window()` for a live window.
        let saved_window = unsafe { Retained::retain(ns_window.cast::<NSWindow>()) }
            .and_then(enter_kiosk_window);

        Ok(ArmedSession {
            saved_presentation: saved,
            saved_window,
            tap,
        })
    }

    pub fn disarm(session: ArmedSession) {
        stop_event_tap(session.tap);
        if let Some(saved_window) = session.saved_window {
            exit_kiosk_window(saved_window);
        }
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm)
                .setPresentationOptions(session.saved_presentation.0);
        }
    }
}
