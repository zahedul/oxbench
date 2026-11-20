use eframe::egui;

use super::ShellAction;
use crate::APP_NAME;
use crate::registry::{Category, ToolId, list_tools_in};
use crate::workspace::Workspace;

pub fn show_home(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    palette_shortcut: &str,
    actions: &mut Vec<ShellAction>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);
        ui.heading(APP_NAME);
        ui.label(format!(
            "Press {palette_shortcut} to search tools. Right-click a tool to add it to favorites."
        ));
        ui.add_space(12.0);
        show_tool_section(ui, "Favorites", &workspace.favorites, actions);
        show_tool_section(ui, "Recent", &workspace.recent, actions);
        ui.strong("All tools");
        for category in Category::ALL {
            ui.add_space(4.0);
            ui.weak(category.label());
            ui.horizontal_wrapped(|ui| {
                for tool in list_tools_in(category) {
                    if ui.button(tool.name).on_hover_text(tool.summary).clicked() {
                        actions.push(ShellAction::OpenTool(tool.id));
                    }
                }
            });
        }
    });
}

fn show_tool_section(
    ui: &mut egui::Ui,
    title: &str,
    ids: &[ToolId],
    actions: &mut Vec<ShellAction>,
) {
    if ids.is_empty() {
        return;
    }
    ui.strong(title);
    ui.horizontal_wrapped(|ui| {
        for id in ids {
            if ui.button(id.meta().name).clicked() {
                actions.push(ShellAction::OpenTool(*id));
            }
        }
    });
    ui.add_space(12.0);
}
