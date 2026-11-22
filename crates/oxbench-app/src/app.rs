use std::collections::HashMap;

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, ThemePreference};
use serde::{Deserialize, Serialize};

use crate::desktop::{Desktop, DesktopEvent, DesktopStatus};
use crate::registry::ToolId;
use crate::settings::Settings;
use crate::shell::ShellAction;
use crate::shell::command_palette::CommandPalette;
use crate::shell::home::show_home;
use crate::shell::settings_menu::{SettingsView, show_release_link};
use crate::shell::sidebar::show_sidebar;
use crate::shell::tabs::{TAB_BAR_HEIGHT, show_tab_bar};
use crate::style::install_style;
use crate::style::palette::Palette;
use crate::tool::Tool;
use crate::tools::create_tool;
use crate::update_check::{UpdateChecker, UpdateStatus};
use crate::widgets::card::show_card;
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
    settings: Settings,
}

pub struct OxbenchApp {
    workspace: Workspace,
    theme: ThemePreference,
    settings: Settings,
    tools: HashMap<ToolId, Box<dyn Tool>>,
    palette: CommandPalette,
    sidebar_filter: String,
    /// `None` in tests, which must not register shortcuts or icons on the machine running them.
    desktop: Option<Desktop>,
    desktop_status: DesktopStatus,
    updates: UpdateChecker,
}

impl OxbenchApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let saved: SavedState = creation_context
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        install_style(&creation_context.egui_ctx);
        creation_context.egui_ctx.set_theme(saved.theme);
        let mut workspace = saved.workspace;
        workspace.normalize();
        Self {
            workspace,
            theme: saved.theme,
            settings: saved.settings,
            tools: HashMap::new(),
            palette: CommandPalette::default(),
            sidebar_filter: String::new(),
            desktop: None,
            desktop_status: DesktopStatus::default(),
            updates: UpdateChecker::default(),
        }
    }

    /// The real app: adds the global shortcut and menu bar icon, and checks for updates.
    pub fn with_desktop(creation_context: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::new(creation_context);
        let ctx = &creation_context.egui_ctx;
        let mut desktop = Desktop::new(ctx);
        desktop.apply_settings(app.settings);
        app.desktop_status = desktop.status();
        app.desktop = Some(desktop);
        if app.settings.should_check_for_updates {
            app.updates.start(ctx);
        }
        app
    }

    fn collect_desktop_actions(&self, ctx: &egui::Context, actions: &mut Vec<ShellAction>) {
        let Some(desktop) = &self.desktop else {
            return;
        };
        for event in desktop.take_events() {
            match event {
                // The window was already raised when the event arrived.
                DesktopEvent::Show => {}
                DesktopEvent::OpenTool(id) => actions.push(ShellAction::OpenTool(id)),
                DesktopEvent::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }

    fn change_settings(&mut self, settings: Settings) {
        self.settings = settings;
        let Some(desktop) = &mut self.desktop else {
            return;
        };
        desktop.apply_settings(settings);
        self.desktop_status = desktop.status();
    }

    /// A newer release is announced at the foot of the sidebar until the app restarts.
    fn show_update_notice(&self, ui: &mut egui::Ui) {
        let Some(Ok(UpdateStatus::Available { version, page })) = self.updates.outcome() else {
            return;
        };
        egui::Panel::bottom("update_notice")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(0, 8)))
            .show(ui, |ui| {
                show_card(ui, "Update available", |ui| {
                    ui.label(format!("Version {version} is ready to download."));
                    show_release_link(ui, version, page);
                });
            });
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
            ShellAction::ChangeSettings(settings) => self.change_settings(settings),
            ShellAction::CheckForUpdates => self.updates.start(ctx),
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
        self.updates.poll();
        self.collect_desktop_actions(&ctx, &mut actions);
        self.collect_shortcut_actions(&ctx, &mut actions);

        let palette = Palette::of_context(&ctx);
        egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(232.0)
            .min_size(190.0)
            .frame(
                egui::Frame::default()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin::symmetric(10, 12)),
            )
            .show(ui, |ui| {
                self.show_update_notice(ui);
                show_sidebar(ui, &self.workspace, &mut self.sidebar_filter, &mut actions);
            });
        egui::Panel::top("tab_bar")
            .exact_size(TAB_BAR_HEIGHT)
            .frame(
                egui::Frame::default()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin {
                        left: 6,
                        right: 10,
                        top: 6,
                        bottom: 0,
                    }),
            )
            .show(ui, |ui| {
                let settings_view = SettingsView {
                    settings: self.settings,
                    desktop: &self.desktop_status,
                    updates: &self.updates,
                };
                show_tab_bar(
                    ui,
                    &self.workspace,
                    self.theme,
                    &palette_shortcut,
                    &settings_view,
                    &mut actions,
                );
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(palette.background)
                    .inner_margin(egui::Margin::symmetric(20, 16)),
            )
            .show(ui, |ui| {
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
            settings: self.settings,
        };
        eframe::set_value(storage, eframe::APP_KEY, &saved);
    }
}

#[cfg(test)]
mod ui_tests;
