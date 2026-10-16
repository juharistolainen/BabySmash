//! macOS Baby Lock keyboard capture (scaffold).
//!
//! Phase 0: shared state, status/settings commands, macOS/non-macOS stubs.
//! Later phases fill in presentation options and the active CGEventTap.

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
}

impl Default for KeyboardLockState {
    fn default() -> Self {
        Self { armed: false }
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
            accessibility_granted: false, // filled in Phase 2
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

/// Arm or disarm macOS keyboard capture layers (stub in Phase 0).
pub fn set_locked(state: &SharedLockState, locked: bool) -> Result<(), String> {
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    guard.armed = locked;
    let _ = ERR_NEEDS_ACCESSIBILITY; // used by later phases
    Ok(())
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

pub fn reassert_if_armed(_state: &SharedLockState) {
    // Phase 1+: re-apply kiosk presentation options when still armed.
}
