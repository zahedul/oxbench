use eframe::egui::{self, Color32};

/// Colour tokens. Widgets read these instead of hard-coding colours, so both themes stay consistent.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Content area behind tools and the active tab.
    pub background: Color32,
    /// Sidebar and tab strip.
    pub panel: Color32,
    /// Inputs, cards, buttons.
    pub surface: Color32,
    pub surface_hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub text_weak: Color32,
    pub accent: Color32,
    /// Accent tint for selected rows and highlights.
    pub accent_soft: Color32,
    pub success: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub syntax_string: Color32,
    pub syntax_number: Color32,
    pub syntax_bool: Color32,
    pub syntax_null: Color32,
    /// Row tints for compared text, with a stronger shade for the exact words that changed.
    pub diff_added: Color32,
    pub diff_added_strong: Color32,
    pub diff_removed: Color32,
    pub diff_removed_strong: Color32,
}

pub const DARK: Palette = Palette {
    background: Color32::from_rgb(0x15, 0x17, 0x1C),
    panel: Color32::from_rgb(0x1B, 0x1E, 0x25),
    surface: Color32::from_rgb(0x23, 0x27, 0x30),
    surface_hover: Color32::from_rgb(0x2C, 0x31, 0x3C),
    border: Color32::from_rgb(0x2A, 0x2F, 0x39),
    border_strong: Color32::from_rgb(0x3B, 0x41, 0x4E),
    text: Color32::from_rgb(0xD5, 0xD9, 0xE0),
    text_strong: Color32::from_rgb(0xF2, 0xF4, 0xF7),
    text_weak: Color32::from_rgb(0x86, 0x8E, 0x9C),
    accent: Color32::from_rgb(0x8B, 0x93, 0xFF),
    accent_soft: Color32::from_rgb(0x2C, 0x2F, 0x52),
    success: Color32::from_rgb(0x5C, 0xD1, 0x8C),
    error: Color32::from_rgb(0xF2, 0x7A, 0x7A),
    warning: Color32::from_rgb(0xF2, 0xC0, 0x5C),
    syntax_string: Color32::from_rgb(0x9C, 0xD6, 0x8A),
    syntax_number: Color32::from_rgb(0x7C, 0xB7, 0xFF),
    syntax_bool: Color32::from_rgb(0xF0, 0xA3, 0x6E),
    syntax_null: Color32::from_rgb(0x8A, 0x91, 0x9E),
    diff_added: Color32::from_rgb(0x19, 0x2B, 0x23),
    diff_added_strong: Color32::from_rgb(0x23, 0x55, 0x3B),
    diff_removed: Color32::from_rgb(0x30, 0x1D, 0x21),
    diff_removed_strong: Color32::from_rgb(0x66, 0x2B, 0x31),
};

pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    panel: Color32::from_rgb(0xF5, 0xF6, 0xF8),
    surface: Color32::from_rgb(0xF1, 0xF2, 0xF5),
    surface_hover: Color32::from_rgb(0xE7, 0xE9, 0xEE),
    border: Color32::from_rgb(0xE1, 0xE4, 0xEA),
    border_strong: Color32::from_rgb(0xC9, 0xCE, 0xD8),
    text: Color32::from_rgb(0x2B, 0x30, 0x3A),
    text_strong: Color32::from_rgb(0x11, 0x14, 0x1A),
    text_weak: Color32::from_rgb(0x6B, 0x72, 0x80),
    accent: Color32::from_rgb(0x4F, 0x5B, 0xD5),
    accent_soft: Color32::from_rgb(0xE6, 0xE8, 0xFB),
    success: Color32::from_rgb(0x1F, 0x8F, 0x52),
    error: Color32::from_rgb(0xD0, 0x3B, 0x3B),
    warning: Color32::from_rgb(0xB7, 0x79, 0x0F),
    syntax_string: Color32::from_rgb(0x2E, 0x7D, 0x32),
    syntax_number: Color32::from_rgb(0x1F, 0x5F, 0xBF),
    syntax_bool: Color32::from_rgb(0xB4, 0x55, 0x0B),
    syntax_null: Color32::from_rgb(0x7A, 0x80, 0x8C),
    diff_added: Color32::from_rgb(0xE9, 0xF7, 0xEE),
    diff_added_strong: Color32::from_rgb(0xB5, 0xE4, 0xC4),
    diff_removed: Color32::from_rgb(0xFD, 0xEC, 0xEC),
    diff_removed_strong: Color32::from_rgb(0xF6, 0xBF, 0xBF),
};

impl Palette {
    pub const fn for_dark_mode(is_dark: bool) -> Palette {
        if is_dark { DARK } else { LIGHT }
    }

    pub fn of(ui: &egui::Ui) -> Palette {
        Palette::for_dark_mode(ui.visuals().dark_mode)
    }
}

impl Palette {
    pub fn of_context(ctx: &egui::Context) -> Palette {
        Palette::for_dark_mode(ctx.theme() == egui::Theme::Dark)
    }
}
