//! `dayditto://auth/callback?code=…` from the system browser finishes Google sign-in:
//! the main window opens https://dayditto.prototypolab.com/auth/callback?code=…, where the
//! server exchanges the code with the PKCE verifier cookie this webview set.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, Url};
use tauri_plugin_deep_link::DeepLinkExt;

/// The last link handled. One link can arrive twice (cold start and event), and a code works once.
static LAST: Mutex<Option<String>> = Mutex::new(None);

pub fn listen(app: &AppHandle) {
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            open(&handle, &url);
        }
    });
    // Launched by a link while not running.
    match app.deep_link().get_current() {
        Ok(Some(urls)) => urls.iter().for_each(|url| open(app, url)),
        Ok(None) => {}
        Err(err) => eprintln!("[dayditto] couldn't read the launch link: {err}"),
    }
}

/// A second launch passes the link as an argument (single-instance).
pub fn handle_args(app: &AppHandle, argv: &[String]) {
    for arg in argv.iter().skip(1) {
        if let Ok(url) = Url::parse(arg) {
            open(app, &url);
        }
    }
}

fn open(app: &AppHandle, url: &Url) {
    let Some(target) = auth_callback(url) else {
        return;
    };
    {
        let mut last = LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.as_deref() == Some(url.as_str()) {
            return;
        }
        *last = Some(url.as_str().to_owned());
    }
    let Some(window) = app.get_webview_window(crate::MAIN) else {
        return;
    };
    if let Err(err) = window.navigate(target) {
        eprintln!("[dayditto] couldn't open the sign-in callback: {err}");
    }
    crate::show_main(app);
}

/// The web callback for `dayditto://auth/callback?…` (same query); None for any other link.
fn auth_callback(url: &Url) -> Option<Url> {
    // A non-special scheme: `auth` parses as the host, `/callback` as the path.
    if url.scheme() != "dayditto" || url.host_str() != Some("auth") || url.path() != "/callback" {
        return None;
    }
    let mut target = Url::parse(crate::APP_URL).ok()?.join("/auth/callback").ok()?;
    target.set_query(url.query());
    Some(target)
}
