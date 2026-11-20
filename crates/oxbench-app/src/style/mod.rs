//! Oxbench's look: fonts, icons, colour palette and widget styling, defined once.

pub mod fonts;
pub mod icons;
pub mod palette;
mod visuals;

use eframe::egui;

/// Call once at startup (and in UI tests) before drawing anything.
pub fn install_style(ctx: &egui::Context) {
    fonts::install_fonts(ctx);
    visuals::apply_visuals(ctx);
}
