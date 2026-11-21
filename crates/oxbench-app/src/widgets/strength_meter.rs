use eframe::egui;

use crate::style::palette::Palette;

const SEGMENT_COUNT: u8 = 4;
const SEGMENT_HEIGHT: f32 = 6.0;
const SEGMENT_GAP: f32 = 4.0;

/// Four bars that fill with the 0–4 strength score, followed by its label.
pub fn show_strength_meter(ui: &mut egui::Ui, score: u8, label: &str) {
    let palette = Palette::of(ui);
    let color = match score {
        0 | 1 => palette.error,
        2 => palette.warning,
        _ => palette.success,
    };
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, SEGMENT_HEIGHT), egui::Sense::hover());
    let segment_width =
        (width - SEGMENT_GAP * f32::from(SEGMENT_COUNT - 1)) / f32::from(SEGMENT_COUNT);
    // A score of 0 still lights one bar, so "very weak" reads as a result rather than nothing.
    let filled = score.max(1);
    for index in 0..SEGMENT_COUNT {
        let left = rect.left() + f32::from(index) * (segment_width + SEGMENT_GAP);
        let segment = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(segment_width, SEGMENT_HEIGHT),
        );
        let fill = if index < filled {
            color
        } else {
            palette.border
        };
        ui.painter().rect_filled(segment, 3, fill);
    }
    ui.add_space(4.0);
    ui.colored_label(color, format!("Strength: {label}"));
}
