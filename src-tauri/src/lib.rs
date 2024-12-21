use std::sync::Mutex;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{ActivationPolicy, AppHandle, Listener, Manager, State};
use tauri_plugin_positioner::{Position, WindowExt};
use tpower::{
    de::IORegistry,
    ffi::smc::{SMCConnection, SMCPowerData, SMCReadSensor},
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
    let main = app.get_webview_window("main").unwrap();
    !main.is_visible().unwrap()
}

#[tauri::command]
fn get_power(state: State<Mutex<SMCConnection>>) -> SMCPowerData {
    let mut state = state.lock().unwrap();
    state.read_sensor()
}

#[tauri::command]
fn get_ioreg() -> IORegistry {
    get_mac_ioreg().unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let smc_conn = Mutex::new(SMCConnection::new("AppleSMC").unwrap());

    tauri::Builder::default()
        .manage(smc_conn)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap();

                window
                    .app_handle()
                    .set_activation_policy(ActivationPolicy::Accessory)
                    .unwrap();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            get_power,
            get_ioreg,
            open_app,
            is_main_window_hidden
        ])
        .setup(|app| {
            let show = MenuItemBuilder::new("Show Window").build(app)?;

            let quit = MenuItemBuilder::new("Quit").build(app)?;

            let menu = MenuBuilder::new(app.handle())
                .separator()
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

            tray_icon.on_menu_event(move |tray_handle, event| {
                if event.id() == show.id() {
                    let window = tray_handle.app_handle().get_webview_window("main").unwrap();

                    if !window.is_visible().unwrap() {
                        window.show().unwrap();
                        window.set_focus().unwrap();

                        let _ = tray_handle
                            .app_handle()
                            .set_activation_policy(ActivationPolicy::Regular);
                    }
                }
            });

            tray_icon.on_tray_icon_event(move |tray_handle, event| {
                tauri_plugin_positioner::on_tray_event(tray_handle.app_handle(), &event);
                if let TrayIconEvent::Click {
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let _ = tray_handle
                        .app_handle()
                        .set_activation_policy(ActivationPolicy::Regular);

                    // icon.app_handle().show().unwrap();

                    let window = tray_handle
                        .app_handle()
                        .get_webview_window("popover")
                        .unwrap();

                    window.move_window(Position::TrayLeft).unwrap();

                    if window.is_visible().unwrap() {
                        window.hide().unwrap();

                        let _ = tray_handle
                            .app_handle()
                            .set_activation_policy(ActivationPolicy::Accessory);
                    } else {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
            });

            app.listen("power-updated", move |event| {
                tray_icon.set_title(
                    // this is a JSON String
                    Some(event.payload().trim_matches('"')
                )).unwrap();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
