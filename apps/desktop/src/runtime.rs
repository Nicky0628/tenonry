use crate::{commands, navigation::local_app_url};
use tauri::{Emitter, Manager, WebviewWindowBuilder};
use tenon::{OperationState, Tenon};

pub(crate) struct AppState {
    pub(crate) tenon: Tenon,
    pub(crate) environment_issue: Option<String>,
}

pub(crate) fn run(environment_issue: Option<String>) {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::set_window_theme,
            commands::probe_report,
            commands::system_report,
            commands::selected_report,
            commands::refresh_catalog,
            commands::list_versions,
            commands::check_updates,
            commands::check_update,
            commands::prepare_plan,
            commands::prepare_batch,
            commands::start_request,
            commands::start_batch,
            commands::operations,
            commands::operation,
            commands::latest_operation,
            commands::recheck_operation,
        ])
        .setup(move |app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| std::io::Error::other(format!("无法确定数据目录：{error}")))?;
            let tenon = Tenon::new(data_dir).map_err(std::io::Error::other)?;
            app.manage(AppState {
                tenon,
                environment_issue,
            });
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .ok_or_else(|| std::io::Error::other("找不到主窗口配置。"))?;
            WebviewWindowBuilder::from_config(app, config)?
                .on_navigation(local_app_url)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && let Ok(Some(operation)) = window.state::<AppState>().tenon.latest_operation()
                && operation.state == OperationState::Running
            {
                api.prevent_close();
                let _ = window.emit("tenon://close-blocked", operation.id);
            }
        })
        // Tauri exits cleanly when the last window closes; no tray or daemon.
        .run(tauri::generate_context!());

    if let Err(error) = result {
        show_startup_error(&format!("Tenon 无法启动：{error}"));
        std::process::exit(1);
    }
}

fn show_startup_error(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(
                window: *mut std::ffi::c_void,
                text: *const u16,
                caption: *const u16,
                kind: u32,
            ) -> i32;
        }
        let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let caption: Vec<u16> = "Tenon".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: Both UTF-16 buffers are NUL-terminated and live across the synchronous call.
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x10);
        }
    }
}
