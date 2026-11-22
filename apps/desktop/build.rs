fn main() {
    for path in ["ui", "icons", "tauri.conf.json", "capabilities"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "app_info",
            "set_window_theme",
            "probe_report",
            "system_report",
            "selected_report",
            "refresh_catalog",
            "list_versions",
            "check_updates",
            "check_update",
            "prepare_plan",
            "prepare_batch",
            "start_request",
            "start_batch",
            "operations",
            "operation",
            "latest_operation",
            "recheck_operation",
        ]));
    tauri_build::try_build(attributes).expect("failed to prepare the Tenon desktop application");
}
