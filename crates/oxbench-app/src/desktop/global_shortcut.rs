//! A system-wide shortcut that brings Oxbench to the front from any app.

use std::sync::mpsc::Sender;

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use global_hotkey::hotkey::{Code, HotKey, Modifiers as HotKeyModifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use super::{DesktopEvent, deliver};

#[cfg(target_os = "macos")]
const SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Space);
#[cfg(target_os = "macos")]
const HOTKEY_MODIFIERS: HotKeyModifiers = HotKeyModifiers::META.union(HotKeyModifiers::SHIFT);

// Ctrl+Shift+Space is taken by many editors on Windows and Linux, so Alt is used instead.
#[cfg(not(target_os = "macos"))]
const SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::CTRL.plus(Modifiers::ALT), Key::Space);
#[cfg(not(target_os = "macos"))]
const HOTKEY_MODIFIERS: HotKeyModifiers = HotKeyModifiers::CONTROL.union(HotKeyModifiers::ALT);

/// The shortcut as the platform writes it, such as "⇧⌘Space".
pub fn shortcut_label(ctx: &egui::Context) -> String {
    ctx.format_shortcut(&SHORTCUT)
}

pub struct GlobalShortcut {
    manager: Result<GlobalHotKeyManager, String>,
    hotkey: HotKey,
    is_registered: bool,
    problem: Option<String>,
}

impl GlobalShortcut {
    pub fn new(ctx: &egui::Context, sender: Sender<DesktopEvent>) -> Self {
        let hotkey = HotKey::new(Some(HOTKEY_MODIFIERS), Code::Space);
        let ctx = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.id() == hotkey.id() && event.state() == HotKeyState::Pressed {
                deliver(&ctx, &sender, DesktopEvent::Show);
            }
        }));
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| format!("Global shortcuts aren't available here: {error}"));
        Self {
            manager,
            hotkey,
            is_registered: false,
            problem: None,
        }
    }

    pub fn set_enabled(&mut self, should_enable: bool) {
        if should_enable == self.is_registered {
            return;
        }
        let manager = match &self.manager {
            Ok(manager) => manager,
            Err(message) => {
                self.problem = should_enable.then(|| message.clone());
                return;
            }
        };
        let result = if should_enable {
            manager.register(self.hotkey)
        } else {
            manager.unregister(self.hotkey)
        };
        match result {
            Ok(()) => {
                self.is_registered = should_enable;
                self.problem = None;
            }
            Err(error) if should_enable => {
                self.problem = Some(format!(
                    "The shortcut couldn't be set, so another app may be using it: {error}"
                ));
            }
            Err(error) => {
                self.problem = Some(format!("The shortcut couldn't be removed: {error}"));
            }
        }
    }

    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }
}
