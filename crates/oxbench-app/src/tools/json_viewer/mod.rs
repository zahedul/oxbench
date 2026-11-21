//! JSON Viewer: input on the left; formatted text, tree, `JSONPath` query or schema check on the right.

mod query;
mod schema;
mod tree;

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::json::{
    FormatOptions, Indent, Value, format_minified, format_pretty, measure_json, parse_json,
};
use oxbench_core::{CoreError, CoreResult};

use crate::style::icons;
use crate::style::palette::Palette;
use crate::tool::Tool;
use crate::widgets::code_editor::{show_code_editor, show_code_view};
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::view_tabs::show_view_tabs;
use query::QueryPanel;
use schema::SchemaPanel;
use tree::TreeView;

const SAMPLE_JSON: &str = r#"{
  "id": 42,
  "name": "Ada Lovelace",
  "email": "ada@example.com",
  "active": true,
  "roles": ["admin", "editor"],
  "address": { "city": "London", "postcode": "W1" },
  "orders": [
    { "id": 1, "total": 19.99, "items": 2 },
    { "id": 2, "total": 120.50, "items": 7 }
  ],
  "manager": null
}"#;

const STATUS_LINE_HEIGHT: f32 = 30.0;
const MIN_BODY_HEIGHT: f32 = 120.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum JsonView {
    #[default]
    Text,
    Tree,
    Query,
    Schema,
}

impl JsonView {
    const ALL: [JsonView; 4] = [
        JsonView::Text,
        JsonView::Tree,
        JsonView::Query,
        JsonView::Schema,
    ];

    const fn label(self) -> &'static str {
        match self {
            JsonView::Text => "Text",
            JsonView::Tree => "Tree",
            JsonView::Query => "Query",
            JsonView::Schema => "Schema",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OutputStyle {
    #[default]
    Pretty,
    Minified,
}

pub struct JsonViewer {
    input: String,
    view: JsonView,
    format_options: FormatOptions,
    output_style: OutputStyle,
    /// `None` while the input is blank.
    document: Option<CoreResult<Value>>,
    /// Bumped on every re-parse, so the views know when their cached results are stale.
    document_revision: u64,
    output: CoreResult<String>,
    is_input_changed: bool,
    tree: TreeView,
    query: QueryPanel,
    schema: SchemaPanel,
}

impl Default for JsonViewer {
    fn default() -> Self {
        Self {
            input: String::new(),
            view: JsonView::default(),
            format_options: FormatOptions::default(),
            output_style: OutputStyle::default(),
            document: None,
            document_revision: 0,
            output: Ok(String::new()),
            is_input_changed: false,
            tree: TreeView::default(),
            query: QueryPanel::default(),
            schema: SchemaPanel::default(),
        }
    }
}

impl JsonViewer {
    fn reparse_if_changed(&mut self) {
        if !self.is_input_changed {
            return;
        }
        self.is_input_changed = false;
        self.document_revision += 1;
        self.document = if self.input.trim().is_empty() {
            None
        } else {
            Some(parse_json(&self.input))
        };
        self.reformat_output();
    }

    fn reformat_output(&mut self) {
        self.output = match &self.document {
            Some(Ok(value)) => self.format_value(value, self.output_style),
            Some(Err(_)) | None => Ok(String::new()),
        };
    }

    fn format_value(&self, value: &Value, style: OutputStyle) -> CoreResult<String> {
        match style {
            OutputStyle::Pretty => format_pretty(value, self.format_options),
            OutputStyle::Minified => format_minified(value, self.format_options.should_sort_keys),
        }
    }

    fn replace_input(&mut self, style: OutputStyle) {
        let Some(Ok(value)) = &self.document else {
            return;
        };
        let Ok(text) = self.format_value(value, style) else {
            return;
        };
        self.input = text;
        self.is_input_changed = true;
    }

    fn valid_document(&self) -> Option<&Value> {
        match &self.document {
            Some(Ok(value)) => Some(value),
            Some(Err(_)) | None => None,
        }
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        let palette = Palette::of(ui);
        ui.horizontal(|ui| {
            let options = JsonView::ALL.map(|view| (view, view.label()));
            show_view_tabs(ui, &mut self.view, &options);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(icon_label_button(icons::ERASER, "Clear")).clicked() {
                    self.input.clear();
                    self.is_input_changed = true;
                }
                if ui
                    .add(icon_label_button(icons::CLIPBOARD_TEXT, "Sample"))
                    .clicked()
                {
                    SAMPLE_JSON.clone_into(&mut self.input);
                    self.is_input_changed = true;
                }
            });
        });
        let line_y = ui.min_rect().bottom();
        ui.painter().hline(
            ui.max_rect().x_range(),
            line_y,
            egui::Stroke::new(1.0, palette.border),
        );
        ui.add_space(10.0);
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        let options_before = (self.format_options, self.output_style);
        let has_valid_document = self.valid_document().is_some();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    has_valid_document,
                    icon_label_button(icons::MAGIC_WAND, "Format"),
                )
                .on_hover_text("Rewrite the input pretty-printed")
                .clicked()
            {
                self.replace_input(OutputStyle::Pretty);
            }
            if ui
                .add_enabled(
                    has_valid_document,
                    icon_label_button(icons::ARROWS_IN, "Minify"),
                )
                .on_hover_text("Rewrite the input on one line")
                .clicked()
            {
                self.replace_input(OutputStyle::Minified);
            }
            ui.separator();
            let indent_label = ui.label("Indent");
            let indent_combo = egui::ComboBox::from_id_salt("json_indent")
                .selected_text(self.format_options.indent.label())
                .show_ui(ui, |ui| {
                    for indent in Indent::ALL {
                        ui.selectable_value(
                            &mut self.format_options.indent,
                            indent,
                            indent.label(),
                        );
                    }
                });
            // Names the dropdown for screen readers, since its visible label sits beside it.
            let _ = indent_combo.response.labelled_by(indent_label.id);
            ui.checkbox(&mut self.format_options.should_sort_keys, "Sort keys");
            if self.view == JsonView::Text {
                ui.separator();
                ui.selectable_value(&mut self.output_style, OutputStyle::Pretty, "Pretty");
                ui.selectable_value(&mut self.output_style, OutputStyle::Minified, "Minified");
            }
        });
        ui.add_space(8.0);
        if options_before != (self.format_options, self.output_style) {
            self.reformat_output();
        }
    }

    fn show_status_line(&self, ui: &mut egui::Ui) {
        let (tone, text) = match &self.document {
            None => (
                StatusTone::Neutral,
                "Paste JSON or a raw HTTP response on the left.".to_owned(),
            ),
            Some(Ok(value)) => (
                StatusTone::Success,
                describe_valid_document(value, self.input.len()),
            ),
            Some(Err(error)) => (StatusTone::Error, describe_error(error)),
        };
        show_status(ui, tone, &text);
    }

    fn show_input_pane(&mut self, ui: &mut egui::Ui) {
        show_pane_header(ui, "Input", |_| {});
        let response = show_code_editor(ui, "json_input", &mut self.input, "Paste JSON here");
        if response.changed() {
            self.is_input_changed = true;
        }
    }

    fn show_result_pane(&mut self, ui: &mut egui::Ui) {
        // Field borrows (not `valid_document()`) let the views take `&mut` of their own state.
        let Some(Ok(document)) = &self.document else {
            show_pane_header(ui, self.view.label(), |_| {});
            ui.weak(match self.document {
                None => "The result appears here.",
                Some(_) => "Fix the JSON on the left to see the result.",
            });
            return;
        };
        let revision = self.document_revision;
        match self.view {
            JsonView::Text => show_text_output(ui, &self.output),
            JsonView::Tree => {
                show_pane_header(ui, "Tree", |_| {});
                self.tree.show(ui, document, revision);
            }
            JsonView::Query => {
                show_pane_header(ui, "Query", |_| {});
                self.query.show(ui, document, revision, self.format_options);
            }
            JsonView::Schema => {
                show_pane_header(ui, "Schema", |_| {});
                self.schema.show(ui, document, revision);
            }
        }
    }
}

