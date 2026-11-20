use eframe::egui;

use super::ShellAction;
use crate::APP_NAME;
use crate::registry::{Category, ToolMeta, list_tools_in, search_tools};
use crate::style::fonts::semibold_family;
use crate::style::icons::{self, icon_text};
use crate::style::palette::Palette;
use crate::widgets::nav_row::{show_nav_row, show_section_label};
use crate::widgets::search_field::show_search_field;
use crate::workspace::Workspace;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    filter: &mut String,
    actions: &mut Vec<ShellAction>,
) {
    show_brand(ui);
    ui.add_space(10.0);
    show_search_field(ui, filter, "Filter tools", 14.0);
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            if !filter.trim().is_empty() {
                show_filtered_tools(ui, workspace, filter, actions);
                return;
            }
            show_favorites(ui, workspace, actions);
            for category in Category::ALL {
                show_section_label(ui, None, category.label());
                for tool in list_tools_in(category) {
                    show_tool_row(ui, workspace, tool, actions);
                }
            }
        });
}

fn show_brand(ui: &mut egui::Ui) {
    let palette = Palette::of(ui);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(icon_text(icons::CODE).size(20.0).color(palette.accent));
        ui.label(
            egui::RichText::new(APP_NAME)
                .family(semibold_family())
                .size(17.0)
                .color(palette.text_strong),
        );
    });
}

fn show_filtered_tools(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    filter: &str,
    actions: &mut Vec<ShellAction>,
) {
    let results = search_tools(filter);
    show_section_label(ui, None, "Results");
    if results.is_empty() {
        ui.weak("No tools match.");
        return;
    }
    for tool in results {
        show_tool_row(ui, workspace, tool, actions);
    }
}

fn show_favorites(ui: &mut egui::Ui, workspace: &Workspace, actions: &mut Vec<ShellAction>) {
    if workspace.favorites.is_empty() {
        return;
    }
    show_section_label(ui, Some(icons::STAR), "Favorites");
    for id in &workspace.favorites {
        show_tool_row(ui, workspace, id.meta(), actions);
    }
}

fn show_tool_row(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    tool: ToolMeta,
    actions: &mut Vec<ShellAction>,
) {
    let is_active = workspace.active_tab == Some(tool.id);
    let response = show_nav_row(ui, tool.icon, tool.name, is_active).on_hover_text(tool.summary);
    if response.clicked() {
        actions.push(ShellAction::OpenTool(tool.id));
    }
    let is_favorite = workspace.is_favorite(tool.id);
    response.context_menu(|ui| {
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
