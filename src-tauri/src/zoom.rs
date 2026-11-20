//! Page zoom for the main window (WKWebView has none of its own). The quick capture panel stays at 100%.
//!
//! The level is kept in `zoom.json` in the app config dir (`{"level":1.0}`), written on every
//! change and read at startup. WKWebView may drop the zoom on navigation (the splash hands over
//! to the remote site), so it is applied again after every page load of the main window.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde_json::{json, Value};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, WebviewWindow};

const FILE: &str = "zoom.json";
const DEFAULT: f64 = 1.0;
const MIN: f64 = 0.5;
const MAX: f64 = 2.0;
const STEP: f64 = 0.1;

/// The current level, shared between the menu and the page load hook.
struct Level(Mutex<f64>);

/// Loads the saved level. Call before the main window is created.
pub fn init(app: &AppHandle) {
    app.manage(Level(Mutex::new(load(app))));
}

pub fn zoom_in(app: &AppHandle) {
    change(app, |level| level + STEP);
}

pub fn zoom_out(app: &AppHandle) {
    change(app, |level| level - STEP);
}

pub fn reset(app: &AppHandle) {
    change(app, |_| DEFAULT);
}

/// `on_page_load` hook: re-applies the level once the main window has loaded a page.
pub fn on_page_load(window: WebviewWindow, payload: PageLoadPayload<'_>) {
    if window.label() == crate::MAIN && payload.event() == PageLoadEvent::Finished {
        apply(window.app_handle());
    }
}

fn change(app: &AppHandle, step: impl Fn(f64) -> f64) {
    let state = app.state::<Level>();
    let mut level = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    *level = snap(step(*level));
    apply(app);
    if let Err(err) = save(app, *level) {
        eprintln!("[dayditto] couldn't save the zoom level: {err}");
    }
}

fn apply(app: &AppHandle) {
    let Some(window) = app.get_webview_window(crate::MAIN) else {
        return;
    };
    let level = *app.state::<Level>().0.lock().unwrap_or_else(PoisonError::into_inner);
    if let Err(err) = window.set_zoom(level) {
        eprintln!("[dayditto] couldn't set the zoom level: {err}");
    }
}

/// Rounds to a whole step (no drift like 1.2000000000000002) and keeps it in range.
fn snap(level: f64) -> f64 {
    (((level / STEP).round() * STEP * 100.0).round() / 100.0).clamp(MIN, MAX)
}

fn path(app: &AppHandle) -> Result<PathBuf, Box<dyn Error>> {
    Ok(app.path().app_config_dir()?.join(FILE))
}

fn load(app: &AppHandle) -> f64 {
    let path = match path(app) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("[dayditto] no config dir for the zoom level: {err}");
            return DEFAULT;
        }
    };
    if !path.exists() {
        return DEFAULT;
    }
    read(&path).unwrap_or_else(|err| {
        eprintln!("[dayditto] ignoring {}: {err}", path.display());
        DEFAULT
    })
}

fn read(path: &Path) -> Result<f64, Box<dyn Error>> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let level = value.get("level").and_then(Value::as_f64).ok_or("no numeric `level`")?;
    Ok(snap(level))
}

fn save(app: &AppHandle, level: f64) -> Result<(), Box<dyn Error>> {
    let path = path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, json!({ "level": level }).to_string())?;
    Ok(())
}
