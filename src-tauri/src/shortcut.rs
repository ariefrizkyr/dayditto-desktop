//! The global quick capture shortcut, ⌃⌥D (fixed in v1).

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Wry};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn quick_capture() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD)
}

/// The plugin; its only shortcut toggles the quick capture window.
pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::quick::toggle(app);
            }
        })
        .build()
}

/// Another app may already own the shortcut: log it and run without one.
pub fn register(app: &AppHandle) {
    if let Err(err) = app.global_shortcut().register(quick_capture()) {
        eprintln!("[dayditto] couldn't register the quick capture shortcut: {err}");
    }
}
