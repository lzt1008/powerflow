use std::process;
use std::time::Duration;

use data::{start_sender, SenderMessage};
use event::{
    HidePopoverEvent, PowerTickEvent, PowerUpdatedEvent, PreferenceEvent, WindowLoadedEvent,
};
use ext::WebviewWindowExt;
use menu::setup_menu;
#[cfg(debug_assertions)]
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    async_runtime, ActivationPolicy, AppHandle, Manager, RunEvent, Runtime, Window, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_specta::{collect_commands, collect_events, Event};
use tokio::sync::mpsc;
use util::set_window_controls_pos;

mod data;
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

pub async fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![open_app, is_main_window_hidden, open_settings])
        .events(collect_events![
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

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_pinia::init())
        .invoke_handler(builder.invoke_handler())
        .menu(setup_menu)
        .on_window_event(handle_window_event)
        .setup(move |app| {
            builder.mount_events(app);
            setup_tray_icon(app)?;
            setup_sender_with_events(app);

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
                window.hide().unwrap();
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