fn show_text_output(ui: &mut egui::Ui, output: &CoreResult<String>) {
    match output {
        Ok(text) => {
            show_pane_header(ui, "Output", |ui| {
                show_copy_button(ui, "Copy output", text);
            });
            show_code_view(ui, "json_output", text);
        }
        Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
    }
}

impl Tool for JsonViewer {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.reparse_if_changed();
        self.show_header(ui);
        self.show_toolbar(ui);
        // Panes get what is left after reserving the status line, so it is never pushed off-screen.
        let body_height = (ui.available_height() - STATUS_LINE_HEIGHT).max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                self.show_input_pane(&mut columns[0]);
                self.show_result_pane(&mut columns[1]);
            });
        });
        ui.add_space(6.0);
        self.show_status_line(ui);
    }
}

fn describe_valid_document(value: &Value, input_bytes: usize) -> String {
    let stats = measure_json(value);
    format!(
        "Valid JSON · {} objects · {} arrays · {} values · depth {} · {}",
        stats.objects,
        stats.arrays,
        stats.scalars,
        stats.max_depth,
        format_byte_size(input_bytes)
    )
}

fn describe_error(error: &CoreError) -> String {
    match error {
        CoreError::InvalidJson { .. } => format!("Invalid JSON. {error}"),
        CoreError::InvalidInput(message) => message.clone(),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "sizes are shown to one decimal place, far below f64 precision"
)]
fn format_byte_size(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    let size = bytes as f64;
    if size < KIB {
        return format!("{bytes} B");
    }
    if size < KIB * KIB {
        return format!("{:.1} KB", size / KIB);
    }
    format!("{:.1} MB", size / (KIB * KIB))
}
