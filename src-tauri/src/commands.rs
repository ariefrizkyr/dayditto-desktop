//! The only IPC the web app may use (granted to its origin in capabilities/default.json).

use tauri::{AppHandle, Emitter, Manager, Url};
use tauri_plugin_opener::OpenerExt;

/// Dock badge: today's open items; 0 clears it.
#[tauri::command]
pub fn set_badge(app: AppHandle, count: u32) {
    let Some(window) = app.get_webview_window(crate::MAIN) else {
        return;
    };
    let badge = (count > 0).then(|| i64::from(count));
    if let Err(err) = window.set_badge_count(badge) {
        eprintln!("[dayditto] couldn't set the badge: {err}");
    }
}

/// Hides the quick capture window (Esc, or after saving).
#[tauri::command]
pub fn hide_quick(app: AppHandle) {
    crate::quick::hide(&app);
}

/// Quick capture saved a task: the main window reloads its planner (hidden, its webview still runs).
#[tauri::command]
pub fn quick_saved(app: AppHandle) {
    if let Err(err) = app.emit_to(crate::MAIN, "tasks-changed", ()) {
        eprintln!("[dayditto] couldn't tell the main window about the new task: {err}");
    }
}

/// Opens an https URL in the system browser (used for Google sign-in).
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let url = Url::parse(&url).map_err(|err| err.to_string())?;
    if url.scheme() != "https" {
        return Err("only https URLs can be opened".into());
    }
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|err| err.to_string())
}
