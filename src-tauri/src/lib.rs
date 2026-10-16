mod macos_keyboard_lock;

use std::sync::Mutex;

use macos_keyboard_lock::{KeyboardLockState, LockStatus, SharedLockState};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_prevent_default::Flags;

#[tauri::command]
fn set_baby_lock(app: AppHandle, locked: bool) -> Result<(), String> {
    let state = app.state::<SharedLockState>();

    if locked {
        // macOS capture first so we fail closed before mutating the window.
        macos_keyboard_lock::set_locked(&state, true)?;
        match apply_window_lock(&app, true) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = macos_keyboard_lock::set_locked(&state, false);
                Err(e)
            }
        }
    } else {
        let window_result = apply_window_lock(&app, false);
        // Always tear down capture even if window ops fail.
        let capture_result = macos_keyboard_lock::set_locked(&state, false);
        window_result?;
        capture_result
    }
}

fn apply_window_lock(app: &AppHandle, locked: bool) -> Result<(), String> {
    let window = main_window(app)?;
    // Order matters on Linux: always-on-top first, then fullscreen. Avoid
    // toggling decorations here — it can remount the webview and drop UI state.
    window
        .set_always_on_top(locked)
        .map_err(|e| e.to_string())?;
    window
        .set_fullscreen(locked)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_fullscreen(app: AppHandle, fullscreen: bool) -> Result<(), String> {
    let window = main_window(&app)?;
    window
        .set_fullscreen(fullscreen)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn macos_keyboard_lock_status(app: AppHandle) -> LockStatus {
    let state = app.state::<SharedLockState>();
    macos_keyboard_lock::status(&state)
}

#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    macos_keyboard_lock::open_accessibility_settings()
}

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Block webview/browser shortcuts (Ctrl+R, F5, Ctrl+W, etc.) that babies
    // can hit accidentally. OS-reserved chords still need platform-level
    // handling — see docs/macos-keyboard-lock.md.
    #[cfg(debug_assertions)]
    let prevent = tauri_plugin_prevent_default::Builder::new()
        .with_flags(Flags::all().difference(Flags::DEV_TOOLS | Flags::RELOAD))
        .build();

    #[cfg(all(not(debug_assertions), windows))]
    let prevent = {
        use tauri_plugin_prevent_default::PlatformOptions;
        tauri_plugin_prevent_default::Builder::new()
            .with_flags(Flags::all())
            .platform(
                PlatformOptions::new()
                    .browser_accelerator_keys(false)
                    .default_context_menus(false),
            )
            .build()
    };

    #[cfg(all(not(debug_assertions), not(windows)))]
    let prevent = tauri_plugin_prevent_default::Builder::new()
        .with_flags(Flags::all())
        .build();

    tauri::Builder::default()
        .plugin(prevent)
        .manage(Mutex::new(KeyboardLockState::new()) as SharedLockState)
        .invoke_handler(tauri::generate_handler![
            set_baby_lock,
            set_fullscreen,
            macos_keyboard_lock_status,
            open_accessibility_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
