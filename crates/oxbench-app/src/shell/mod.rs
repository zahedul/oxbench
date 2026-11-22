//! The app frame around the tools. Each part reports what the user did as a `ShellAction`,
//! and `OxbenchApp` applies them in one place.

pub mod command_palette;
pub mod home;
pub mod settings_menu;
pub mod sidebar;
pub mod tabs;
pub mod theme;

use eframe::egui::ThemePreference;

use crate::registry::ToolId;
use crate::settings::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellAction {
    OpenTool(ToolId),
    CloseTool(ToolId),
    ShowHome,
    ToggleFavorite(ToolId),
    OpenPalette,
    SetTheme(ThemePreference),
    ChangeSettings(Settings),
    CheckForUpdates,
}
