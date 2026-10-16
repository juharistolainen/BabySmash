mod macos_keyboard_lock;

#[cfg(target_os = "macos")]
use std::sync::mpsc;
use std::sync::Mutex;

use macos_keyboard_lock::{KeyboardLockState, LockStatus, SharedLockState};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildInfo {
    name: String,
    version: String,
    /// Unix seconds when build.rs last ran.
    build_time: u64,
    rustc: &'static str,
    target: &'static str,
    tauri: &'static str,
    /// WebKit (macOS), WebView2 (Windows) or WebKitGTK (Linux) version.
    webview: Option<String>,
    os: &'static str,
    arch: &'static str,
}

#[tauri::command]
fn app_build_info(app: AppHandle) -> BuildInfo {
    let pkg = app.package_info();
    BuildInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        build_time: env!("BABYSMASH_BUILD_TIME").parse().unwrap_or(0),
        rustc: env!("BABYSMASH_RUSTC_VERSION"),
        target: env!("BABYSMASH_TARGET"),
        tauri: tauri::VERSION,
        webview: tauri::webview_version().ok(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

const PREFERENCES_MENU_ID: &str = "preferences";
const OPEN_PREFERENCES_EVENT: &str = "open-preferences";
const ABOUT_MENU_ID: &str = "about";
const OPEN_ABOUT_EVENT: &str = "open-about";

/// Mirrors Tauri's `Menu::default`, but with File > Preferences… and an About
/// item that opens the app's own dialog instead of the stock native panel.
/// About sits in the app menu on macOS and under Help elsewhere.
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about = MenuItem::with_id(
        app,
        ABOUT_MENU_ID,
        format!("About {}", app.package_info().name),
        true,
        None::<&str>,
    )?;
    let preferences = MenuItem::with_id(
        app,
        PREFERENCES_MENU_ID,
        "Preferences…",
        true,
        Some("CmdOrCtrl+,"),
    )?;

    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        app,
        app.package_info().name.clone(),
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    // GTK doesn't support the predefined Close Window / Quit items.
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &preferences,
            #[cfg(not(target_os = "linux"))]
            &PredefinedMenuItem::separator(app)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::close_window(app, None)?,
            #[cfg(windows)]
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    #[cfg(target_os = "macos")]
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    #[cfg(not(target_os = "macos"))]
    let help_menu = Submenu::with_items(app, "Help", true, &[&about])?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &app_menu,
            &file_menu,
            &edit_menu,
            #[cfg(target_os = "macos")]
            &view_menu,
            &window_menu,
            #[cfg(not(target_os = "macos"))]
            &help_menu,
        ],
    )
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
            // The webview ignores these while Baby Lock is on.
            let ui_event = match event.id().as_ref() {
                PREFERENCES_MENU_ID => OPEN_PREFERENCES_EVENT,
                ABOUT_MENU_ID => OPEN_ABOUT_EVENT,
                _ => return,
            };
            let _ = app.emit_to("main", ui_event, ());
        })
        .invoke_handler(tauri::generate_handler![
            set_baby_lock,
            set_fullscreen,
            macos_keyboard_lock_status,
            open_accessibility_settings,
            app_build_info
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
