use eframe::egui;

use crate::style::fonts::semibold_family;
use crate::style::palette::Palette;

/// A bordered panel with a title, grouping one task inside a tool.
pub fn show_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let palette = Palette::of(ui);
    egui::Frame::default()
        .fill(palette.panel)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(title)
                    .family(semibold_family())
                    .size(15.0)
                    .color(palette.text_strong),
            );
            ui.add_space(6.0);
            add_contents(ui)
        })
        .inner
}
