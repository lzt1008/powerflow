use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{
    menu::{AboutMetadataBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Wry,
};
use tauri_specta::Event;

use crate::ext::WebviewWindowExt;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Type,
    Event,
    strum::EnumString,
    strum::AsRefStr,
    strum::Display,
)]
pub enum MenuEvent {
    Preferences,
    Close,
}

pub fn setup_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    app.on_menu_event(move |app, event| {
        if let Ok(event) = MenuEvent::from_str(&event.id.0) {
            handle_menu_event(app, event);
        } else {
            // println!("Unknown menu event: {}", event);
        }
    });
    let app_menu = SubmenuBuilder::new(app, "powerflow")
        .about(Some(
            AboutMetadataBuilder::new()
                .authors(Some(vec!["Samuel Lyon.".to_string()]))
                .license(Some(env!("CARGO_PKG_VERSION")))
                .version(Some(env!("CARGO_PKG_VERSION")))
                .build(),
        ))
        .separator()
        .item(
            &MenuItemBuilder::with_id(MenuEvent::Preferences, "Preferences")
                .accelerator("Cmd+,")
                .build(app)?,
        )
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View").fullscreen().build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_menu)
        // .item(&file_menu)
        // .item(&edit_menu)
        .item(&view_menu)
        // .item(&window_menu)
        .build()?;

    Ok(menu)
}

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    // event.emit(app).unwrap();
    match event {
        MenuEvent::Preferences => {
            app.get_or_create_window("settings")
                .unwrap()
                .0
                .show()
                .unwrap();
        }
        MenuEvent::Close => {
            app.exit(0);
        }
    }
}
