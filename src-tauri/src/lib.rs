use std::process;
use std::time::Duration;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    ActivationPolicy, AppHandle, Emitter, Listener, Manager, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tokio::select;
use tpower::{
    ffi::smc::{SMCConnection, SMCReadSensor},
    provider::get_mac_ioreg,
};

#[tauri::command]
fn open_app(app: AppHandle) {
    let main = app.get_webview_window("main").unwrap();
    main.show().unwrap();
    main.set_focus().unwrap();
    app.set_activation_policy(ActivationPolicy::Regular)
        .unwrap();
    app.get_webview_window("popover").unwrap().hide().unwrap();
}

#[tauri::command]
fn is_main_window_hidden(app: AppHandle) -> bool {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(visible) => !visible,
            Err(_) => true,
        }
    } else {
        false
    }
}

fn start_sender(app: AppHandle) -> tokio::task::JoinHandle<()> {
    let mut smc_conn = SMCConnection::new("AppleSMC").unwrap();
    let mut timer = tokio::time::interval(Duration::from_millis(2000));

    tokio::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    let data = smc_conn.read_sensor();
                    let ioreg = get_mac_ioreg().unwrap();
                    app.emit("power-updated", format!("{:.1} w", data.system_total)).unwrap();
                    app.emit("power-data", (data, ioreg)).unwrap();
                }
            }
        }
    })
}

trait WebviewWindowExt {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![open_app, is_main_window_hidden])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap();
                // window.position_

                window
                    .app_handle()
                    .set_activation_policy(ActivationPolicy::Accessory)
                    .unwrap();
            }
        })
        .setup(|app| {
            let _handle = start_sender(app.handle().clone());

            let show = MenuItemBuilder::new("Show Window").build(app)?;
            let quit = MenuItemBuilder::new("Quit").build(app)?;

            let menu = MenuBuilder::new(app.handle())
                .item(&show)
                .separator()
                .item(&quit)
                .build()
                .unwrap();

            let tray_icon = TrayIconBuilder::new()
                .title("0 w")
                .menu_on_left_click(false)
                .menu(&menu)
                .build(app)
                .unwrap();

            tray_icon.on_menu_event(move |tray_handle, event| match event.id() {
                val if val == show.id() => {
                    let (window, is_new) = tray_handle
                        .app_handle()
                        .get_or_create_window("main")
                        .unwrap();

                    if !window.is_visible().unwrap() {
                        window.show().unwrap();
                        window.set_focus().unwrap();

                        let _ = tray_handle
                            .app_handle()
                            .set_activation_policy(ActivationPolicy::Regular);
                    }
                }
                val if val == quit.id() => {
                    tray_handle.app_handle().cleanup_before_exit();
                    process::exit(0);
                }
                _ => {}
            });

            tray_icon.on_tray_icon_event(move |tray_handle, event| {
                tauri_plugin_positioner::on_tray_event(tray_handle.app_handle(), &event);
                if let TrayIconEvent::Click {
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let (window, is_new) = tray_handle
                        .app_handle()
                        .get_or_create_window("popover")
                        .unwrap();

                    if window.is_visible().unwrap() && !is_new {
                        // window.hide().unwrap();
                        tray_handle
                            .app_handle()
                            .emit("hide-popover", ());
                    } else {
                        window.move_window(Position::TrayLeft).unwrap();
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
            });

            app.listen("power-updated", move |event| {
                tray_icon
                    .set_title(Some(event.payload().trim_matches('"')))
                    .unwrap();
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app, event| if let tauri::RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    });
}
