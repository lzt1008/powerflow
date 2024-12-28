use std::ffi::c_void;

use objc2::msg_send;
use objc2_app_kit::{NSView, NSWindow, NSWindowButton};
use objc2_foundation::NSRect;

// From https://github.com/hoppscotch/hoppscotch/blob/a08c6f6b3eb7df152f1e0e9bc001e61bbe7a582e/packages/hoppscotch-selfhost-desktop/src-tauri/src/mac/window.rs
pub fn set_window_controls_pos(window: *mut c_void, x: f64, y: f64) {
    unsafe {
        let window = (window as *mut NSWindow).as_ref().unwrap();
        let close = window
            .standardWindowButton(NSWindowButton::NSWindowCloseButton)
            .unwrap();
        let miniaturize = window
            .standardWindowButton(NSWindowButton::NSWindowMiniaturizeButton)
            .unwrap();
        let zoom = window
            .standardWindowButton(NSWindowButton::NSWindowZoomButton)
            .unwrap();

        let title_bar_container_view = close.superview().unwrap().superview().unwrap();

        let close_rect: NSRect = msg_send![&close, frame];
        let button_height = close_rect.size.height;

        let title_bar_frame_height = button_height + y;
        let mut title_bar_rect = NSView::frame(&title_bar_container_view);
        title_bar_rect.size.height = title_bar_frame_height;
        title_bar_rect.origin.y = window.frame().size.height - title_bar_frame_height;
        let _: () = msg_send![&title_bar_container_view, setFrame: title_bar_rect];

        let window_buttons = vec![&close, &miniaturize, &zoom];
        let space_between = miniaturize.frame().origin.x - close.frame().origin.x;

        for (i, button) in window_buttons.into_iter().enumerate() {
            let mut rect: NSRect = button.frame();
            rect.origin.x = x + (i as f64 * space_between);
            button.setFrameOrigin(rect.origin);
        }
    }
}
