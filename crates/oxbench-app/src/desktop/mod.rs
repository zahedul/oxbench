//! Ways to reach Oxbench from outside its window: a global shortcut and a menu bar icon.
//! Both report through `DesktopEvent`s that the app applies on its next frame.

mod global_shortcut;
mod menu_bar;

use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;

use crate::registry::ToolId;
use crate::settings::Settings;
use global_shortcut::GlobalShortcut;
use menu_bar::MenuBarIcon;

pub use global_shortcut::shortcut_label;
pub use menu_bar::IS_MENU_BAR_SUPPORTED;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopEvent {
    Show,
    OpenTool(ToolId),
    Quit,
}

/// What the settings menu shows about the desktop integrations.
#[derive(Clone, Debug, Default)]
pub struct DesktopStatus {
    pub shortcut_problem: Option<String>,
    pub menu_bar_problem: Option<String>,
}

pub struct Desktop {
    receiver: Receiver<DesktopEvent>,
    shortcut: GlobalShortcut,
    menu_bar: MenuBarIcon,
}

impl Desktop {
    /// Call on the main thread once the event loop is running, as macOS requires.
    pub fn new(ctx: &egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            receiver,
            shortcut: GlobalShortcut::new(ctx, sender.clone()),
            menu_bar: MenuBarIcon::new(ctx, sender),
        }
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        self.shortcut
            .set_enabled(settings.is_global_shortcut_enabled);
        self.menu_bar
            .set_visible(settings.should_show_menu_bar_icon);
    }

    pub fn take_events(&self) -> Vec<DesktopEvent> {
        self.receiver.try_iter().collect()
    }

    pub fn status(&self) -> DesktopStatus {
        DesktopStatus {
            shortcut_problem: self.shortcut.problem().map(str::to_owned),
            menu_bar_problem: self.menu_bar.problem().map(str::to_owned),
        }
    }
}

/// Raises the window, restoring it first when minimised.
fn bring_to_front(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

/// Sends `event` and wakes the app so it is handled straight away.
fn deliver(ctx: &egui::Context, sender: &Sender<DesktopEvent>, event: DesktopEvent) {
    // Sending fails only while the app is shutting down, when the event no longer matters.
    if sender.send(event).is_err() {
        return;
    }
    if event == DesktopEvent::Quit {
        ctx.request_repaint();
        return;
    }
    bring_to_front(ctx);
}
