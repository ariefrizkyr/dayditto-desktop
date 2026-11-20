fn main() {
    // Listing the app's commands makes tauri-build generate `allow-<command>` permissions
    // (e.g. `allow-set-badge`) and stops them from being callable without one: only the
    // capability in capabilities/default.json grants them, to the DayDitto origin.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["set_badge", "hide_quick", "quick_saved", "open_external"]),
    ))
    .expect("failed to run tauri-build");
}
