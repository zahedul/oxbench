//! Phosphor icon glyphs (MIT, bundled in `assets/fonts/Phosphor.ttf`).
//! Codepoints come from Phosphor's regular weight. Draw them with `icon_text` or
//! `fonts::icon_font`; text fonts would show unrelated glyphs at these codepoints.

use eframe::egui::RichText;

use super::fonts::icon_family;

pub fn icon_text(icon: &str) -> RichText {
    RichText::new(icon).family(icon_family())
}

pub const ARROW_SQUARE_OUT: &str = "\u{E5DE}";
pub const ARROWS_CLOCKWISE: &str = "\u{E094}";
pub const ARROWS_LEFT_RIGHT: &str = "\u{E0A0}";
pub const ARROWS_IN: &str = "\u{E530}";
pub const BRACKETS_CURLY: &str = "\u{E860}";
pub const CHECK_CIRCLE: &str = "\u{E184}";
pub const CIRCLE_HALF: &str = "\u{E18C}";
pub const CLIPBOARD_TEXT: &str = "\u{E198}";
pub const CLOCK: &str = "\u{E19A}";
pub const CLOCK_HISTORY: &str = "\u{E1A0}";
pub const CODE: &str = "\u{E1BC}";
pub const COPY: &str = "\u{E1CA}";
pub const DICE_FIVE: &str = "\u{E1EE}";
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
pub const ERASER: &str = "\u{E21E}";
pub const EYE: &str = "\u{E220}";
pub const EYE_SLASH: &str = "\u{E224}";
pub const FINGERPRINT: &str = "\u{E23E}";
pub const FOLDER_OPEN: &str = "\u{E256}";
pub const GEAR_SIX: &str = "\u{E272}";
pub const GIT_DIFF: &str = "\u{E27C}";
pub const HASH: &str = "\u{E2A2}";
pub const HOUSE: &str = "\u{E2C2}";
pub const KEY: &str = "\u{E2D6}";
pub const LOCK_KEY: &str = "\u{E2FE}";
pub const MAGIC_WAND: &str = "\u{E6B6}";
pub const MAGNIFYING_GLASS: &str = "\u{E30C}";
pub const MOON: &str = "\u{E330}";
pub const NETWORK: &str = "\u{EDDE}";
pub const PARAGRAPH: &str = "\u{E960}";
pub const PASSWORD: &str = "\u{E752}";
pub const QR_CODE: &str = "\u{E3E6}";
pub const SEAL_CHECK: &str = "\u{E606}";
pub const SHIELD_CHECK: &str = "\u{E40C}";
pub const STAR: &str = "\u{E46A}";
pub const SUN: &str = "\u{E472}";
pub const WARNING_CIRCLE: &str = "\u{E4E2}";
pub const X: &str = "\u{E4F6}";
