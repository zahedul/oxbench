//! Drives the real app headlessly: clicks, key presses and typed text, as a user would.

use eframe::egui::{Event, Key, Modifiers, ThemePreference};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::OxbenchApp;
use crate::registry::ToolId;

fn launch() -> Harness<'static, OxbenchApp> {
    let mut harness = Harness::new_eframe(|creation_context| OxbenchApp::new(creation_context));
    harness.run();
    harness
}

/// The sidebar and the home screen both list every tool, so a label can appear more than once.
fn click_first(harness: &Harness<'_, OxbenchApp>, label: &str) {
    harness
        .get_all_by_label(label)
        .next()
        .unwrap_or_else(|| panic!("no widget labelled {label:?}"))
        .click();
}

fn active_tab(harness: &Harness<'_, OxbenchApp>) -> Option<ToolId> {
    harness.state().workspace.active_tab
}

#[test]
fn starts_on_the_home_screen() {
    let harness = launch();
    assert_eq!(active_tab(&harness), None);
    harness.get_by_label("All tools");
}

#[test]
fn clicking_a_tool_opens_it_in_a_tab() {
    let mut harness = launch();
    click_first(&harness, "Epoch Converter");
    harness.run();
    assert_eq!(active_tab(&harness), Some(ToolId::EpochConverter));
    assert_eq!(
        harness.state().workspace.open_tabs,
        [ToolId::EpochConverter]
    );
    harness.get_by_label("This tool is planned and not built yet.");
}

#[test]
fn command_palette_opens_the_chosen_tool() {
    let mut harness = launch();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert!(harness.state().palette.is_open());

    harness.event(Event::Text("jwt".to_owned()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();

    assert!(!harness.state().palette.is_open());
    assert_eq!(active_tab(&harness), Some(ToolId::JwtDecoder));
}

#[test]
fn command_palette_arrow_keys_move_the_selection() {
    let mut harness = launch();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(active_tab(&harness), Some(ToolId::ALL[1]));
}

#[test]
fn escape_closes_the_command_palette_without_opening_a_tool() {
    let mut harness = launch();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().palette.is_open());
    assert_eq!(active_tab(&harness), None);
}

#[test]
fn close_tab_shortcut_returns_home() {
    let mut harness = launch();
    harness
        .state_mut()
        .workspace
        .open_tool(ToolId::HashGenerator);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
    harness.run();
    assert_eq!(active_tab(&harness), None);
    assert_eq!(harness.state().workspace.open_tabs, []);
}

#[test]
fn favorite_shortcut_opens_the_favorite() {
    let mut harness = launch();
    harness
        .state_mut()
        .workspace
        .toggle_favorite(ToolId::CidrCalculator);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    harness.run();
    assert_eq!(active_tab(&harness), Some(ToolId::CidrCalculator));
}

#[test]
fn switching_tabs_keeps_both_open() {
    let mut harness = launch();
    harness.state_mut().workspace.open_tool(ToolId::JsonViewer);
    harness.state_mut().workspace.open_tool(ToolId::Bcrypt);
    harness.run();
    click_first(&harness, "JSON Viewer");
    harness.run();
    assert_eq!(active_tab(&harness), Some(ToolId::JsonViewer));
    assert_eq!(
        harness.state().workspace.open_tabs,
        [ToolId::JsonViewer, ToolId::Bcrypt]
    );
}

#[test]
fn theme_button_cycles_the_theme() {
    let mut harness = launch();
    harness.get_by_label("Theme: System").click();
    harness.run();
    assert_eq!(harness.state().theme, ThemePreference::Light);
    harness.get_by_label("Theme: Light");
}
