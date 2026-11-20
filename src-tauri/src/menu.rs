//! The native menu bar. Go menu items are forwarded to the web app as `menu` events.

use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

/// Go menu item ids are `go:<action>`; the action string is what the web app receives.
const GO_PREFIX: &str = "go:";

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let launch_at_login = CheckMenuItemBuilder::with_id("launch-at-login", "Launch at Login")
        .checked(app.autolaunch().is_enabled().unwrap_or(false))
        .build(app)?;

    let app_menu = SubmenuBuilder::new(app, "DayDitto")
        .about(None)
        .item(&MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(app)?)
        .separator()
        .item(&launch_at_login)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    // Predefined items: they are what make ⌘C / ⌘V / ⌘Z work in text fields.
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&MenuItemBuilder::with_id("reload", "Reload").accelerator("CmdOrCtrl+R").build(app)?)
        .separator()
        .fullscreen()
        .build()?;
    let go = SubmenuBuilder::new(app, "Go")
        .item(&go_item(app, "today", "Today", "CmdOrCtrl+T")?)
        // ⇧⌘W: ⌘W stays Close Window.
        .item(&go_item(app, "week", "Week", "CmdOrCtrl+Shift+W")?)
        .item(&go_item(app, "focus", "Focus Mode", "CmdOrCtrl+Shift+F")?)
        .separator()
        .item(&go_item(app, "quick-dump", "Quick Dump", "CmdOrCtrl+N")?)
        .item(&go_item(app, "settings", "Settings…", "CmdOrCtrl+,")?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&MenuItemBuilder::with_id("website", "DayDitto Website").build(app)?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_menu)
        .item(&edit)
        .item(&view)
        .item(&go)
        .item(&window)
        .item(&help)
        .build()?;
    app.set_menu(menu)?;
    app.on_menu_event(move |app, event| handle(app, event.id().0.as_str(), &launch_at_login));
    Ok(())
}

fn go_item(app: &AppHandle, action: &str, text: &str, accelerator: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItemBuilder::with_id(format!("{GO_PREFIX}{action}"), text)
        .accelerator(accelerator)
        .build(app)
}

fn handle(app: &AppHandle, id: &str, launch_at_login: &CheckMenuItem<Wry>) {
    if let Some(action) = id.strip_prefix(GO_PREFIX) {
        crate::show_main(app);
        if let Err(err) = app.emit_to(crate::MAIN, "menu", action) {
            eprintln!("[dayditto] couldn't send the menu action: {err}");
        }
        return;
    }
    match id {
        "check-updates" => crate::updater::check_now(app),
        "launch-at-login" => toggle_launch_at_login(app, launch_at_login),
        "reload" => {
            if let Some(window) = app.get_webview_window(crate::MAIN) {
                let _ = window.reload();
            }
        }
        "website" => {
            if let Err(err) = app.opener().open_url(crate::APP_URL, None::<&str>) {
                eprintln!("[dayditto] couldn't open the website: {err}");
            }
        }
        _ => {}
    }
}

fn toggle_launch_at_login(app: &AppHandle, item: &CheckMenuItem<Wry>) {
    let autostart = app.autolaunch();
    let enabled = autostart.is_enabled().unwrap_or(false);
    let result = if enabled { autostart.disable() } else { autostart.enable() };
    if let Err(err) = result {
        eprintln!("[dayditto] couldn't change Launch at Login: {err}");
    }
    // The click already flipped the check mark; show the real state.
    let _ = item.set_checked(autostart.is_enabled().unwrap_or(false));
}
