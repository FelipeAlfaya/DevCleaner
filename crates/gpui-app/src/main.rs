//! DevCleaner with a GPUI interface.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[path = "../../app/src/i18n.rs"]
mod i18n;
mod screens;
mod theme;
mod widgets;

use gpui::{px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions};

fn main() {
    let settings = cleaner_core::Settings::load();
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("DevCleaner".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(880.), px(580.))),
                app_id: Some("devcleaner".into()),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| app::DevCleaner::new(settings, cx)),
        )
        .expect("failed to open the window");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
