mod macos_keyboard_lock;

#[cfg(target_os = "macos")]
use std::sync::mpsc;
use std::sync::Mutex;

use macos_keyboard_lock::{KeyboardLockState, LockStatus, SharedLockState};
use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow, WindowEvent};
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
    // Windows/Linux draw the menu bar inside the window, where a baby could
    // click File > Quit. macOS hides it via kiosk `HideMenuBar` instead.
    #[cfg(not(target_os = "macos"))]
    if locked {
        window.hide_menu().map_err(|e| e.to_string())?;
    } else {
        window.show_menu().map_err(|e| e.to_string())?;
    }
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

const PREFERENCES_MENU_ID: &str = "preferences";
const OPEN_PREFERENCES_EVENT: &str = "open-preferences";

/// Platform default menu plus File > Preferences…. Linux has no default File
/// submenu, so one is created there.
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let preferences = MenuItem::with_id(
        app,
        PREFERENCES_MENU_ID,
        "Preferences…",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let file_menu = menu.items()?.into_iter().find_map(|item| match item {
        MenuItemKind::Submenu(sub) if sub.text().ok().as_deref() == Some("File") => Some(sub),
        _ => None,
    });
    match file_menu {
        Some(file) => {
            file.prepend_items(&[&preferences, &PredefinedMenuItem::separator(app)?])?
        }
        None => menu.prepend(&Submenu::with_items(app, "File", true, &[&preferences])?)?,
    }
    Ok(menu)
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
        .menu(build_menu)
        .on_menu_event(|app, event| {
            // The webview ignores this while Baby Lock is on.
            if event.id() == PREFERENCES_MENU_ID {
                let _ = app.emit_to("main", OPEN_PREFERENCES_EVENT, ());
            }
        })
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
