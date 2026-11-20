use eframe::egui::{self, Align2, FontId, Role, Sense, ThemePreference, WidgetInfo};

use super::ShellAction;
use super::theme::{next_theme, theme_icon, theme_label};
use crate::registry::ToolMeta;
use crate::style::fonts::icon_font;
use crate::style::icons;
use crate::style::palette::Palette;
use crate::widgets::icon_button::show_icon_button;
use crate::workspace::Workspace;

pub const TAB_BAR_HEIGHT: f32 = 40.0;
const TAB_HEIGHT: f32 = 34.0;
const TAB_FONT_SIZE: f32 = 13.5;
const SEARCH_BUTTON_WIDTH: f32 = 220.0;

pub fn show_tab_bar(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    theme: ThemePreference,
    palette_shortcut: &str,
    actions: &mut Vec<ShellAction>,
) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if show_home_tab(ui, workspace.active_tab.is_none()).clicked() {
            actions.push(ShellAction::ShowHome);
        }
        for id in &workspace.open_tabs {
            let is_active = workspace.active_tab == Some(*id);
            show_tool_tab(
                ui,
                id.meta(),
                is_active,
                workspace.is_favorite(*id),
                actions,
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if show_icon_button(ui, theme_icon(theme), theme_label(theme)).clicked() {
                actions.push(ShellAction::SetTheme(next_theme(theme)));
            }
            if show_search_button(ui, palette_shortcut).clicked() {
                actions.push(ShellAction::OpenPalette);
            }
        });
    });
}

fn show_home_tab(ui: &mut egui::Ui, is_active: bool) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(38.0, TAB_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, is_active, "Home"));
    paint_tab_background(ui, rect, is_active, response.hovered(), &palette);
    let color = if is_active {
        palette.accent
    } else {
        palette.text_weak
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icons::HOUSE,
        icon_font(16.0),
        color,
    );
    response.on_hover_text("Home")
}

fn show_tool_tab(
    ui: &mut egui::Ui,
    tool: ToolMeta,
    is_active: bool,
    is_favorite: bool,
    actions: &mut Vec<ShellAction>,
) {
    let palette = Palette::of(ui);
    let text_color = if is_active {
        palette.text_strong
    } else {
        palette.text_weak
    };
    let galley = ui.painter().layout_no_wrap(
        tool.name.to_owned(),
        FontId::proportional(TAB_FONT_SIZE),
        text_color,
    );
    let width = 12.0 + 16.0 + 8.0 + galley.size().x + 8.0 + 18.0 + 8.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, TAB_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, is_active, tool.name));

    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 17.0, rect.center().y),
        egui::vec2(18.0, 18.0),
    );
    let close = ui.interact(close_rect, response.id.with("close"), Sense::click());
    close.widget_info(|| WidgetInfo::labeled(Role::Button, true, format!("Close {}", tool.name)));

    let is_hovered = response.hovered() || close.hovered();
    paint_tab_background(ui, rect, is_active, is_hovered, &palette);
    let painter = ui.painter();
    let icon_color = if is_active {
        palette.accent
    } else {
        palette.text_weak
    };
    painter.text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        Align2::LEFT_CENTER,
        tool.icon,
        icon_font(15.0),
        icon_color,
    );
    let text_pos = egui::pos2(rect.left() + 36.0, rect.center().y - galley.size().y / 2.0);
    painter.galley(text_pos, galley, text_color);
    if is_active || is_hovered {
        if close.hovered() {
            painter.rect_filled(close_rect, 4, palette.surface_hover);
        }
        painter.text(
            close_rect.center(),
            Align2::CENTER_CENTER,
            icons::X,
            icon_font(12.0),
            palette.text_weak,
        );
    }

    if close.clicked() || response.middle_clicked() {
        actions.push(ShellAction::CloseTool(tool.id));
    } else if response.clicked() {
        actions.push(ShellAction::OpenTool(tool.id));
    }
    response.context_menu(|ui| {
        if ui.button("Close").clicked() {
            actions.push(ShellAction::CloseTool(tool.id));
        }
        let favorite_label = if is_favorite {
            "Remove from favorites"
        } else {
            "Add to favorites"
        };
        if ui.button(favorite_label).clicked() {
            actions.push(ShellAction::ToggleFavorite(tool.id));
        }
    });
}

/// The active tab takes the content background and an accent line, so it reads as part of the page.
fn paint_tab_background(
    ui: &egui::Ui,
    rect: egui::Rect,
    is_active: bool,
    is_hovered: bool,
    palette: &Palette,
) {
    let painter = ui.painter();
    let top_rounded = egui::CornerRadius {
        nw: 6,
        ne: 6,
        sw: 0,
        se: 0,
    };
    if is_active {
        let tab_rect =
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, ui.max_rect().bottom()));
        painter.rect_filled(tab_rect, top_rounded, palette.background);
        let accent_line = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0));
        painter.rect_filled(accent_line, top_rounded, palette.accent);
        return;
    }
    if is_hovered {
        painter.rect_filled(rect.shrink2(egui::vec2(0.0, 3.0)), 6, palette.surface_hover);
    }
}

fn show_search_button(ui: &mut egui::Ui, shortcut: &str) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(SEARCH_BUTTON_WIDTH, 28.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Search tools"));
    let painter = ui.painter();
    let fill = if response.hovered() {
        palette.surface_hover
    } else {
        palette.surface
    };
    painter.rect_filled(rect, 6, fill);
    painter.rect_stroke(
        rect,
        6,
        egui::Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        Align2::LEFT_CENTER,
        icons::MAGNIFYING_GLASS,
        icon_font(14.0),
        palette.text_weak,
    );
    painter.text(
        rect.left_center() + egui::vec2(32.0, 0.0),
        Align2::LEFT_CENTER,
        "Search tools",
        FontId::proportional(13.5),
        palette.text_weak,
    );
    painter.text(
        rect.right_center() - egui::vec2(10.0, 0.0),
        Align2::RIGHT_CENTER,
        shortcut,
        FontId::proportional(12.0),
        palette.text_weak,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
