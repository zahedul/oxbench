use eframe::egui;

use crate::style::icons::{self, icon_text};
use crate::style::palette::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Success,
    Warning,
    Error,
}

/// A status message with a matching icon. The icon is a separate label, so the text alone
/// stays the accessible name.
pub fn show_status(ui: &mut egui::Ui, tone: StatusTone, text: &str) {
    let palette = Palette::of(ui);
    let (icon, color) = match tone {
        StatusTone::Neutral => (None, palette.text_weak),
        StatusTone::Success => (Some(icons::CHECK_CIRCLE), palette.success),
        StatusTone::Warning => (Some(icons::WARNING_CIRCLE), palette.warning),
        StatusTone::Error => (Some(icons::WARNING_CIRCLE), palette.error),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if let Some(icon) = icon {
            ui.label(icon_text(icon).size(15.0).color(color));
        }
        ui.colored_label(color, text);
    });
}
