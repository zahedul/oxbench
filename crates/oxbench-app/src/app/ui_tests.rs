//! Drives the real app headlessly: clicks, key presses and typed text, as a user would.

use eframe::egui::{self, Event, Key, Modifiers, ThemePreference};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

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
    harness.get_by_label("Search all tools");
}

#[test]
fn the_active_tab_stays_in_view_when_many_are_open() {
    let mut harness = launch();
    for id in ToolId::ALL {
        harness.state_mut().workspace.open_tool(id);
    }
    harness.run();
    let last = ToolId::ALL[ToolId::ALL.len() - 1].meta().name;
    let tab_right = harness
        .get_all_by_role(egui::accesskit::Role::Tab)
        .find(|node| node.accesskit_node().label().as_deref() == Some(last))
        .unwrap_or_else(|| panic!("no tab for {last}"))
        .rect()
        .right();
    let search_left = harness.get_by_label("Search tools").rect().left();
    assert!(
        tab_right <= search_left,
        "the {last} tab ends at {tab_right}, under the search box at {search_left}"
    );
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
    harness.get_by_label("Current Unix time");
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

/// Renders screenshots for visual review. Run explicitly:
/// `OXBENCH_SCREENSHOT_DIR=/some/dir cargo test -p oxbench-app render_screenshots -- --ignored`
#[test]
#[ignore = "writes PNG files; run on demand for visual review"]
fn render_screenshots() {
    let output_dir =
        std::env::var("OXBENCH_SCREENSHOT_DIR").unwrap_or_else(|_| "target/screenshots".to_owned());
    std::fs::create_dir_all(&output_dir).unwrap();
    for (theme, theme_name) in [(egui::Theme::Dark, "dark"), (egui::Theme::Light, "light")] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .with_theme(theme)
            .build_eframe(|creation_context| OxbenchApp::new(creation_context));
        harness
            .state_mut()
            .workspace
            .toggle_favorite(ToolId::JsonViewer);
        harness.state_mut().workspace.open_tool(ToolId::JwtDecoder);
        harness.state_mut().workspace.show_home();
        harness.run();
        save_render(&mut harness, &output_dir, &format!("home-{theme_name}"));

        harness.state_mut().workspace.open_tool(ToolId::JsonViewer);
        harness.run();
        harness.get_by_label_contains("Sample").click();
        harness.run();
        save_render(
            &mut harness,
            &output_dir,
            &format!("json-text-{theme_name}"),
        );

        harness.get_by_label("Tree").click();
        harness.run();
        save_render(
            &mut harness,
            &output_dir,
            &format!("json-tree-{theme_name}"),
        );

        harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
        harness.run();
        harness.event(Event::Text("ke".to_owned()));
        harness.run();
        save_render(&mut harness, &output_dir, &format!("palette-{theme_name}"));

        harness.key_press(Key::Escape);
        harness
            .state_mut()
            .workspace
            .open_tool(ToolId::EpochConverter);
        harness.run();
        save_render(&mut harness, &output_dir, &format!("epoch-{theme_name}"));

        harness.state_mut().workspace.open_tool(ToolId::JwtDecoder);
        harness.run();
        harness.get_by_label_contains("Sample").click();
        harness.run();
        save_render(&mut harness, &output_dir, &format!("jwt-{theme_name}"));
        harness
            .state_mut()
            .workspace
            .open_tool(ToolId::EpochConverter);
        harness.run();

        harness.get_by_label("Time zone").click();
        harness.run();
        save_render(
            &mut harness,
            &output_dir,
            &format!("epoch-zones-{theme_name}"),
        );
        harness.key_press(Key::Escape);

        let generators = [
            (ToolId::HashGenerator, "hash"),
            (ToolId::IdGenerator, "ids"),
            (ToolId::RandomString, "random"),
            (ToolId::PasswordGenerator, "password"),
            (ToolId::Bcrypt, "bcrypt"),
        ];
        for (tool, name) in generators {
            harness.state_mut().workspace.open_tool(tool);
            harness.run();
            save_render(&mut harness, &output_dir, &format!("{name}-{theme_name}"));
        }

        render_compare_and_network_tools(&mut harness, &output_dir, theme_name);
        render_step_eight_tools(&mut harness, &output_dir, theme_name);
    }
}

fn render_compare_and_network_tools(
    harness: &mut Harness<'_, OxbenchApp>,
    output_dir: &str,
    theme_name: &str,
) {
    harness.state_mut().workspace.open_tool(ToolId::TextCompare);
    harness.run();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    save_render(harness, output_dir, &format!("compare-side-{theme_name}"));
    harness.get_by_label("Unified").click();
    harness.run();
    save_render(
        harness,
        output_dir,
        &format!("compare-unified-{theme_name}"),
    );

    harness
        .state_mut()
        .workspace
        .open_tool(ToolId::CidrCalculator);
    harness.run();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    save_render(harness, output_dir, &format!("cidr-subnet-{theme_name}"));
    for (view, name) in [("Split", "split"), ("Merge", "merge")] {
        harness.get_by_label(view).click();
        harness.run();
        if view == "Merge" {
            harness.get_by_label_contains("Sample").click();
            harness.run();
        }
        save_render(harness, output_dir, &format!("cidr-{name}-{theme_name}"));
    }
}

fn render_step_eight_tools(
    harness: &mut Harness<'_, OxbenchApp>,
    output_dir: &str,
    theme_name: &str,
) {
    harness.state_mut().workspace.open_tool(ToolId::QrGenerator);
    harness.run();
    harness
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.event(Event::Text("https://example.com/oxbench".to_owned()));
    harness.run();
    save_render(harness, output_dir, &format!("qr-{theme_name}"));

    harness.state_mut().workspace.open_tool(ToolId::LoremIpsum);
    harness.run();
    save_render(harness, output_dir, &format!("lorem-{theme_name}"));

    harness.state_mut().workspace.open_tool(ToolId::KeyPair);
    for _ in 0..50 {
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    harness.run();
    save_render(harness, output_dir, &format!("keypair-{theme_name}"));
}

fn save_render(harness: &mut Harness<'_, OxbenchApp>, output_dir: &str, name: &str) {
    let image = harness.render().unwrap();
    image.save(format!("{output_dir}/{name}.png")).unwrap();
}
