use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

const INTER: &str = "inter";
const INTER_SEMIBOLD: &str = "inter-semibold";
const JETBRAINS_MONO: &str = "jetbrains-mono";
const PHOSPHOR: &str = "phosphor";

/// Headings and emphasis. egui has no font weights, so semibold is its own family.
const SEMIBOLD: &str = "semibold";
/// Phosphor gets its own family: it also maps a-z (for ligatures) and Inter has glyphs in the
/// private-use range, so mixing them in one family shows the wrong glyphs.
const ICONS: &str = "icons";

pub fn semibold_family() -> FontFamily {
    FontFamily::Name(SEMIBOLD.into())
}

pub fn icon_family() -> FontFamily {
    FontFamily::Name(ICONS.into())
}

pub fn icon_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, icon_family())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    insert_font(
        &mut fonts,
        INTER,
        include_bytes!("../../../../assets/fonts/Inter-Regular.ttf"),
    );
    insert_font(
        &mut fonts,
        INTER_SEMIBOLD,
        include_bytes!("../../../../assets/fonts/Inter-SemiBold.ttf"),
    );
    insert_font(
        &mut fonts,
        JETBRAINS_MONO,
        include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf"),
    );
    insert_font(
        &mut fonts,
        PHOSPHOR,
        include_bytes!("../../../../assets/fonts/Phosphor.ttf"),
    );

    // egui's bundled fonts stay at the end of each list as fallbacks for symbols and emoji.
    let proportional_fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallbacks = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    fonts.families.insert(
        FontFamily::Proportional,
        with_fallbacks(&[INTER], &proportional_fallbacks),
    );
    fonts.families.insert(
        FontFamily::Monospace,
        with_fallbacks(&[JETBRAINS_MONO], &monospace_fallbacks),
    );
    fonts.families.insert(
        semibold_family(),
        with_fallbacks(&[INTER_SEMIBOLD, INTER], &proportional_fallbacks),
    );
    fonts
        .families
        .insert(icon_family(), vec![PHOSPHOR.to_owned()]);
    ctx.set_fonts(fonts);
}

fn insert_font(fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]) {
    fonts
        .font_data
        .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
}

fn with_fallbacks(primary: &[&str], fallbacks: &[String]) -> Vec<String> {
    primary
        .iter()
        .map(|name| (*name).to_owned())
        .chain(fallbacks.iter().cloned())
        .collect()
}
