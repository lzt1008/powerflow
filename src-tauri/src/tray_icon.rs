use std::process;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, Manager, Runtime,
};
use tauri_plugin_nspopover::{AppExt, WindowExt as _};
use tauri_specta::Event;

use crate::{event::PowerUpdatedEvent, ext::WebviewWindowExt};

pub fn setup_tray_icon<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::new("Show Window").build(app)?;
    let quit = MenuItemBuilder::new("Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&quit)
        .build()?;

    let tray_icon = TrayIconBuilder::with_id("main")
        .title("0 w")
        .menu_on_left_click(false)
        .menu(&menu)
        .build(app)?;

    tray_icon.on_menu_event(move |tray_handle, event| match event.id() {
        val if val == show.id() => match tray_handle.app_handle().get_or_create_window("main") {
            Ok((window, _)) => {
                if !window.is_visible().unwrap_or(false) {
                    if let Err(error) = window.show() {
                        log::error!("Failed to show main window: {error}");
                    }
                    if let Err(error) = window.set_focus() {
                        log::error!("Failed to focus main window: {error}");
                    }
                    if let Err(error) = tray_handle
                        .app_handle()
                        .set_activation_policy(ActivationPolicy::Regular)
                    {
                        log::error!("Failed to update activation policy: {error}");
                    }
                }
            }
            Err(error) => log::error!("Failed to create main window: {error}"),
        },
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
            let handle = tray_handle.app_handle();
            if handle.is_popover_shown() {
                handle.hide_popover();
            } else {
                handle.show_popover();
            }
        }
    });

    PowerUpdatedEvent::listen(app.app_handle(), move |event| {
        if let Err(error) = tray_icon.set_title(Some(event.payload.0)) {
            log::error!("Failed to update tray title: {error}");
        }
    });

    if let Some(popover) = app.popover_window() {
        popover.to_popover();
    }

    Ok(())
}
