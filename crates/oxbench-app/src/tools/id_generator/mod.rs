//! UUID / ULID: generate IDs of any version on the left; inspect a pasted ID on the right.

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::epoch::{describe_relative, format_iso8601, format_readable};
use oxbench_core::id::{
    IdDetails, IdFormat, IdKind, MAX_ID_COUNT, NameInput, Namespace, generate_ids, inspect_id,
};

use crate::cached::Cached;
use crate::clock::Clock;
use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::code_editor::show_code_block;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{
    show_checkbox_row, show_choice_row, show_custom_row, show_form, show_number_row,
};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::value_list::{ValueRow, show_value_list};

const MIN_BODY_HEIGHT: f32 = 160.0;

/// Everything that decides the generated IDs; a change regenerates them.
#[derive(Clone, PartialEq)]
struct GenerateKey {
    kind: IdKind,
    count: usize,
    format: IdFormat,
    namespace: Namespace,
    custom_namespace: String,
    name: String,
}

pub struct IdGenerator {
    clock: Clock,
    kind: IdKind,
    count: usize,
    format: IdFormat,
    namespace: Namespace,
    custom_namespace: String,
    name: String,
    generated: Option<Cached<GenerateKey, CoreResult<Vec<String>>>>,
    inspect_input: String,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(Clock::System)
    }
}

impl IdGenerator {
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            kind: IdKind::default(),
            count: 1,
            format: IdFormat::default(),
            namespace: Namespace::default(),
            custom_namespace: String::new(),
            name: String::new(),
            generated: None,
            inspect_input: String::new(),
        }
    }

    fn refresh_generated(&mut self) {
        let key = GenerateKey {
            kind: self.kind,
            count: self.count,
            format: self.format,
            namespace: self.namespace,
            custom_namespace: self.custom_namespace.clone(),
            name: self.name.clone(),
        };
        Cached::refresh(&mut self.generated, key, |key| {
            let name = NameInput {
                namespace: key.namespace,
                custom_namespace: &key.custom_namespace,
                name: &key.name,
            };
            generate_ids(key.kind, key.count, key.format, &name)
        });
    }

    fn generated_text(&self) -> String {
        match self.generated.as_ref().map(|cached| &cached.value) {
            Some(Ok(ids)) => ids.join("\n"),
            _ => String::new(),
        }
    }

    /// ULIDs are conventionally uppercase and UUIDs lowercase, so switching kinds follows suit.
    fn change_kind(&mut self, previous: IdKind) {
        if previous.is_ulid() != self.kind.is_ulid() {
            self.format.is_uppercase = self.kind.is_ulid();
        }
    }

    fn show_generate_pane(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Generate", |ui| {
            show_form(ui, "id_options", |ui| self.show_options(ui));
        });
        ui.add_space(12.0);
        let text = self.generated_text();
        let is_repeatable = !self.kind.is_deterministic();
        let mut is_regenerate_clicked = false;
        show_pane_header(ui, "Result", |ui| {
            show_copy_button(ui, "Copy all", &text);
            if is_repeatable {
                is_regenerate_clicked = ui
                    .add(icon_label_button(icons::ARROWS_CLOCKWISE, "Regenerate"))
                    .clicked();
            }
        });
        if is_regenerate_clicked {
            self.generated = None;
            self.refresh_generated();
        }
        match self.generated.as_ref().map(|cached| &cached.value) {
            Some(Ok(_)) => show_code_block(ui, "id_result", &self.generated_text()),
            Some(Err(error)) => show_status(ui, StatusTone::Error, &error.to_string()),
            None => {}
        }
        if self.kind.is_name_based() {
            ui.add_space(6.0);
            ui.weak("The same namespace and name always give the same UUID.");
        }
    }

    fn show_options(&mut self, ui: &mut egui::Ui) {
        let kinds: Vec<(IdKind, &str)> = IdKind::ALL.iter().map(|k| (*k, k.label())).collect();
        let previous = self.kind;
        if show_choice_row(ui, "Kind", &mut self.kind, &kinds) {
            self.change_kind(previous);
        }
        if self.kind.is_name_based() {
            let namespaces: Vec<(Namespace, &str)> =
                Namespace::ALL.iter().map(|n| (*n, n.label())).collect();
            show_choice_row(ui, "Namespace", &mut self.namespace, &namespaces);
            if self.namespace == Namespace::Custom {
                show_text_row(
                    ui,
                    "Namespace UUID",
                    &mut self.custom_namespace,
                    "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                );
            }
            show_text_row(ui, "Name", &mut self.name, "example.com");
        }
        if self.kind.is_ulid() {
            show_checkbox_row(
                ui,
                "Format",
                &mut [(&mut self.format.is_uppercase, "Uppercase")],
            );
        } else {
            show_checkbox_row(
                ui,
                "Format",
                &mut [
                    (&mut self.format.is_uppercase, "Uppercase"),
                    (&mut self.format.has_hyphens, "Hyphens"),
                    (&mut self.format.has_braces, "Braces"),
                ],
            );
        }
        if !self.kind.is_deterministic() {
            show_number_row(ui, "How many", &mut self.count, 1..=MAX_ID_COUNT);
        }
    }

    fn show_inspect_pane(&mut self, ui: &mut egui::Ui) {
        let now = self.clock.now();
        show_card(ui, "Inspect", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.inspect_input)
                    .id_salt("id_inspect")
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Paste a UUID or ULID")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            if self.inspect_input.trim().is_empty() {
                ui.weak("Shows the version, embedded time and both spellings of an ID.");
                return;
            }
            match inspect_id(&self.inspect_input) {
                Ok(details) => show_details(ui, &details, now),
                Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
            }
        });
    }
}

impl Tool for IdGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.refresh_generated();
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("id_generate_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_generate_pane(ui));
                egui::ScrollArea::vertical()
                    .id_salt("id_inspect_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_inspect_pane(ui));
            });
        });
    }
}

fn show_text_row(ui: &mut egui::Ui, label: &str, text: &mut String, hint: &str) {
    show_custom_row(ui, label, |ui, label_id| {
        ui.add(
            egui::TextEdit::singleline(text)
                .id_salt(label)
                .font(egui::TextStyle::Monospace)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        )
        .labelled_by(label_id);
    });
}

fn show_details(
    ui: &mut egui::Ui,
    details: &IdDetails,
    now: oxbench_core::epoch::DateTime<oxbench_core::epoch::Utc>,
) {
    show_status(ui, StatusTone::Success, &details.description);
    ui.add_space(6.0);
    let mut rows = vec![
        ValueRow::new("UUID", &details.uuid),
        ValueRow::new("ULID", &details.ulid),
    ];
    if let Some(time) = details.time {
        let readable = format!(
            "{} ({})",
            format_readable(time.fixed_offset()),
            describe_relative(time, now)
        );
        rows.push(ValueRow::new("Time", readable).copying(&format_iso8601(time.fixed_offset())));
    }
    if let Some(node) = &details.node {
        rows.push(ValueRow::new("Node", node));
    }
    if let Some(variant) = details.variant {
        rows.push(ValueRow::new("Variant", variant));
    }
    show_value_list(ui, &rows);
}
