use std::collections::HashMap;

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, ThemePreference};
use serde::{Deserialize, Serialize};

use crate::registry::ToolId;
use crate::shell::ShellAction;
use crate::shell::command_palette::CommandPalette;
use crate::shell::home::show_home;
use crate::shell::sidebar::show_sidebar;
use crate::shell::tabs::show_tab_bar;
use crate::tool::Tool;
use crate::tools::create_tool;
use crate::workspace::Workspace;

const PALETTE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
const CLOSE_TAB_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::W);
const FAVORITE_KEYS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SavedState {
    workspace: Workspace,
    theme: ThemePreference,
}

pub struct OxbenchApp {
    workspace: Workspace,
    theme: ThemePreference,
    tools: HashMap<ToolId, Box<dyn Tool>>,
    palette: CommandPalette,
    sidebar_filter: String,
}

impl OxbenchApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let saved: SavedState = creation_context
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        creation_context.egui_ctx.set_theme(saved.theme);
        let mut workspace = saved.workspace;
        workspace.normalize();
        Self {
            workspace,
            theme: saved.theme,
            tools: HashMap::new(),
            palette: CommandPalette::default(),
            sidebar_filter: String::new(),
        }
    }

    fn collect_shortcut_actions(&self, ctx: &egui::Context, actions: &mut Vec<ShellAction>) {
        if self.palette.is_open() {
            return;
        }
        ctx.input_mut(|input| {
            if input.consume_shortcut(&PALETTE_SHORTCUT) {
                actions.push(ShellAction::OpenPalette);
            }
            if input.consume_shortcut(&CLOSE_TAB_SHORTCUT)
                && let Some(active) = self.workspace.active_tab
            {
                actions.push(ShellAction::CloseTool(active));
            }
            for (key, favorite) in FAVORITE_KEYS.iter().zip(&self.workspace.favorites) {
                if input.consume_key(Modifiers::COMMAND, *key) {
                    actions.push(ShellAction::OpenTool(*favorite));
                }
            }
        });
    }

    fn apply_action(&mut self, ctx: &egui::Context, action: ShellAction) {
        match action {
            ShellAction::OpenTool(id) => self.workspace.open_tool(id),
            ShellAction::CloseTool(id) => {
                if self.workspace.close_tool(id) {
                    self.tools.remove(&id);
                }
            }
            ShellAction::ShowHome => self.workspace.show_home(),
            ShellAction::ToggleFavorite(id) => {
                // Full favorites are visible in the sidebar, so a refused add needs no message.
                self.workspace.toggle_favorite(id);
            }
            ShellAction::OpenPalette => self.palette.open(),
            ShellAction::SetTheme(theme) => {
                self.theme = theme;
                ctx.set_theme(theme);
            }
        }
    }

    fn show_active_tool(
        &mut self,
        ui: &mut egui::Ui,
        palette_shortcut: &str,
        actions: &mut Vec<ShellAction>,
    ) {
        let Some(id) = self.workspace.active_tab else {
            show_home(ui, &self.workspace, palette_shortcut, actions);
            return;
        };
        let tool = self.tools.entry(id).or_insert_with(|| create_tool(id));
        tool.ui(ui);
    }
}

impl eframe::App for OxbenchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let palette_shortcut = ctx.format_shortcut(&PALETTE_SHORTCUT);
        let mut actions = Vec::new();
        self.collect_shortcut_actions(&ctx, &mut actions);

        egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(220.0)
            .min_size(170.0)
            .show(ui, |ui| {
                show_sidebar(ui, &self.workspace, &mut self.sidebar_filter, &mut actions);
            });
        egui::Panel::top("tab_bar").show(ui, |ui| {
            show_tab_bar(
                ui,
                &self.workspace,
                self.theme,
                &palette_shortcut,
                &mut actions,
            );
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_active_tool(ui, &palette_shortcut, &mut actions);
        });
        self.palette.show(&ctx, &mut actions);

        for action in actions {
            self.apply_action(&ctx, action);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let saved = SavedState {
            workspace: self.workspace.clone(),
            theme: self.theme,
        };
        eframe::set_value(storage, eframe::APP_KEY, &saved);
    }
}

#[cfg(test)]
mod ui_tests;
