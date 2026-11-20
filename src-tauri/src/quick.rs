//! The quick capture window: a small transparent window showing /quick, toggled by the global shortcut.
//!
//! On macOS it is a non-activating floating panel (tauri-nspanel): it takes the keyboard without
//! activating DayDitto, so the main window stays exactly as it was (hidden, or behind other apps)
//! and closing the panel leaves the user in the app they were in.

use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

pub const LABEL: &str = "quick";

/// Sent to the page each time the window is shown, so it focuses its title field.
const SHOWN_EVENT: &str = "quick-shown";

/// A Dock reopen this soon after the window was toggled came from the shortcut, not the Dock.
const REOPEN_GRACE_MS: u64 = 1_500;

/// When the window was last shown or hidden (ms since the epoch).
static LAST_TOGGLE: AtomicU64 = AtomicU64::new(0);

/// Created hidden at startup so the shortcut shows it instantly.
pub fn create(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = crate::build_window(app, LABEL)?;
    // Runs in `setup`, on the main thread, as AppKit requires.
    #[cfg(target_os = "macos")]
    if let Err(err) = panel::convert(&window) {
        eprintln!("[dayditto] couldn't make the quick capture window a panel: {err}");
    }
    let handle = app.clone();
    // Clicking anywhere else dismisses it.
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            hide(&handle);
        }
    });
    Ok(())
}

/// The global shortcut: show the window and put the cursor in it, or hide it if it's open.
pub fn toggle(app: &AppHandle) {
    let handle = app.clone();
    // The shortcut handler may run off the main thread; panels must be driven from it.
    if let Err(err) = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(LABEL) else {
            return;
        };
        if window.is_visible().unwrap_or(false) {
            hide(&handle);
        } else {
            show(&handle, &window);
        }
    }) {
        eprintln!("[dayditto] couldn't toggle quick capture: {err}");
    }
}

/// Esc, after saving, a click elsewhere, or the shortcut again. Never touches the main window.
pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    if let Err(err) = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(LABEL) {
            mark_toggled();
            let _ = window.hide();
        }
    }) {
        eprintln!("[dayditto] couldn't hide quick capture: {err}");
    }
}

/// Whether the window was shown or hidden in the last moment (see `RunEvent::Reopen` in lib.rs).
pub fn recently_toggled() -> bool {
    now_ms().saturating_sub(LAST_TOGGLE.load(Ordering::Relaxed)) < REOPEN_GRACE_MS
}

fn mark_toggled() {
    LAST_TOGGLE.store(now_ms(), Ordering::Relaxed);
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// Main thread only.
fn show(app: &AppHandle, window: &WebviewWindow) {
    mark_toggled();
    place(app, window);
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        // `set_focus` would activate the whole app (and raise the main window): the panel becomes
        // key on its own, and only the webview inside it gets the keyboard.
        match app.get_webview_panel(LABEL) {
            Ok(panel) => panel.show_and_make_key(),
            Err(_) => {
                let _ = window.show();
            }
        }
        let webview: &tauri::webview::Webview = window.as_ref();
        let _ = webview.set_focus();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Err(err) = app.emit_to(LABEL, SHOWN_EVENT, ()) {
        eprintln!("[dayditto] couldn't tell quick capture it's shown: {err}");
    }
}

/// Centered horizontally in the upper third of the screen with the pointer.
fn place(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        let _ = window.center();
        return;
    };
    let area = monitor.size();
    let origin = monitor.position();
    let x = origin.x + (area.width as i32 - size.width as i32) / 2;
    let y = origin.y + area.height as i32 / 3 - size.height as i32 / 2;
    let _ = window.set_position(PhysicalPosition::new(x, y.max(origin.y)));
}

#[cfg(target_os = "macos")]
mod panel {
    use tauri::WebviewWindow;
    use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

    tauri_panel! {
        panel!(QuickPanel {
            config: {
                // Key (it takes typing) but never main: the main window keeps that role.
                can_become_key_window: true,
                can_become_main_window: false,
                is_floating_panel: true
            }
        })
    }

    /// Turns the existing window into a panel in place: same webview, so the user agent and
    /// navigation policy from `build_window` stay. No event handler is set, so Tauri's window
    /// delegate (and with it `WindowEvent::Focused`) keeps working.
    pub fn convert(window: &WebviewWindow) -> tauri::Result<()> {
        let panel = window.to_panel::<QuickPanel>()?;
        panel.set_level(PanelLevel::Floating.value());
        // Showing it and typing into it must not activate DayDitto.
        if panel
            .add_style_mask(StyleMask::empty().nonactivating_panel().into())
            .is_err()
        {
            eprintln!("[dayditto] couldn't make the quick capture panel non-activating");
        }
        // Every Space, including other apps' full-screen ones.
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .into(),
        );
        // DayDitto usually isn't the active app while it's open; it hides itself on blur.
        panel.set_hides_on_deactivate(false);
        Ok(())
    }
}
