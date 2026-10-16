#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod theme;

fn main() -> eframe::Result<()> {
    let settings = cleaner_core::Settings::load();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::app_title())
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        i18n::app_title(),
        native_options,
        Box::new(|cc| Ok(Box::new(app::CleanerApp::new(cc, settings)))),
    )
}
