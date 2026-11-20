//! The quick capture window: a small transparent window showing /quick, toggled by the global shortcut.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

pub const LABEL: &str = "quick";

/// DayDitto was in the background when the window opened, so closing it hands focus back.
static RETURN_FOCUS: AtomicBool = AtomicBool::new(false);

/// Created hidden at startup so the shortcut shows it instantly.
pub fn create(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = crate::build_window(app, LABEL)?;
    let handle = window.clone();
    // Clicking anywhere else dismisses it; focus has already moved on, so don't hand it back.
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            RETURN_FOCUS.store(false, Ordering::Relaxed);
            let _ = handle.hide();
        }
    });
    Ok(())
}

/// The global shortcut: show and focus the window, or hide it if it's open.
pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    let main_focused = app
        .get_webview_window(crate::MAIN)
        .and_then(|main| main.is_focused().ok())
        .unwrap_or(false);
    RETURN_FOCUS.store(!main_focused, Ordering::Relaxed);
    place(app, &window);
    let _ = window.show();
    let _ = window.set_focus();
}

/// Esc, after saving, or the shortcut again.
pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let _ = window.hide();
    // Back to the app the user was in, instead of leaving DayDitto in front.
    #[cfg(target_os = "macos")]
    if RETURN_FOCUS.swap(false, Ordering::Relaxed) {
        let _ = app.hide();
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
