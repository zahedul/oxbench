use eframe::egui;

use super::ShellAction;
use crate::registry::{Category, ToolMeta, list_tools_in, search_tools};
use crate::workspace::Workspace;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    filter: &mut String,
    actions: &mut Vec<ShellAction>,
) {
    ui.add_space(6.0);
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text("Filter tools")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        if !filter.trim().is_empty() {
            show_filtered_tools(ui, workspace, filter, actions);
            return;
        }
        show_favorites(ui, workspace, actions);
        for category in Category::ALL {
            egui::CollapsingHeader::new(category.label())
                .default_open(true)
                .show(ui, |ui| {
                    for tool in list_tools_in(category) {
                        show_tool_row(ui, workspace, tool, actions);
                    }
                });
        }
    });
}

fn show_filtered_tools(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    filter: &str,
    actions: &mut Vec<ShellAction>,
) {
    let results = search_tools(filter);
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
    egui::CollapsingHeader::new("Favorites")
        .default_open(true)
        .show(ui, |ui| {
            for id in &workspace.favorites {
                show_tool_row(ui, workspace, id.meta(), actions);
            }
        });
}

fn show_tool_row(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    tool: ToolMeta,
    actions: &mut Vec<ShellAction>,
) {
    let is_active = workspace.active_tab == Some(tool.id);
    let is_favorite = workspace.is_favorite(tool.id);
    let response = ui
        .add_sized(
            [ui.available_width(), 22.0],
            egui::Button::selectable(is_active, tool.name),
        )
        .on_hover_text(tool.summary);
    if response.clicked() {
        actions.push(ShellAction::OpenTool(tool.id));
    }
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
