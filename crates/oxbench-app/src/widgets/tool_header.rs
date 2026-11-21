use eframe::egui;

use crate::style::palette::Palette;

/// A tool's top row, such as view tabs on the left and actions on the right, with a rule
/// beneath it. Actions run right to left, so add the rightmost first.
pub fn show_tool_header(
    ui: &mut egui::Ui,
    add_left: impl FnOnce(&mut egui::Ui),
    add_actions: impl FnOnce(&mut egui::Ui),
) {
    let palette = Palette::of(ui);
    let row = ui.horizontal(|ui| {
        add_left(ui);
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            add_actions,
        );
    });
    // Measured from the row itself: the tool's own area may already span the whole page.
    ui.painter().hline(
        ui.max_rect().x_range(),
        row.response.rect.bottom() + 4.0,
        egui::Stroke::new(1.0, palette.border),
    );
    ui.add_space(14.0);
}
