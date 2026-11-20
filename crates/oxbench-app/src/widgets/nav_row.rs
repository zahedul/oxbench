use eframe::egui::{self, Align2, FontId, Role, Sense, WidgetInfo};

use crate::style::fonts::icon_font;
use crate::style::icons::icon_text;
use crate::style::palette::Palette;

const ROW_HEIGHT: f32 = 30.0;
const ICON_X: f32 = 12.0;
const LABEL_X: f32 = 38.0;

/// A full-width navigation row: icon and label, hover tint, accent bar when active.
pub fn show_nav_row(ui: &mut egui::Ui, icon: &str, label: &str, is_active: bool) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, is_active, label));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let fill = if is_active {
        palette.accent_soft
    } else if response.hovered() {
        palette.surface_hover
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 6, fill);
    if is_active {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 7.0),
            egui::vec2(3.0, ROW_HEIGHT - 14.0),
        );
        painter.rect_filled(bar, 2, palette.accent);
    }
    let (icon_color, label_color) = if is_active {
        (palette.accent, palette.text_strong)
    } else {
        (palette.text_weak, palette.text)
    };
    painter.text(
        rect.left_center() + egui::vec2(ICON_X, 0.0),
        Align2::LEFT_CENTER,
        icon,
        icon_font(16.0),
        icon_color,
    );
    painter.text(
        rect.left_center() + egui::vec2(LABEL_X, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        label_color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Small uppercase label that starts a group of rows, with an optional leading icon.
pub fn show_section_label(ui: &mut egui::Ui, icon: Option<&str>, text: &str) {
    let palette = Palette::of(ui);
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        if let Some(icon) = icon {
            ui.label(icon_text(icon).size(12.0).color(palette.text_weak));
        }
        ui.label(
            egui::RichText::new(text.to_uppercase())
                .size(11.0)
                .color(palette.text_weak),
        );
    });
    ui.add_space(2.0);
}
