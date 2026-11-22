#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cached;
mod clock;
mod desktop;
mod registry;
mod save_file;
mod settings;
mod shell;
mod style;
mod tasks;
mod tool;
mod tools;
mod update_check;
mod widgets;
mod workspace;

use app::OxbenchApp;

pub const APP_NAME: &str = "Oxbench";
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/256x256.png");

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([720.0, 480.0]);
    match eframe::icon_data::from_png_bytes(WINDOW_ICON_PNG) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        // The window still works without its icon, so this is reported rather than fatal.
        Err(error) => eprintln!("{APP_NAME}: the window icon couldn't be loaded: {error}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|creation_context| Ok(Box::new(OxbenchApp::with_desktop(creation_context)))),
    )
}
