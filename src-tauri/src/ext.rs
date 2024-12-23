use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};


pub trait WebviewWindowExt {
    fn get_or_create_window(&self, label: &str) -> tauri::Result<(WebviewWindow, bool)>;
}

impl WebviewWindowExt for AppHandle {
    fn get_or_create_window(&self, label: &str) -> tauri::Result<(WebviewWindow, bool)> {
        if let Some(window) = self.get_webview_window(label) {
            Ok((window, false))
        } else {
            WebviewWindowBuilder::from_config(
                self,
                self.config()
                    .app
                    .windows
                    .iter()
                    .find(|w| w.label == label)
                    .unwrap(),
            )
            .unwrap()
            .build()
            .map(|window| (window, true))
        }
    }
}