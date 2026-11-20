use eframe::egui;

use crate::style::palette::Palette;
use crate::widgets::copy_button::show_copy_icon_button;

const MIN_LABEL_WIDTH: f32 = 72.0;
const MAX_LABEL_WIDTH: f32 = 170.0;
const LABEL_GAP: f32 = 14.0;
/// Matches `show_icon_button`, so the value knows how much room to leave for it.
const COPY_BUTTON_WIDTH: f32 = 28.0;
const MIN_VALUE_WIDTH: f32 = 80.0;

/// One labelled result, such as "UTC" and its date.
pub struct ValueRow {
    pub label: String,
    pub value: String,
}

impl ValueRow {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Label, selectable monospace value and a copy button per row. Labels share one column width;
/// long values wrap within the space left beside them.
pub fn show_value_list(ui: &mut egui::Ui, rows: &[ValueRow]) {
    let palette = Palette::of(ui);
    let label_width = measure_label_column(ui, rows);
    let row_height = ui.spacing().interact_size.y;
    for row in rows {
        // Top-aligned with equal minimum heights, so a label stays level with the first line
        // of a value that wraps.
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(label_width);
                    ui.add(
                        egui::Label::new(egui::RichText::new(&row.label).color(palette.text_weak))
                            .truncate(),
                    );
                },
            );
            let value_width =
                (ui.available_width() - COPY_BUTTON_WIDTH - ui.spacing().item_spacing.x)
                    .max(MIN_VALUE_WIDTH);
            ui.allocate_ui_with_layout(
                egui::vec2(value_width, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(egui::vec2(value_width, row_height));
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&row.value)
                                .monospace()
                                .color(palette.text_strong),
                        )
                        .wrap(),
                    );
                },
            );
            show_copy_icon_button(ui, &format!("Copy {}", row.label), &row.value);
        });
    }
}

fn measure_label_column(ui: &egui::Ui, rows: &[ValueRow]) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let widest = rows
        .iter()
        .map(|row| {
            ui.painter()
                .layout_no_wrap(row.label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
                .size()
                .x
        })
        .fold(0.0, f32::max);
    (widest + LABEL_GAP).clamp(MIN_LABEL_WIDTH, MAX_LABEL_WIDTH)
}
