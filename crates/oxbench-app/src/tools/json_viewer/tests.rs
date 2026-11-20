//! Drives the JSON Viewer headlessly with clicks and typing.

use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::{JsonViewer, SAMPLE_JSON};
use crate::style::install_style;
use crate::tool::Tool;

fn open_viewer() -> Harness<'static, JsonViewer> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .build_ui_state(
            |ui, viewer: &mut JsonViewer| viewer.ui(ui),
            JsonViewer::default(),
        );
    harness.run();
    harness
}

fn open_viewer_with(input: &str) -> Harness<'static, JsonViewer> {
    let mut harness = open_viewer();
    harness.state_mut().input = input.to_owned();
    harness.state_mut().is_input_changed = true;
    harness.run();
    harness
}

/// Focuses the field first, as a click would; typed text goes to the focused widget.
fn type_into(harness: &mut Harness<'static, JsonViewer>, role: Role, text: &str) {
    harness.get_by_role(role).focus();
    harness.run();
    harness.get_by_role(role).type_text(text);
}

fn output_text(harness: &Harness<'_, JsonViewer>) -> String {
    harness.state().output.as_ref().cloned().unwrap_or_default()
}

#[test]
fn starts_with_a_hint() {
    let harness = open_viewer();
    harness.get_by_label("Paste JSON or a raw HTTP response on the left.");
}

#[test]
fn typing_valid_json_shows_formatted_output_and_stats() {
    let mut harness = open_viewer();
    type_into(
        &mut harness,
        Role::MultilineTextInput,
        r#"{"b":1,"a":[true,null]}"#,
    );
    harness.run();
    harness.get_by_label_contains("Valid JSON · 1 objects · 1 arrays · 3 values · depth 3");
    assert_eq!(
        output_text(&harness),
        "{\n  \"b\": 1,\n  \"a\": [\n    true,\n    null\n  ]\n}"
    );
}

#[test]
fn invalid_json_reports_line_and_column() {
    let harness = open_viewer_with("{\n  \"a\": 1,\n}");
    harness.get_by_label_contains("Invalid JSON. Line 3, column 1:");
    harness.get_by_label("Fix the JSON on the left to see the result.");
}

#[test]
fn sample_then_format_input_rewrites_the_input() {
    let mut harness = open_viewer();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    harness.get_by_label_contains("Minify").click();
    harness.run();
    assert!(!harness.state().input.contains('\n'));
    harness.get_by_label_contains("Format").click();
    harness.run();
    assert!(harness.state().input.starts_with("{\n  \"id\": 42,"));
}

#[test]
fn sort_keys_and_minified_change_the_output() {
    let mut harness = open_viewer_with(r#"{"b":1,"a":{"d":2,"c":3}}"#);
    harness.get_by_label("Sort keys").click();
    harness.run();
    harness.get_by_label("Minified").click();
    harness.run();
    assert_eq!(output_text(&harness), r#"{"a":{"c":3,"d":2},"b":1}"#);
}

#[test]
fn accepts_a_pasted_http_response() {
    let harness =
        open_viewer_with("HTTP/1.1 200 OK\nContent-Type: application/json\n\n{\"ok\":true}");
    harness.get_by_label_contains("Valid JSON");
}

#[test]
fn tree_shows_nodes_and_finds_values() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.get_by_label("Tree").click();
    harness.run();
    harness.get_by_label("roles  [2 items]");
    harness.get_by_label("\"Ada Lovelace\"");

    type_into(&mut harness, Role::TextInput, "london");
    harness.run();
    harness.get_by_label("1 match. Click the path to copy it.");
    harness.get_by_label("$.address.city");
}

#[test]
fn query_runs_jsonpath_filters() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.get_by_label("Query").click();
    harness.run();
    type_into(&mut harness, Role::TextInput, "$.orders[?@.total > 50].id");
    harness.run();
    harness.get_by_label("1 match");
}

#[test]
fn query_reports_an_invalid_expression() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.get_by_label("Query").click();
    harness.run();
    type_into(&mut harness, Role::TextInput, "$.orders[");
    harness.run();
    harness.get_by_label_contains("Invalid JSONPath");
}

#[test]
fn schema_lists_each_problem() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.state_mut().schema.schema_text =
        r#"{"type":"object","required":["id","phone"],"properties":{"id":{"type":"string"}}}"#
            .to_owned();
    harness.get_by_label("Schema").click();
    harness.run();
    harness.get_by_label("2 problems");
    harness.get_by_label("/id");
}

#[test]
fn schema_confirms_a_match() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.state_mut().schema.schema_text = r#"{"type":"object","required":["id"]}"#.to_owned();
    harness.get_by_label("Schema").click();
    harness.run();
    harness.get_by_label("The JSON matches the schema.");
}

#[test]
fn clear_empties_the_input() {
    let mut harness = open_viewer_with(SAMPLE_JSON);
    harness.get_by_label_contains("Clear").click();
    harness.run();
    assert_eq!(harness.state().input, "");
    harness.get_by_label("Paste JSON or a raw HTTP response on the left.");
}
