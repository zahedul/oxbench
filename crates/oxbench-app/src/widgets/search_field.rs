use eframe::egui;

use crate::style::icons::{self, icon_text};
use crate::style::palette::Palette;

/// A framed single-line search box with a leading magnifier icon. Returns the text field's response.
pub fn show_search_field(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    font_size: f32,
) -> egui::Response {
    let palette = Palette::of(ui);
    egui::Frame::default()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(
                    icon_text(icons::MAGNIFYING_GLASS)
                        .size(font_size)
                        .color(palette.text_weak),
                );
                ui.add(
                    egui::TextEdit::singleline(text)
                        .frame(egui::Frame::NONE)
                        .hint_text(hint.to_owned())
                        .font(egui::FontId::proportional(font_size))
                        .align(egui::Align2::LEFT_CENTER)
                        .desired_width(f32::INFINITY),
                )
            })
            .inner
        })
        .inner
}
