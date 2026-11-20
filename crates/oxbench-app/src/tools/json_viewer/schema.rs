use eframe::egui;
use oxbench_core::json::{SchemaViolation, Value, parse_json, validate_against_schema};

use crate::widgets::code_editor::show_code_editor;
use crate::widgets::status::{StatusTone, show_status};

#[derive(Default)]
pub struct SchemaPanel {
    pub(super) schema_text: String,
    cached: Option<CachedValidation>,
}

struct CachedValidation {
    revision: u64,
    pub(super) schema_text: String,
    outcome: ValidationOutcome,
}

enum ValidationOutcome {
    NoSchema,
    SchemaProblem(String),
    Valid,
    Violations(Vec<SchemaViolation>),
}

impl SchemaPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, document: &Value, revision: u64) {
        ui.label("JSON Schema");
        let editor_height = (ui.available_height() * 0.5).max(120.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), editor_height), |ui| {
            show_code_editor(
                ui,
                "json_schema_input",
                &mut self.schema_text,
                "Paste a JSON Schema here",
            );
        });
        ui.separator();
        self.refresh(document, revision);
        let Some(cached) = &self.cached else {
            return;
        };
        show_outcome(ui, &cached.outcome);
    }

    fn refresh(&mut self, document: &Value, revision: u64) {
        let is_fresh = self.cached.as_ref().is_some_and(|cached| {
            cached.revision == revision && cached.schema_text == self.schema_text
        });
        if is_fresh {
            return;
        }
        self.cached = Some(CachedValidation {
            revision,
            schema_text: self.schema_text.clone(),
            outcome: validate(document, &self.schema_text),
        });
    }
}

fn validate(document: &Value, schema_text: &str) -> ValidationOutcome {
    if schema_text.trim().is_empty() {
        return ValidationOutcome::NoSchema;
    }
    let schema = match parse_json(schema_text) {
        Ok(schema) => schema,
        Err(error) => {
            return ValidationOutcome::SchemaProblem(format!("Schema is not valid JSON. {error}"));
        }
    };
    match validate_against_schema(document, &schema) {
        Ok(violations) if violations.is_empty() => ValidationOutcome::Valid,
        Ok(violations) => ValidationOutcome::Violations(violations),
        Err(error) => ValidationOutcome::SchemaProblem(error.to_string()),
    }
}

fn show_outcome(ui: &mut egui::Ui, outcome: &ValidationOutcome) {
    match outcome {
        ValidationOutcome::NoSchema => {
            ui.weak("Paste a schema above to check the JSON against it. Remote $ref links are not fetched.");
        }
        ValidationOutcome::SchemaProblem(message) => show_status(ui, StatusTone::Error, message),
        ValidationOutcome::Valid => {
            show_status(ui, StatusTone::Success, "The JSON matches the schema.");
        }
        ValidationOutcome::Violations(violations) => show_violations(ui, violations),
    }
}

fn show_violations(ui: &mut egui::Ui, violations: &[SchemaViolation]) {
    let summary = match violations.len() {
        1 => "1 problem".to_owned(),
        count => format!("{count} problems"),
    };
    show_status(ui, StatusTone::Error, &summary);
    egui::ScrollArea::vertical()
        .id_salt("json_schema_violations")
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Grid::new("json_schema_violation_grid")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    for violation in violations {
                        ui.label(egui::RichText::new(&violation.path).monospace());
                        ui.label(&violation.message);
                        ui.end_row();
                    }
                });
        });
}
