use eframe::egui::{self, Align2, Role, Sense, WidgetInfo};

use crate::style::fonts::icon_font;
use crate::style::icons::icon_text;
use crate::style::palette::Palette;

const SIZE: f32 = 28.0;

/// A square, frameless icon button. `label` is the tooltip and the accessible name.
pub fn show_icon_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(SIZE, SIZE), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, 6, palette.surface_hover);
        }
        let color = if response.hovered() {
            palette.text_strong
        } else {
            palette.text_weak
        };
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            icon_font(17.0),
            color,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

/// A regular framed button with a leading icon.
pub fn icon_label_button(icon: &str, label: &str) -> egui::Button<'static> {
    egui::Button::new((icon_text(icon), label.to_owned()))
}
