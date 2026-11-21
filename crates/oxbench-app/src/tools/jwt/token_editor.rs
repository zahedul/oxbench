use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32, FontId, TextBuffer};

use crate::style::palette::Palette;

const EDITOR_ROWS: usize = 7;

/// A token input that colours the header, payload and signature differently, so the parts
/// are easy to tell apart.
pub fn show_token_editor(ui: &mut egui::Ui, token: &mut String) -> egui::Response {
    let palette = Palette::of(ui);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = color_token(text.as_str(), &palette, &font);
        job.wrap.max_width = wrap_width;
        // A token has no spaces, so it can only wrap mid-word.
        job.wrap.break_anywhere = true;
        ui.painter().layout_job(job)
    };
    ui.add(
        egui::TextEdit::multiline(token)
            .id_salt("jwt_token")
            .code_editor()
            .hint_text("Paste a JWT (a Bearer prefix is fine)")
            .desired_width(f32::INFINITY)
            .desired_rows(EDITOR_ROWS)
            .layouter(&mut layouter),
    )
}

/// One colour per dot-separated part; dots stay neutral.
pub(super) fn color_token(text: &str, palette: &Palette, font: &FontId) -> LayoutJob {
    let part_colors = [palette.error, palette.accent, palette.syntax_string];
    let mut job = LayoutJob::default();
    for (index, part) in text.split('.').enumerate() {
        if index > 0 {
            append(&mut job, ".", palette.text_weak, font);
        }
        let color = part_colors.get(index).copied().unwrap_or(palette.text);
        append(&mut job, part, color, font);
    }
    job
}

fn append(job: &mut LayoutJob, text: &str, color: Color32, font: &FontId) {
    job.append(text, 0.0, TextFormat::simple(font.clone(), color));
}
