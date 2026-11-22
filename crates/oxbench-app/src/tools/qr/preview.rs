//! Draws the code on screen and builds the image "Copy image" puts on the clipboard.

use eframe::egui::{self, Color32, ColorImage, Role, Sense, WidgetInfo};
use oxbench_core::qr::{QrMatrix, QrStyle};

use crate::style::palette::Palette;

const MAX_PREVIEW_SIZE: f32 = 420.0;
const MIN_PREVIEW_SIZE: f32 = 120.0;
/// Clipboard images get at most this many pixels a side, like the PNG export.
const MAX_CLIPBOARD_SIZE: usize = 2048;

/// A square preview as wide as the pane allows, up to `MAX_PREVIEW_SIZE`.
#[allow(
    clippy::cast_precision_loss,
    reason = "a QR code is at most 177 modules plus a margin, far below f32's exact range"
)]
pub fn show_preview(ui: &mut egui::Ui, matrix: &QrMatrix, style: QrStyle) {
    let side = ui
        .available_width()
        .clamp(MIN_PREVIEW_SIZE, MAX_PREVIEW_SIZE);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, "QR code preview"));
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    let [light_red, light_green, light_blue] = style.light;
    let [dark_red, dark_green, dark_blue] = style.dark;
    painter.rect_filled(
        rect,
        4,
        Color32::from_rgb(light_red, light_green, light_blue),
    );
    // Outlined, so a white code still has an edge on a white page.
    painter.rect_stroke(
        rect,
        4,
        egui::Stroke::new(1.0, Palette::of(ui).border),
        egui::StrokeKind::Inside,
    );
    let dark = Color32::from_rgb(dark_red, dark_green, dark_blue);
    let module = side / (matrix.width + style.quiet_zone * 2) as f32;
    let origin = rect.min + egui::vec2(module, module) * style.quiet_zone as f32;
    // Runs of dark modules in a row are drawn as one rectangle, so dense codes stay cheap.
    for y in 0..matrix.width {
        let mut x = 0;
        while x < matrix.width {
            if !matrix.is_dark(x, y) {
                x += 1;
                continue;
            }
            let run_start = x;
            while x < matrix.width && matrix.is_dark(x, y) {
                x += 1;
            }
            let min = origin + egui::vec2(run_start as f32, y as f32) * module;
            let run_size = egui::vec2((x - run_start) as f32 * module, module);
            painter.rect_filled(egui::Rect::from_min_size(min, run_size), 0, dark);
        }
    }
}

/// The code at roughly `target_size` pixels, each module a whole number of pixels.
pub fn to_color_image(matrix: &QrMatrix, style: QrStyle, target_size: usize) -> ColorImage {
    let modules = matrix.width + style.quiet_zone * 2;
    let scale = (target_size.min(MAX_CLIPBOARD_SIZE) / modules).max(1);
    let size = modules * scale;
    let mut rgb = Vec::with_capacity(size * size * 3);
    for pixel_y in 0..size {
        for pixel_x in 0..size {
            let (module_x, module_y) = (pixel_x / scale, pixel_y / scale);
            let is_dark = module_x >= style.quiet_zone
                && module_y >= style.quiet_zone
                && matrix.is_dark(module_x - style.quiet_zone, module_y - style.quiet_zone);
            rgb.extend_from_slice(if is_dark { &style.dark } else { &style.light });
        }
    }
    ColorImage::from_rgb([size, size], &rgb)
}
