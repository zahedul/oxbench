use eframe::egui;

use crate::style::palette::Palette;

pub const PANE_HEADER_HEIGHT: f32 = 30.0;

/// A fixed-height uppercase title row with actions on the right, so side-by-side panes line up.
pub fn show_pane_header(ui: &mut egui::Ui, title: &str, add_actions: impl FnOnce(&mut egui::Ui)) {
    let palette = Palette::of(ui);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), PANE_HEADER_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(
                egui::RichText::new(title.to_uppercase())
                    .size(11.0)
                    .color(palette.text_weak),
            );
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                add_actions,
            );
        },
    );
}
