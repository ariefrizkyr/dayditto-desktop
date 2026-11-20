use tauri::{AppHandle, Url};
use tauri_plugin_opener::OpenerExt;

/// Navigation policy: DayDitto pages and the bundled splash load in the webview; any
/// other web or mail link opens in the default app and the navigation is cancelled.
pub fn allow(app: &AppHandle, url: &Url) -> bool {
    if is_internal(url) {
        return true;
    }
    open_outside(app, url);
    false
}

/// Opens a web or mail link in the user's default app; anything else is dropped.
pub fn open_outside(app: &AppHandle, url: &Url) {
    if !matches!(url.scheme(), "https" | "http" | "mailto") {
        return;
    }
    if let Err(err) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("[dayditto] couldn't open {url}: {err}");
    }
}

fn is_internal(url: &Url) -> bool {
    match url.scheme() {
        "https" => url.host_str() == Some(crate::APP_HOST),
        // The bundled splash (tauri://localhost) and in-page blank/blob documents.
        "tauri" | "about" | "blob" => true,
        // `tauri dev` serves the splash from localhost.
        "http" => cfg!(debug_assertions) && url.host_str() == Some("localhost"),
        _ => false,
    }
}
