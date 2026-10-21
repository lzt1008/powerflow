use std::{
    process,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

const FONT_ATTEMPTS: u8 = 5;

use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};
use tauri_plugin_nspopover::{AppExt, WindowExt as _};
use tauri_specta::Event;

use crate::{event::PowerUpdatedEvent, ext::WebviewWindowExt, show_main_window, util::log_err};

pub fn setup_tray_icon<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::new("Show Window").build(app)?;
    let quit = MenuItemBuilder::new("Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&quit)
        .build()?;

    // The menu must not stay attached to the status item. `NSStatusItem.setMenu`
    // makes AppKit handle `mouseDown:` itself and swallow it before it reaches
    // the tray icon's own view, so no Click event is ever emitted and the
    // popover can never be opened. `menu_on_left_click(false)` only flips
    // tray-icon's own flag; the AppKit-level interception stays. The menu is
    // therefore attached on demand for right clicks only (see below).
    let menu_for_right_click = menu.clone();

    let tray_icon = TrayIconBuilder::with_id("main")
        .title(PowerUpdatedEvent::new(0.0).0)
        .menu_on_left_click(false)
        .build(app)?;

    // Turn the popover window into a real `NSPopover` anchored to the status
    // item button. This has to happen before the tray event handler is
    // registered: it is what initializes the plugin's state, and the handler
    // reads the status bar button from it.
    if let Some(popover) = app.popover_window() {
        popover.to_popover();
    }

    tray_icon.on_menu_event(move |app, event| match event.id() {
        val if val == show.id() => show_main_window(app),
        val if val == quit.id() => {
            app.cleanup_before_exit();
            process::exit(0);
        }
        _ => {}
    });

    tray_icon.on_tray_icon_event(move |tray_handle, event| {
        tauri_plugin_positioner::on_tray_event(tray_handle.app_handle(), &event);
        match event {
            // Left click toggles the popover.
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                let handle = tray_handle.app_handle();
                // Make sure no menu is attached, otherwise AppKit would swallow
                // the click that opened the popover.
                log_err(
                    tray_handle.set_menu(None::<Menu<R>>),
                    "detach tray menu before opening the popover",
                );
                if handle.is_popover_shown() {
                    handle.hide_popover();
                } else {
                    handle.show_popover();
                }
            }
            // Right click gets the context menu. The menu cannot stay
            // attached (see above), so attach it for this click, ask the
            // button to perform its click so AppKit actually pops the menu,
            // then detach it again. `performClick` blocks until the menu
            // closes, so the matching mouse-up may never arrive and cannot be
            // relied on for cleanup.
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Down,
                ..
            } => {
                if let Err(error) = tray_handle.set_menu(Some(menu_for_right_click.clone())) {
                    log::error!("failed to attach tray menu: {error}");
                    return;
                }
                let button = tray_handle.app_handle().ns_statusbar_button();
                unsafe { button.performClick(None) };
                if let Err(error) = tray_handle.set_menu(None::<Menu<R>>) {
                    log::error!("failed to detach tray menu: {error}");
                }
            }
            _ => {}
        }
    });

    // Apply the monospaced-digit font on the first title updates, once the
    // status bar window exists. Give up after a few attempts.
    let font_attempts = Arc::new(AtomicU8::new(0));
    PowerUpdatedEvent::listen(app.app_handle(), move |event| {
        log_err(
            tray_icon.set_title(Some(event.payload.0)),
            "update tray title",
        );
        let attempt = font_attempts.load(Ordering::Relaxed);
        if attempt < FONT_ATTEMPTS {
            let font_attempts = font_attempts.clone();
            log_err(
                tray_icon.app_handle().run_on_main_thread(move || {
                    // SAFETY: running on the main thread.
                    if unsafe { use_monospaced_digits_in_status_bar() } {
                        font_attempts.store(u8::MAX, Ordering::Relaxed);
                    } else if font_attempts.fetch_add(1, Ordering::Relaxed) + 1 == FONT_ATTEMPTS {
                        log::warn!("status bar button not found; menu bar width may jitter");
                    }
                }),
                "schedule status bar font update",
            );
        }
    });

    Ok(())
}

/// The default menu bar font has proportional digits, so the status item
/// changes width on every update. Switch our status item button to the
/// monospaced-digit variant of the menu bar font. Returns whether the button
/// was found (the status bar window may not exist yet right after setup).
///
/// # Safety
/// Must be called on the main thread.
unsafe fn use_monospaced_digits_in_status_bar() -> bool {
    use cocoa::base::{id, nil, BOOL, NO};
    use objc::{class, msg_send, runtime::Class, sel, sel_impl};

    unsafe fn find_button(view: id, button_class: &Class) -> Option<id> {
        if view == nil {
            return None;
        }
        let is_button: BOOL = msg_send![view, isKindOfClass: button_class];
        if is_button != NO {
            return Some(view);
        }
        let subviews: id = msg_send![view, subviews];
        let count: usize = msg_send![subviews, count];
        (0..count).find_map(|i| {
            let child: id = msg_send![subviews, objectAtIndex: i];
            find_button(child, button_class)
        })
    }

    let (Some(status_window_class), Some(button_class)) =
        (Class::get("NSStatusBarWindow"), Class::get("NSButton"))
    else {
        return false;
    };

    let font_size: f64 = {
        let menu_font: id = msg_send![class!(NSFont), menuBarFontOfSize: 0.0f64];
        msg_send![menu_font, pointSize]
    };
    // NSFontWeightRegular
    let font: id =
        msg_send![class!(NSFont), monospacedDigitSystemFontOfSize: font_size weight: 0.0f64];
    if font == nil {
        return false;
    }

    let app: id = msg_send![class!(NSApplication), sharedApplication];
    let windows: id = msg_send![app, windows];
    let count: usize = msg_send![windows, count];
    let mut found = false;
    for i in 0..count {
        let window: id = msg_send![windows, objectAtIndex: i];
        let is_status_window: BOOL = msg_send![window, isKindOfClass: status_window_class];
        if is_status_window == NO {
            continue;
        }
        let content: id = msg_send![window, contentView];
        if let Some(button) = find_button(content, button_class) {
            let _: () = msg_send![button, setFont: font];
            found = true;
        }
    }
    found
}
