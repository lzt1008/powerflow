use std::collections::HashSet;

use database::{setup_database, ChargingHistory};
use device::{setup_device_listener, start_device_sender, DevicePowerTickEvent, DeviceState};
use event::{
    DeviceEvent, HidePopoverEvent, PowerUpdatedEvent, PreferenceEvent, Theme, WindowLoadedEvent,
};
use ext::WebviewWindowExt;
use history::{setup_history_recorder, ChargingHistoryDetail, HistoryRecordedEvent};
use local::{setup_sender_with_events, PowerTickEvent};
use menu::setup_menu;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight, NSWindow,
};
#[cfg(debug_assertions)]
use specta_typescript::{BigIntExportBehavior, Typescript};
use sqlx::{Pool, Sqlite};
use tauri::{
    async_runtime, ActivationPolicy, AppHandle, Manager, RunEvent, State, Window, WindowEvent,
};
use tauri_specta::{collect_commands, collect_events, Event};
use tpower::ffi::InterfaceType;
use tray_icon::setup_tray_icon;
use util::setup_traffic_light_positioner;

mod database;
pub mod device;
mod event;
mod ext;
mod history;
mod local;
mod menu;
mod tray_icon;
mod util;

#[tauri::command]
#[specta::specta]
fn open_app(app: AppHandle) {
    let main = app.main_window().unwrap();
    main.show().unwrap();
    main.set_focus().unwrap();
    app.set_activation_policy(ActivationPolicy::Regular)
        .unwrap();
    app.popover_window().unwrap().hide().unwrap();
}

#[tauri::command]
#[specta::specta]
fn open_settings(app: AppHandle) {
    let settings = app.settings_window().unwrap();
    settings.show().unwrap();
    settings.set_focus().unwrap();
}

#[tauri::command]
#[specta::specta]
fn is_main_window_hidden(app: AppHandle) -> bool {
    app.main_window()
        .map(|w| w.is_visible().map(|v| !v).unwrap_or(true))
        .unwrap_or(false)
}

#[tauri::command]
#[specta::specta]
fn get_device_name(
    id: String,
    state: State<DeviceState>,
) -> Option<(String, HashSet<InterfaceType>)> {
    let state = state.read().unwrap();
    let data = state.get(&id);
    data.cloned()
}

#[tauri::command]
#[specta::specta]
fn get_mac_name() -> Option<String> {
    tpower::util::get_mac_name()
}

#[tauri::command]
#[specta::specta]
fn switch_theme(theme: Theme, app: AppHandle) {
    let apprence = match theme {
        Theme::Light => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameVibrantLight }),
        Theme::Dark => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameVibrantDark }),
        Theme::System => None,
    };
    app.webview_windows().values().for_each(|w| unsafe {
        if let Some(w) = (w.ns_window().unwrap() as *mut NSWindow).as_ref() {
            w.setAppearance(apprence.as_deref())
        }
    });
}

#[tauri::command]
#[specta::specta]
async fn get_detail_by_id(
    id: i64,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ChargingHistoryDetail, String> {
    let bytes = database::get_detail_by_id(&db, id).await;
    let detail = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    Ok(detail)
}

#[tauri::command]
#[specta::specta]
async fn get_all_charging_history(
    db: State<'_, Pool<Sqlite>>,
) -> Result<Vec<ChargingHistory>, String> {
    database::get_all_charging_history(&db)
        .await
        .map_err(|e| e.to_string())
}

pub fn create_specta() -> tauri_specta::Builder {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            open_app,
            is_main_window_hidden,
            open_settings,
            get_device_name,
            get_mac_name,
            switch_theme,
            get_detail_by_id,
            get_all_charging_history
        ])
        .events(collect_events![
            DeviceEvent,
            DevicePowerTickEvent,
            PowerTickEvent,
            PreferenceEvent,
            PowerUpdatedEvent,
            WindowLoadedEvent,
            HidePopoverEvent,
            HistoryRecordedEvent,
        ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                .header("// @ts-nocheck"),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    builder
}

pub fn run() {
    let specta = create_specta();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_pinia::init())
        .invoke_handler(specta.invoke_handler())
        .manage(DeviceState::default())
        .menu(setup_menu)
        .on_window_event(handle_window_event)
        .setup(move |app| {
            specta.mount_events(app);

            setup_database(app.handle().clone());

            setup_tray_icon(app)?;
            setup_sender_with_events(app);
            start_device_sender(app.app_handle().clone());
            setup_device_listener(app.app_handle().clone());
            setup_history_recorder(app.app_handle().clone());

            setup_traffic_light_positioner(app.main_window().unwrap());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app, event| match event {
        // prevent app from exiting when all windows are closed
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Reopen {
            has_visible_windows,
            ..
        } if !has_visible_windows => {
            app.main_window().unwrap().show().unwrap();
            app.set_activation_policy(ActivationPolicy::Regular)
                .unwrap();
        }
        _ => (),
    });
}

fn handle_window_event(window: &Window, event: &WindowEvent) {
    match window.label() {
        "main" => match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();

                window.hide().unwrap();
                window
                    .app_handle()
                    .set_activation_policy(ActivationPolicy::Accessory)
                    .unwrap();
            }
            WindowEvent::ThemeChanged(theme) => {
                println!("Theme changed to: {}", theme);
            }
            _ => (),
        },
        "popover" => match event {
            WindowEvent::Focused(focused) if !focused => {
                HidePopoverEvent.emit(window.app_handle()).unwrap();
            }
            _ => (),
        },
        _ => (),
    }
}
