//! Labelled option rows for generator settings: labels in one column, controls in the next.

use std::ops::RangeInclusive;

use eframe::egui;

use crate::style::palette::Palette;

const LABEL_COLUMN_WIDTH: f32 = 96.0;
const SLIDER_WIDTH: f32 = 220.0;

/// A two-column grid; add rows with the `show_*_row` functions below.
pub fn show_form(ui: &mut egui::Ui, id_salt: &str, add_rows: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id_salt)
        .num_columns(2)
        .min_col_width(LABEL_COLUMN_WIDTH)
        .spacing(egui::vec2(12.0, 10.0))
        .show(ui, add_rows);
}

/// A dropdown row. Returns whether the choice changed.
pub fn show_choice_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let label_response = show_row_label(ui, label);
    let selected_text = options
        .iter()
        .find(|(value, _)| value == current)
        .map_or("", |(_, text)| *text);
    let before = *current;
    let combo = egui::ComboBox::from_id_salt(label)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (value, text) in options {
                ui.selectable_value(current, *value, *text);
            }
        });
    // Names the dropdown for screen readers, since its visible label sits beside it.
    let _ = combo.response.labelled_by(label_response.id);
    ui.end_row();
    *current != before
}

/// A slider row for a count or length. Returns whether the value changed.
pub fn show_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut usize,
    range: RangeInclusive<usize>,
) -> bool {
    let label_response = show_row_label(ui, label);
    // Scoped, so the wider slider does not leak into later rows of the grid.
    let changed = ui
        .scope(|ui| {
            ui.spacing_mut().slider_width = SLIDER_WIDTH;
            ui.add(egui::Slider::new(value, range).clamping(egui::SliderClamping::Always))
                .labelled_by(label_response.id)
                .changed()
        })
        .inner;
    ui.end_row();
    changed
}

/// A number field row for values that are usually typed, such as how many to generate.
pub fn show_number_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut usize,
    range: RangeInclusive<usize>,
) -> bool {
    let label_response = show_row_label(ui, label);
    let changed = ui
        .add(egui::DragValue::new(value).range(range).speed(1.0))
        .labelled_by(label_response.id)
        .changed();
    ui.end_row();
    changed
}

/// A row of checkboxes under one label. Returns whether any changed.
pub fn show_checkbox_row(ui: &mut egui::Ui, label: &str, boxes: &mut [(&mut bool, &str)]) -> bool {
    show_row_label(ui, label);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for (value, text) in boxes.iter_mut() {
            changed |= ui.checkbox(value, *text).changed();
        }
    });
    ui.end_row();
    changed
}

/// A row with any control; `add_control` fills the second column.
pub fn show_custom_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    add_control: impl FnOnce(&mut egui::Ui, egui::Id) -> R,
) -> R {
    let label_response = show_row_label(ui, label);
    let result = add_control(ui, label_response.id);
    ui.end_row();
    result
}

fn show_row_label(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let palette = Palette::of(ui);
    ui.label(egui::RichText::new(label).color(palette.text_weak))
}
