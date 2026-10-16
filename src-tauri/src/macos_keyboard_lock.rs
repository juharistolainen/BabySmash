//! macOS Baby Lock keyboard capture.
//!
//! Phase 1: AppKit kiosk `presentationOptions` (no TCC).
//! Phase 2+: active CGEventTap (Accessibility).

use serde::Serialize;
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

pub struct KeyboardLockState {
    armed: bool,
    #[cfg(target_os = "macos")]
    saved_presentation: Option<platform::SavedPresentation>,
}

impl Default for KeyboardLockState {
    fn default() -> Self {
        Self {
            armed: false,
            #[cfg(target_os = "macos")]
            saved_presentation: None,
        }
    }
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
            accessibility_granted: false, // Phase 2
            secure_input: false,
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
pub fn set_locked(state: &SharedLockState, locked: bool) -> Result<(), String> {
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    if locked == guard.armed {
        return Ok(());
    }

    if locked {
        #[cfg(target_os = "macos")]
        {
            let saved = platform::arm_presentation()?;
            guard.saved_presentation = Some(saved);
            guard.armed = true;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = ERR_NEEDS_ACCESSIBILITY;
            guard.armed = true;
            Ok(())
        }
    } else {
        #[cfg(target_os = "macos")]
        {
            if let Some(saved) = guard.saved_presentation.take() {
                platform::disarm_presentation(saved);
            }
        }
        guard.armed = false;
        Ok(())
    }
}

pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        match std::process::Command::new("open").arg(url).status() {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!("open settings failed with {status}")),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility settings are only available on macOS".into())
    }
}

/// Re-apply kiosk presentation options after the app becomes active again.
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
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};

    #[derive(Clone, Copy)]
    pub struct SavedPresentation(NSApplicationPresentationOptions);

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
        let app = NSApplication::sharedApplication(mtm);
        app.setPresentationOptions(kiosk_options());
        Ok(())
    }

    pub fn arm_presentation() -> Result<SavedPresentation, String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "Baby Lock must arm on the main thread".to_string())?;
        let app = NSApplication::sharedApplication(mtm);
        let saved = SavedPresentation(app.presentationOptions());
        app.setPresentationOptions(kiosk_options());
        Ok(saved)
    }

    pub fn disarm_presentation(saved: SavedPresentation) {
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).setPresentationOptions(saved.0);
        }
    }
}
