use std::process;
use std::time::Duration;

use cocoa::base::id;
use data::{start_sender, SenderMessage};
use ext::WebviewWindowExt;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    async_runtime, ActivationPolicy, AppHandle, Emitter, Listener, Manager, RunEvent, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tokio::sync::mpsc;
use util::set_window_controls_pos;

mod data;
mod ext;
mod util;

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
    app.get_webview_window("main")
        .map(|w| w.is_visible().map(|v| !v).unwrap_or(true))
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_pinia::init())
        .invoke_handler(tauri::generate_handler![open_app, is_main_window_hidden])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                window.hide().unwrap();
                window
                    .app_handle()
                    .set_activation_policy(ActivationPolicy::Accessory)
                    .unwrap();
            }
        })
        .setup(|app| {
            set_window_controls_pos(
                app.get_webview_window("main").unwrap().ns_window().unwrap() as id,
                22.,
                24.,
            );

            let (sender_tx, rx) = mpsc::channel(10);
            let _handle = start_sender(app.handle().clone(), rx);

            let tx = sender_tx.clone();
            app.listen("window:load", move |_| {
                let tx = tx.clone();
                async_runtime::spawn(async move {
                    tx.send(SenderMessage::ImmediateSend).await.unwrap();
                });
            });

            app.listen("preferences:change-interval", move |event| {
                let tx = sender_tx.clone();
                let interval: u64 = serde_json::from_str(event.payload()).unwrap();
                async_runtime::spawn(async move {
                    tx.send(SenderMessage::ChangeInterval(Duration::from_millis(
                        interval,
                    )))
                    .await
                    .unwrap();
                });
            });

            // --------- Tray Icon ---------

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
                    let (window, _) = tray_handle
                        .app_handle()
                        .get_or_create_window("main")
                        .unwrap();

                    if !window.is_visible().unwrap() {
                        window.show().unwrap();
                        window.set_focus().unwrap();

                        tray_handle
                            .app_handle()
                            .set_activation_policy(ActivationPolicy::Regular)
                            .unwrap();
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
                        // let js side handle this, so we can have fade animation
                        tray_handle.app_handle().emit("hide-popover", ()).unwrap();
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

    // prevent app from exiting when all windows are closed
    app.run(|_app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();
        }
    });
}
