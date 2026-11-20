use eframe::egui::{self, FontId, Role, Sense, WidgetInfo};

use crate::style::palette::Palette;

const TAB_HEIGHT: f32 = 32.0;
const TAB_PADDING_X: f32 = 10.0;

/// Underlined tabs that switch between views of one tool.
pub fn show_view_tabs<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    options: &[(T, &str)],
) {
    let palette = Palette::of(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (value, label) in options {
            let is_selected = *current == *value;
            let color = if is_selected {
                palette.text_strong
            } else {
                palette.text_weak
            };
            let galley =
                ui.painter()
                    .layout_no_wrap((*label).to_owned(), FontId::proportional(14.0), color);
            let size = egui::vec2(galley.size().x + TAB_PADDING_X * 2.0, TAB_HEIGHT);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            response.widget_info(|| WidgetInfo::selected(Role::Tab, true, is_selected, *label));
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                if response.hovered() && !is_selected {
                    painter.rect_filled(
                        rect.shrink2(egui::vec2(0.0, 3.0)),
                        6,
                        palette.surface_hover,
                    );
                }
                let text_pos = rect.center() - galley.size() / 2.0;
                painter.galley(text_pos, galley, color);
                if is_selected {
                    let underline = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + 4.0, rect.bottom() - 2.0),
                        egui::pos2(rect.right() - 4.0, rect.bottom()),
                    );
                    painter.rect_filled(underline, 1, palette.accent);
                }
            }
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                *current = *value;
            }
        }
    });
}
