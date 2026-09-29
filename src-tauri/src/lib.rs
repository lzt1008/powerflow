use std::collections::HashSet;

use database::{setup_database, ChargingHistory};
use device::{setup_device_listener, start_device_sender, DevicePowerTickEvent, DeviceState};
use event::{DeviceEvent, PowerUpdatedEvent, PreferenceEvent, Theme, WindowLoadedEvent};
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
use tauri::{ActivationPolicy, AppHandle, Manager, RunEvent, Runtime, State, Window, WindowEvent};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::{collect_commands, collect_events};
use tpower::ffi::InterfaceType;
use tray_icon::setup_tray_icon;
use util::{log_err, setup_traffic_light_positioner};

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
    show_main_window(&app);
    if let Some(popover) = app.popover_window() {
        log_err(popover.hide(), "hide popover");
    }
}

pub(crate) fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.main_window() {
        log_err(main.show(), "show main window");
        log_err(main.set_focus(), "focus main window");
    }
    log_err(
        app.set_activation_policy(ActivationPolicy::Regular),
        "set activation policy",
    );
}

/// Hide the main window and drop the Dock icon; the app keeps running in the
/// menu bar.
fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.main_window() {
        log_err(main.hide(), "hide main window");
    }
    log_err(
        app.set_activation_policy(ActivationPolicy::Accessory),
        "set activation policy",
    );
}

#[tauri::command]
#[specta::specta]
fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}

pub(crate) fn open_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(settings) = app.settings_window() {
        log_err(settings.show(), "show settings window");
        log_err(settings.set_focus(), "focus settings window");
    }
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
    state.read().ok()?.get(&id).cloned()
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
    app.webview_windows()
        .values()
        .for_each(|w| match w.ns_window() {
            Ok(ns_window) => unsafe {
                if let Some(w) = (ns_window as *mut NSWindow).as_ref() {
                    w.setAppearance(apprence.as_deref())
                }
            },
            Err(e) => log::warn!("failed to apply window theme: {e}"),
        });
}

#[tauri::command]
#[specta::specta]
async fn get_detail_by_id(
    id: i64,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ChargingHistoryDetail, String> {
    let bytes = database::get_detail_by_id(&db, id).await?;
    let detail = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    Ok(detail)
}

#[tauri::command]
#[specta::specta]
async fn delete_history_by_id(id: i64, db: State<'_, Pool<Sqlite>>) -> Result<u64, String> {
    database::delete_history_by_id(&db, id)
        .await
        .map(|v| v.rows_affected())
        .map_err(|e| e.to_string())
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
            get_all_charging_history,
            delete_history_by_id
        ])
        .events(collect_events![
            DeviceEvent,
            DevicePowerTickEvent,
            PowerTickEvent,
            PreferenceEvent,
            PowerUpdatedEvent,
            WindowLoadedEvent,
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
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
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
        .plugin(tauri_plugin_nspopover::init())
        .invoke_handler(specta.invoke_handler())
        .manage(DeviceState::default())
        .menu(setup_menu)
        .on_window_event(handle_window_event)
        .setup(move |app| {
            specta.mount_events(app);

            setup_database(app.handle().clone())?;

            setup_tray_icon(app)?;
            setup_sender_with_events(app);
            start_device_sender(app.app_handle().clone());
            setup_device_listener(app.app_handle().clone());
            setup_history_recorder(app.app_handle().clone());

            if let Some(main) = app.main_window() {
                setup_traffic_light_positioner(main);
            }

            // The main window starts hidden (see tauri.conf.json) so it can
            // stay in the background when "hide on startup" is enabled.
            let hide_on_startup = app
                .app_handle()
                .pinia()
                .try_get::<bool>("preference", "hideOnStartup")
                .unwrap_or(false);
            if hide_on_startup {
                hide_main_window(app.app_handle());
            } else {
                show_main_window(app.app_handle());
            }

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
        } if !has_visible_windows => show_main_window(app),
        _ => (),
    });
}

fn handle_window_event(window: &Window, event: &WindowEvent) {
    if window.label() == "main" {
        match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();

                hide_main_window(window.app_handle());
            }
            WindowEvent::ThemeChanged(theme) => {
                println!("Theme changed to: {}", theme);
            }
            _ => (),
        }
    }
}
