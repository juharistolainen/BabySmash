mod macos_keyboard_lock;

#[cfg(target_os = "macos")]
use std::sync::mpsc;
use std::sync::Mutex;

use macos_keyboard_lock::{KeyboardLockState, LockStatus, SharedLockState};
use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_prevent_default::Flags;

#[tauri::command]
fn set_baby_lock(app: AppHandle, locked: bool) -> Result<(), String> {
    if locked {
        // macOS capture first so we fail closed before mutating the window.
        run_lock_on_main(&app, true)?;
        match apply_window_lock(&app, true) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = run_lock_on_main(&app, false);
                Err(e)
            }
        }
    } else {
        let window_result = apply_window_lock(&app, false);
        // Always tear down capture even if window ops fail.
        let capture_result = run_lock_on_main(&app, false);
        window_result?;
        capture_result
    }
}

fn run_lock_on_main(app: &AppHandle, locked: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let app2 = app.clone();
        let (tx, rx) = mpsc::channel();
        app.run_on_main_thread(move || {
            let state = app2.state::<SharedLockState>();
            // Unlock must still tear down capture if the window lookup fails.
            let ns_window = main_window(&app2)
                .ok()
                .and_then(|w| w.ns_window().ok())
                .unwrap_or(std::ptr::null_mut());
            let _ = tx.send(macos_keyboard_lock::set_locked(&state, locked, ns_window));
        })
        .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let state = app.state::<SharedLockState>();
        macos_keyboard_lock::set_locked(&state, locked, std::ptr::null_mut())
    }
}

fn apply_window_lock(app: &AppHandle, locked: bool) -> Result<(), String> {
    let window = main_window(app)?;
    // Order matters on Linux: always-on-top first, then fullscreen. Avoid
    // toggling decorations here — it can remount the webview and drop UI state.
    window
        .set_always_on_top(locked)
        .map_err(|e| e.to_string())?;
    // macOS: the lock module sizes the window over the screen itself, because
    // native fullscreen bounces back while kiosk presentation options are set.
    #[cfg(not(target_os = "macos"))]
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
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(true) = event {
                #[cfg(target_os = "macos")]
                {
                    let app = window.app_handle().clone();
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        let app = handle;
                        let state = app.state::<SharedLockState>();
                        macos_keyboard_lock::reassert_if_armed(&state);
                    });
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = window;
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
