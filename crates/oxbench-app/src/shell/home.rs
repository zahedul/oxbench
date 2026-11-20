use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{self, Align2, FontId, Role, Sense, WidgetInfo};

use super::ShellAction;
use crate::APP_NAME;
use crate::registry::{ToolId, ToolMeta};
use crate::style::fonts::{icon_font, semibold_family};
use crate::style::icons;
use crate::style::palette::Palette;
use crate::widgets::nav_row::show_section_label;
use crate::workspace::Workspace;

const CONTENT_MAX_WIDTH: f32 = 1100.0;
const CARD_SIZE: egui::Vec2 = egui::vec2(272.0, 80.0);
const CARD_GAP: f32 = 12.0;
const SEARCH_BAR_MAX_WIDTH: f32 = 560.0;

pub fn show_home(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    palette_shortcut: &str,
    actions: &mut Vec<ShellAction>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.set_max_width(CONTENT_MAX_WIDTH);
            ui.add_space(12.0);
            show_greeting(ui);
            ui.add_space(16.0);
            if show_search_bar(ui, palette_shortcut).clicked() {
                actions.push(ShellAction::OpenPalette);
            }
            ui.add_space(8.0);
            show_card_section(
                ui,
                Some(icons::STAR),
                "Favorites",
                &workspace.favorites,
                actions,
            );
            show_card_section(
                ui,
                Some(icons::CLOCK_HISTORY),
                "Recent",
                &workspace.recent,
                actions,
            );
            show_card_section(ui, None, "All tools", &ToolId::ALL, actions);
            ui.add_space(16.0);
        });
}

fn show_greeting(ui: &mut egui::Ui) {
    let palette = Palette::of(ui);
    ui.label(
        egui::RichText::new(APP_NAME)
            .family(semibold_family())
            .size(28.0)
            .color(palette.text_strong),
    );
    ui.label(
        egui::RichText::new(format!(
            "Your offline developer toolkit. {} tools, and nothing leaves your machine.",
            ToolId::ALL.len()
        ))
        .color(palette.text_weak),
    );
}

fn show_search_bar(ui: &mut egui::Ui, shortcut: &str) -> egui::Response {
    let palette = Palette::of(ui);
    let width = ui.available_width().min(SEARCH_BAR_MAX_WIDTH);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 40.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Search all tools"));
    let painter = ui.painter();
    let border = if response.hovered() {
        palette.accent
    } else {
        palette.border
    };
    painter.rect_filled(rect, 8, palette.surface);
    painter.rect_stroke(
        rect,
        8,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_center() + egui::vec2(14.0, 0.0),
        Align2::LEFT_CENTER,
        icons::MAGNIFYING_GLASS,
        icon_font(16.0),
        palette.text_weak,
    );
    painter.text(
        rect.left_center() + egui::vec2(40.0, 0.0),
        Align2::LEFT_CENTER,
        "Search tools by name or keyword",
        FontId::proportional(14.0),
        palette.text_weak,
    );
    painter.text(
        rect.right_center() - egui::vec2(14.0, 0.0),
        Align2::RIGHT_CENTER,
        shortcut,
        FontId::proportional(13.0),
        palette.text_weak,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn show_card_section(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    title: &str,
    ids: &[ToolId],
    actions: &mut Vec<ShellAction>,
) {
    if ids.is_empty() {
        return;
    }
    ui.add_space(10.0);
    show_section_label(ui, icon, title);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(CARD_GAP, CARD_GAP);
        for id in ids {
            let tool = id.meta();
            if show_tool_card(ui, tool).clicked() {
                actions.push(ShellAction::OpenTool(tool.id));
            }
        }
    });
}

fn show_tool_card(ui: &mut egui::Ui, tool: ToolMeta) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, tool.name));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let (fill, border) = if response.hovered() {
        (palette.surface_hover, palette.accent)
    } else {
        (palette.surface, palette.border)
    };
    painter.rect_filled(rect, 8, fill);
    painter.rect_stroke(
        rect,
        8,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    let icon_box = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(14.0, 14.0),
        egui::vec2(36.0, 36.0),
    );
    painter.rect_filled(icon_box, 8, palette.accent_soft);
    painter.text(
        icon_box.center(),
        Align2::CENTER_CENTER,
        tool.icon,
        icon_font(19.0),
        palette.accent,
    );

    let text_left = icon_box.right() + 12.0;
    let text_width = rect.right() - text_left - 12.0;
    painter.text(
        egui::pos2(text_left, rect.top() + 14.0),
        Align2::LEFT_TOP,
        tool.name,
        FontId::new(14.0, semibold_family()),
        palette.text_strong,
    );
    let summary = summary_job(tool.summary, text_width, palette.text_weak);
    let galley = painter.layout_job(summary);
    painter.galley(
        egui::pos2(text_left, rect.top() + 35.0),
        galley,
        palette.text_weak,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Up to two lines of small text, cut with an ellipsis when it does not fit.
fn summary_job(text: &str, max_width: f32, color: egui::Color32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(FontId::proportional(12.5), color),
    );
    job.wrap = TextWrapping {
        max_width,
        max_rows: 2,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    job
}
