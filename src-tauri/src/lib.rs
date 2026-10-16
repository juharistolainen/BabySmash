use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_prevent_default::Flags;

#[tauri::command]
fn set_baby_lock(app: AppHandle, locked: bool) -> Result<(), String> {
    let window = main_window(&app)?;
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

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Block webview/browser shortcuts (Ctrl+R, F5, Ctrl+W, etc.) that babies
    // can hit accidentally. OS-reserved chords (Alt+Tab, Cmd+Q, …) still need
    // platform-level handling — see README.
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
        .invoke_handler(tauri::generate_handler![set_baby_lock, set_fullscreen])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
