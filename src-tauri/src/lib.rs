use std::{
    collections::{HashMap, HashSet},
    process,
    sync::Mutex,
    time::Duration,
};

use data::{start_device_sender, start_sender, PowerTickEvent, SenderMessage};
use event::{
    DeviceEvent, DevicePowerTickEvent, HidePopoverEvent, PowerUpdatedEvent, PreferenceEvent, Theme,
    WindowLoadedEvent,
};
use ext::WebviewWindowExt;
use menu::setup_menu;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight, NSWindow,
};
use scopefn::Run;
#[cfg(debug_assertions)]
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri::{
    async_runtime,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, AppHandle, Manager, RunEvent, Runtime, State, Window, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_specta::{collect_commands, collect_events, Event};
use tokio::sync::mpsc;
use tpower::ffi::{Action, InterfaceType};
use util::set_window_controls_pos;

pub mod data;
mod event;
mod ext;
mod menu;
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
    state: State<Mutex<AppState>>,
) -> Option<(String, HashSet<InterfaceType>)> {
    let state = state.lock().unwrap();
    let data = state.devices.get(&id);
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
        (w.ns_window().unwrap() as *mut NSWindow)
            .as_ref()
            .map(|w| w.setAppearance(apprence.as_deref()));
    });
}

pub struct AppState {
    devices: HashMap<String, (String, HashSet<InterfaceType>)>,
}

pub async fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            open_app,
            is_main_window_hidden,
            open_settings,
            get_device_name,
            get_mac_name,
            switch_theme
        ])
        .events(collect_events![
            DeviceEvent,
            DevicePowerTickEvent,
            PowerTickEvent,
            PreferenceEvent,
            PowerUpdatedEvent,
            WindowLoadedEvent,
            HidePopoverEvent
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

    let app_state = Mutex::new(AppState {
        devices: HashMap::new(),
    });

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_pinia::init())
        .invoke_handler(builder.invoke_handler())
        .manage(app_state)
        .menu(setup_menu)
        .on_window_event(handle_window_event)
        .setup(move |app| {
            builder.mount_events(app);

            let handle = app.app_handle().clone();

            DeviceEvent::listen(app, move |event| {
                let event = event.payload;
                let app_state: State<Mutex<AppState>> = handle.state();

                app_state
                    .lock()
                    .unwrap()
                    .devices
                    .entry(event.udid.clone())
                    .or_insert_with(|| (event.name, HashSet::new()))
                    .run(|e| match event.action {
                        Action::Attached => {
                            e.1.insert(event.interface);
                        }
                        Action::Detached => {
                            e.1.remove(&event.interface);
                        }
                        _ => (),
                    });
            });

            setup_tray_icon(app)?;
            setup_sender_with_events(app);
            start_device_sender(app.app_handle().clone());

            set_window_controls_pos(app.main_window().unwrap().ns_window().unwrap(), 22., 24.);

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
            WindowEvent::Resized(_) => {
                // set_window_controls_pos(window.ns_window().unwrap(), 22., 24.);
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

fn setup_tray_icon<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::new("Show Window").build(app)?;
    let quit = MenuItemBuilder::new("Quit").build(app)?;

    let menu = MenuBuilder::new(app)
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
                HidePopoverEvent.emit(tray_handle.app_handle()).unwrap();
            } else {
                window.move_window(Position::TrayLeft).unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
        }
    });

    PowerUpdatedEvent::listen(app.app_handle(), move |event| {
        tray_icon.set_title(Some(event.payload.0)).unwrap();
    });

    Ok(())
}

fn setup_sender_with_events<R: Runtime>(app: &impl Manager<R>) {
    let app = app.app_handle();
    let (sender_tx, rx) = mpsc::channel(10);
    start_sender(app, rx);

    // send an immediate update when the main window is loaded
    let tx = sender_tx.clone();
    WindowLoadedEvent::listen(app, move |_| {
        let tx = tx.clone();
        async_runtime::spawn(async move {
            tx.send(SenderMessage::ImmediateSend).await.unwrap();
        });
    });

    let tx = sender_tx.clone();
    PreferenceEvent::listen(app, move |event| {
        if let Some(msg) = match event.payload {
            PreferenceEvent::UpdateInterval(interval) => Some(SenderMessage::ChangeInterval(
                Duration::from_millis(interval.into()),
            )),
            PreferenceEvent::StatusBarItem(item) => Some(SenderMessage::ChangeStatusBarItem(item)),
            PreferenceEvent::StatusBarShowCharging(show) => {
                Some(SenderMessage::StatusBarShowCharging(show))
            }
            PreferenceEvent::Language(_) => {
                // No need to send, perform some menu refreshing
                None
            }
            _ => None,
        } {
            let tx = tx.clone();
            async_runtime::spawn(async move {
                tx.send(msg).await.unwrap();
            });
        }
    });
}
