#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cached;
mod clock;
mod registry;
mod save_file;
mod shell;
mod style;
mod tasks;
mod tool;
mod tools;
mod widgets;
mod workspace;

use app::OxbenchApp;

pub const APP_NAME: &str = "Oxbench";

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|creation_context| Ok(Box::new(OxbenchApp::new(creation_context)))),
    )
}
