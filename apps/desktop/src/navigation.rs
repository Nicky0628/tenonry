use tauri::WebviewWindow;
use tenon::{Error, Result};
use url::Url;

pub(crate) fn local_app_url(url: &Url) -> bool {
    url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && match url.scheme() {
            "tauri" => url.host_str() == Some("localhost"),
            "http" | "https" => url.host_str() == Some("tauri.localhost"),
            _ => false,
        }
}

pub(crate) fn require_main_window(window: &WebviewWindow) -> Result<()> {
    if window.label() != "main" || !window.url().is_ok_and(|url| local_app_url(&url)) {
        return Err(Error::authorization("当前页面无权访问本地环境接口。"));
    }
    Ok(())
}
