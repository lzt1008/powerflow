use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

pub trait WebviewWindowExt<R: Runtime>: Manager<R> + Sized {
    fn get_or_create_window(&self, label: &str) -> tauri::Result<(WebviewWindow<R>, bool)> {
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

    fn main_window(&self) -> Option<WebviewWindow<R>> {
        self.get_webview_window("main")
    }

    fn popover_window(&self) -> Option<WebviewWindow<R>> {
        self.get_webview_window("popover")
    }

    #[allow(dead_code)]
    fn settings_window(&self) -> Option<WebviewWindow<R>> {
        self.get_or_create_window("settings").ok().map(|v| v.0)
    }
}

impl<R: Runtime, T: Manager<R> + Sized> WebviewWindowExt<R> for T {}
