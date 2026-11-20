//! Updates from GitHub Releases (via https://dayditto.prototypolab.com/desktop/latest.json):
//! a quiet check ~10 s after launch and every 6 hours, plus "Check for Updates…".

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// A check, an open dialog or an install is in progress.
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn schedule(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK);
        loop {
            tauri::async_runtime::block_on(check(&app, false));
            thread::sleep(INTERVAL);
        }
    });
}

/// From the menu: like the quiet check, but also says when there is nothing new.
pub fn check_now(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { check(&app, true).await });
}

async fn check(app: &AppHandle, from_menu: bool) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    match find(app).await {
        // BUSY stays set until the dialog is answered.
        Ok(Some(update)) => offer(app, update),
        Ok(None) => {
            BUSY.store(false, Ordering::SeqCst);
            if from_menu {
                tell(app, "You’re up to date.");
            }
        }
        Err(err) => {
            BUSY.store(false, Ordering::SeqCst);
            eprintln!("[dayditto] update check failed: {err}");
            if from_menu {
                tell(app, "Couldn’t check for updates. Try again later.");
            }
        }
    }
}

async fn find(app: &AppHandle) -> tauri_plugin_updater::Result<Option<Update>> {
    app.updater()?.check().await
}

fn offer(app: &AppHandle, update: Update) {
    let handle = app.clone();
    app.dialog()
        .message(format!("DayDitto {} is available. Install and restart?", update.version))
        .title("Update available")
        .buttons(MessageDialogButtons::OkCancelCustom("Install and Restart".into(), "Later".into()))
        .show(move |install| {
            if !install {
                BUSY.store(false, Ordering::SeqCst);
                return;
            }
            tauri::async_runtime::spawn(async move {
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => handle.restart(),
                    Err(err) => {
                        BUSY.store(false, Ordering::SeqCst);
                        eprintln!("[dayditto] update install failed: {err}");
                        tell(&handle, "The update couldn’t be installed. Try again later.");
                    }
                }
            });
        });
}

fn tell(app: &AppHandle, message: &str) {
    app.dialog()
        .message(message)
        .title("DayDitto")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}
