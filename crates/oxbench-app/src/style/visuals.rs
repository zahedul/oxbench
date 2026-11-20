use eframe::egui::{
    self, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Theme, Visuals,
};

use super::fonts::semibold_family;
use super::palette::{DARK, LIGHT, Palette};

const WIDGET_RADIUS: u8 = 6;
const WINDOW_RADIUS: u8 = 10;

pub fn apply_visuals(ctx: &egui::Context) {
    ctx.set_visuals_of(Theme::Dark, build_visuals(&DARK, Visuals::dark()));
    ctx.set_visuals_of(Theme::Light, build_visuals(&LIGHT, Visuals::light()));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (TextStyle::Heading, FontId::new(20.0, semibold_family())),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();
        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 6.0);
        spacing.button_padding = egui::vec2(10.0, 5.0);
        spacing.interact_size.y = 28.0;
        spacing.menu_margin = Margin::same(6);
        spacing.indent = 16.0;
        spacing.icon_spacing = 6.0;
    });
}

fn build_visuals(palette: &Palette, base: Visuals) -> Visuals {
    let mut visuals = base;
    let radius = CornerRadius::same(WIDGET_RADIUS);

    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.panel;
    visuals.window_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.window_corner_radius = CornerRadius::same(WINDOW_RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: egui::Color32::from_black_alpha(if visuals.dark_mode { 120 } else { 40 }),
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.extreme_bg_color = palette.surface;
    visuals.text_edit_bg_color = Some(palette.surface);
    visuals.code_bg_color = palette.surface;
    visuals.faint_bg_color = palette.surface;
    visuals.hyperlink_color = palette.accent;
    visuals.error_fg_color = palette.error;
    visuals.warn_fg_color = palette.warning;
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = true;

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = palette.panel;
    widgets.noninteractive.weak_bg_fill = palette.panel;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = palette.surface;
    widgets.inactive.weak_bg_fill = palette.surface;
    widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    widgets.inactive.corner_radius = radius;

    widgets.hovered.bg_fill = palette.surface_hover;
    widgets.hovered.weak_bg_fill = palette.surface_hover;
    widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    widgets.hovered.fg_stroke = Stroke::new(1.5, palette.text_strong);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = palette.accent_soft;
    widgets.active.weak_bg_fill = palette.accent_soft;
    widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    widgets.active.fg_stroke = Stroke::new(1.5, palette.text_strong);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open = widgets.hovered;
    visuals
}
