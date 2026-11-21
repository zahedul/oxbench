//! Drives Text Compare headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::{Pane, TextCompare, sample_input};
use crate::style::install_style;
use crate::tool::Tool;

fn open_compare() -> Harness<'static, TextCompare> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, compare: &mut TextCompare| compare.ui(ui),
            TextCompare::default(),
        );
    harness.run();
    harness
}

fn set_texts(harness: &mut Harness<'static, TextCompare>, original: &str, changed: &str) {
    let compare = harness.state_mut();
    compare.original = sample_input(original);
    compare.changed = sample_input(changed);
    harness.run();
}

fn patch(harness: &Harness<'static, TextCompare>) -> String {
    harness
        .state()
        .result
        .as_ref()
        .map_or_else(String::new, |result| result.patch.clone())
}

fn numbered_lines(count: usize) -> String {
    (1..=count)
        .flat_map(|number| ["line ".to_owned(), number.to_string(), "\n".to_owned()])
        .collect()
}

#[test]
fn starts_empty_with_a_hint() {
    let harness = open_compare();
    harness.get_by_label("Paste or open two texts to see what changed.");
}

#[test]
fn sample_shows_each_kind_of_change() {
    let mut harness = open_compare();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    harness.get_by_label("5 changed");
    harness.get_by_label("1 removed");
    harness.get_by_label("1 added");
    harness.get_by_label("Removed: timeout = 30");
    harness.get_by_label("Added: file = \"/var/log/app.log\"");
    harness.get_by_label("Changed: workers = 4 → workers = 8");
    assert!(patch(&harness).starts_with("--- original\n+++ changed\n@@ -1,13 +1,13 @@\n"));
}

#[test]
fn unified_layout_lists_removed_lines_then_added_ones() {
    let mut harness = open_compare();
    set_texts(&mut harness, "keep\nold\nkeep", "keep\nnew\nkeep");
    harness.get_by_label("Unified").click();
    harness.run();
    let removed = harness.get_by_label("Removed: old").rect();
    let added = harness.get_by_label("Added: new").rect();
    assert!(removed.bottom() <= added.top());
}

#[test]
fn identical_texts_say_so() {
    let mut harness = open_compare();
    set_texts(&mut harness, "same\ntext", "same\ntext");
    harness.get_by_label("The texts are identical.");
    assert_eq!(patch(&harness), "");
}

#[test]
fn ignoring_case_hides_case_only_changes() {
    let mut harness = open_compare();
    set_texts(&mut harness, "Hello World", "hello world");
    harness.get_by_label("1 changed");
    harness.get_by_label("Ignore case").click();
    harness.run();
    harness.get_by_label("No differences, ignoring case.");
}

#[test]
fn swap_exchanges_the_texts() {
    let mut harness = open_compare();
    set_texts(&mut harness, "a", "b");
    harness.get_by_label_contains("Swap").click();
    harness.run();
    assert_eq!(harness.state().original.text, "b");
    assert_eq!(harness.state().changed.text, "a");
    harness.get_by_label("Changed: b → a");
}

#[test]
fn clicking_a_fold_shows_every_line() {
    let mut harness = open_compare();
    let original = numbered_lines(40);
    let changed = original.replace("line 20\n", "line twenty\n");
    set_texts(&mut harness, &original, &changed);
    harness.get_by_label("Show 16 unchanged lines").click();
    harness.run();
    assert!(!harness.state().should_fold_unchanged);
    harness.get_by_label("Unchanged: line 1");
}

#[test]
fn clear_empties_both_sides() {
    let mut harness = open_compare();
    set_texts(&mut harness, "a", "b");
    harness.get_by_label_contains("Clear").click();
    harness.run();
    assert_eq!(harness.state().original.text, "");
    assert_eq!(harness.state().changed.text, "");
    harness.get_by_label("Paste or open two texts to see what changed.");
}

#[test]
fn loaded_files_name_the_patch_and_bad_files_explain_why() {
    let directory = std::env::temp_dir().join(format!("oxbench-compare-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let text_path = directory.join("before.txt");
    let binary_path = directory.join("image.bin");
    std::fs::write(&text_path, "old\n").unwrap();
    std::fs::write(&binary_path, [0xFF, 0xFE, 0x00]).unwrap();

    let mut harness = open_compare();
    harness.state_mut().load_file(Pane::Original, &text_path);
    harness.state_mut().changed = sample_input("new\n");
    harness.run();
    assert!(patch(&harness).starts_with("--- before.txt\n+++ changed\n"));
    harness.get_by_label("before.txt");

    harness.state_mut().load_file(Pane::Changed, &binary_path);
    harness.run();
    harness.get_by_label("image.bin isn't UTF-8 text, so it can't be compared.");
    assert_eq!(harness.state().changed.text, "new\n");
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_dropped_file_goes_to_the_pane_under_the_pointer_or_the_empty_one() {
    let mut harness = open_compare();
    let changed_rect = harness.state().pane_rects[1].unwrap();
    let compare = harness.state_mut();
    assert_eq!(
        compare.choose_drop_target(Some(changed_rect.center())),
        Pane::Changed
    );
    assert_eq!(compare.choose_drop_target(None), Pane::Original);
    compare.original = sample_input("filled");
    assert_eq!(compare.choose_drop_target(None), Pane::Changed);
}
