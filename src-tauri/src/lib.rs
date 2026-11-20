//! DayDitto for Mac: a thin native shell around the live web app.

mod commands;
mod deep_link;
mod menu;
mod navigation;
mod quick;
mod shortcut;
mod updater;

use std::error::Error;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, RunEvent, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_window_state::StateFlags;

/// The web app this shell wraps. Only this origin gets IPC (capabilities/default.json).
pub const APP_HOST: &str = "dayditto.prototypolab.com";
pub const APP_URL: &str = "https://dayditto.prototypolab.com";
pub const MAIN: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Registered first: a second launch (e.g. by a deep link) hands its arguments to this instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            deep_link::handle_args(app, &argv);
            show_main(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&[quick::LABEL])
                // A main window hidden at quit (closed, then ⌘Q) must still open on the next launch.
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(shortcut::plugin())
        .invoke_handler(tauri::generate_handler![
            commands::set_badge,
            commands::hide_quick,
            commands::open_external
        ])
        .on_window_event(|window, event| {
            // Both windows live for the whole session: closing one hides it (⌘Q quits the app).
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let handle = app.handle();
            build_window(handle, MAIN)?;
            quick::create(handle)?;
            menu::install(handle)?;
            shortcut::register(handle);
            deep_link::listen(handle);
            updater::schedule(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building DayDitto");

    app.run(|handle, event| {
        // Dock icon clicked while the main window is hidden.
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            show_main(handle);
        }
    });
}

/// Brings the main window to the front, wherever it was.
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Creates a window declared in tauri.conf.json (`"create": false`) with the shell's
/// user agent and navigation policy.
fn build_window(app: &AppHandle, label: &str) -> Result<WebviewWindow, Box<dyn Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == label)
        .ok_or(format!("window `{label}` is missing from tauri.conf.json"))?;
    let navigation_app = app.clone();
    let new_window_app = app.clone();
    let window = WebviewWindowBuilder::from_config(app, config)?
        .user_agent(&user_agent(app))
        .on_navigation(move |url| navigation::allow(&navigation_app, url))
        // target=_blank links and window.open: the system browser, never a new webview.
        .on_new_window(move |url, _features| {
            navigation::open_outside(&new_window_app, &url);
            NewWindowResponse::Deny
        })
        .build()?;
    Ok(window)
}

/// Safari's user agent plus `DayDitto-Desktop/<version>`, which the web app checks for.
fn user_agent(app: &AppHandle) -> String {
    format!(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) \
         Version/18.0 Safari/605.1.15 DayDitto-Desktop/{}",
        app.package_info().version
    )
}
