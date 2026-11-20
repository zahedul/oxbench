use eframe::egui::{self, ThemePreference};

use super::ShellAction;
use super::theme::{next_theme, theme_label};
use crate::registry::ToolId;
use crate::workspace::Workspace;

pub fn show_tab_bar(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    theme: ThemePreference,
    palette_shortcut: &str,
    actions: &mut Vec<ShellAction>,
) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(workspace.active_tab.is_none(), "Home")
            .clicked()
        {
            actions.push(ShellAction::ShowHome);
        }
        ui.separator();
        for id in &workspace.open_tabs {
            show_tab(ui, workspace, *id, actions);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(theme_label(theme)).clicked() {
                actions.push(ShellAction::SetTheme(next_theme(theme)));
            }
            if ui.button(format!("Search  {palette_shortcut}")).clicked() {
                actions.push(ShellAction::OpenPalette);
            }
        });
    });
}

fn show_tab(ui: &mut egui::Ui, workspace: &Workspace, id: ToolId, actions: &mut Vec<ShellAction>) {
    let is_active = workspace.active_tab == Some(id);
    let response = ui.selectable_label(is_active, id.meta().name);
    if response.clicked() {
        actions.push(ShellAction::OpenTool(id));
    }
    if response.middle_clicked() {
        actions.push(ShellAction::CloseTool(id));
    }
    response.context_menu(|ui| {
        if ui.button("Close").clicked() {
            actions.push(ShellAction::CloseTool(id));
        }
        let favorite_label = if workspace.is_favorite(id) {
            "Remove from favorites"
        } else {
            "Add to favorites"
        };
        if ui.button(favorite_label).clicked() {
            actions.push(ShellAction::ToggleFavorite(id));
        }
    });
    if ui.small_button("×").on_hover_text("Close tab").clicked() {
        actions.push(ShellAction::CloseTool(id));
    }
}
