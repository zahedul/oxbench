//! The gear button in the tab bar and its menu of desktop and update preferences.

use eframe::egui::{self, PopupCloseBehavior, RectAlign};

use super::ShellAction;
use crate::APP_NAME;
use crate::desktop::{DesktopStatus, IS_MENU_BAR_SUPPORTED, shortcut_label};
use crate::settings::Settings;
use crate::style::fonts::semibold_family;
use crate::style::icons;
use crate::style::palette::Palette;
use crate::update_check::{CURRENT_VERSION, UpdateChecker, UpdateStatus};
use crate::widgets::icon_button::{icon_label_button, show_icon_button};
use crate::widgets::status::{StatusTone, show_status};

const MENU_WIDTH: f32 = 320.0;

#[cfg(target_os = "macos")]
const MENU_BAR_LABEL: &str = "Show in the menu bar";
#[cfg(not(target_os = "macos"))]
const MENU_BAR_LABEL: &str = "Show in the system tray";

/// Everything the menu reads; changes come back as `ShellAction`s.
pub struct SettingsView<'a> {
    pub settings: Settings,
    pub desktop: &'a DesktopStatus,
    pub updates: &'a UpdateChecker,
}

pub fn show_settings_button(
    ui: &mut egui::Ui,
    view: &SettingsView<'_>,
    actions: &mut Vec<ShellAction>,
) {
    let response = show_icon_button(ui, icons::GEAR_SIX, "Settings");
    egui::Popup::menu(&response)
        .align(RectAlign::BOTTOM_END)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(MENU_WIDTH)
        .show(|ui| {
            ui.set_width(MENU_WIDTH);
            ui.spacing_mut().item_spacing.y = 6.0;
            show_desktop_section(ui, view, actions);
            ui.separator();
            show_updates_section(ui, view, actions);
        });
}

fn show_heading(ui: &mut egui::Ui, text: &str) {
    let palette = Palette::of(ui);
    ui.label(
        egui::RichText::new(text)
            .family(semibold_family())
            .color(palette.text_strong),
    );
}

fn show_desktop_section(
    ui: &mut egui::Ui,
    view: &SettingsView<'_>,
    actions: &mut Vec<ShellAction>,
) {
    show_heading(ui, "Desktop");
    let mut settings = view.settings;
    let shortcut = shortcut_label(ui.ctx());
    ui.checkbox(
        &mut settings.is_global_shortcut_enabled,
        format!("Global shortcut {shortcut}"),
    );
    ui.weak(format!("Brings {APP_NAME} to the front from any app."));
    if let Some(problem) = &view.desktop.shortcut_problem {
        show_status(ui, StatusTone::Warning, problem);
    }
    if IS_MENU_BAR_SUPPORTED {
        ui.checkbox(&mut settings.should_show_menu_bar_icon, MENU_BAR_LABEL);
        if let Some(problem) = &view.desktop.menu_bar_problem {
            show_status(ui, StatusTone::Warning, problem);
        }
    }
    if settings != view.settings {
        actions.push(ShellAction::ChangeSettings(settings));
    }
}

fn show_updates_section(
    ui: &mut egui::Ui,
    view: &SettingsView<'_>,
    actions: &mut Vec<ShellAction>,
) {
    show_heading(ui, "Updates");
    let mut settings = view.settings;
    ui.checkbox(
        &mut settings.should_check_for_updates,
        format!("Check for updates when {APP_NAME} starts"),
    );
    if settings != view.settings {
        actions.push(ShellAction::ChangeSettings(settings));
    }
    ui.horizontal(|ui| {
        ui.weak(format!("Version {CURRENT_VERSION}"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let is_checking = view.updates.is_running();
            let button = icon_label_button(icons::ARROWS_CLOCKWISE, "Check now");
            if ui.add_enabled(!is_checking, button).clicked() {
                actions.push(ShellAction::CheckForUpdates);
            }
            if is_checking {
                ui.spinner();
            }
        });
    });
    show_update_outcome(ui, view.updates);
}

fn show_update_outcome(ui: &mut egui::Ui, updates: &UpdateChecker) {
    match updates.outcome() {
        None => {}
        Some(Ok(UpdateStatus::UpToDate)) => {
            show_status(
                ui,
                StatusTone::Success,
                &format!("{APP_NAME} is up to date."),
            );
        }
        Some(Ok(UpdateStatus::NoReleases)) => {
            show_status(
                ui,
                StatusTone::Neutral,
                "No releases have been published yet.",
            );
        }
        Some(Ok(UpdateStatus::Available { version, page })) => {
            show_status(
                ui,
                StatusTone::Success,
                &format!("Version {version} is available."),
            );
            show_release_link(ui, version, page);
        }
        Some(Err(message)) => show_status(ui, StatusTone::Warning, &message),
    }
}

/// Opens the release page; the download itself happens in the browser.
pub fn show_release_link(ui: &mut egui::Ui, version: &str, page: &str) {
    let button = icon_label_button(icons::ARROW_SQUARE_OUT, &format!("Get {version}"));
    if ui.add(button).on_hover_text(page).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(page));
    }
}
