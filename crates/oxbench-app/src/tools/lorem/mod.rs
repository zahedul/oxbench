//! Lorem Ipsum: placeholder paragraphs, sentences or words, as text or HTML.

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::lorem::{LoremSpec, LoremUnit, format_as_html, format_as_text, generate_lorem};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::cached::Cached;
use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{show_checkbox_row, show_choice_row, show_form, show_number_row};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::pane_header::show_pane_header;

const MIN_BODY_HEIGHT: f32 = 200.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OutputFormat {
    #[default]
    Text,
    Html,
}

/// The text shown, with its counts, made once per set of options.
struct LoremOutput {
    text: String,
    summary: String,
}

/// The options plus a seed, so the text changes only when asked to.
type OutputKey = (LoremSpec, OutputFormat, u64);

pub struct LoremIpsum {
    spec: LoremSpec,
    format: OutputFormat,
    seed: u64,
    output: Option<Cached<OutputKey, LoremOutput>>,
}

impl Default for LoremIpsum {
    fn default() -> Self {
        Self {
            spec: LoremSpec::default(),
            format: OutputFormat::default(),
            seed: rand::random(),
            output: None,
        }
    }
}

impl LoremIpsum {
    fn refresh_output(&mut self) -> &LoremOutput {
        let key = (self.spec, self.format, self.seed);
        Cached::refresh(&mut self.output, key, |&(spec, format, seed)| {
            build_output(spec, format, seed)
        })
    }

    fn show_options(&mut self, ui: &mut egui::Ui) {
        let units = LoremUnit::ALL.map(|unit| (unit, unit.label()));
        if show_choice_row(ui, "Generate", &mut self.spec.unit, &units) {
            self.spec.count = self.spec.unit.default_count();
        }
        let max_count = self.spec.unit.max_count();
        show_number_row(ui, "How many", &mut self.spec.count, 1..=max_count);
        show_checkbox_row(
            ui,
            "Opening",
            &mut [(
                &mut self.spec.should_start_with_lorem,
                "Start with \u{201c}Lorem ipsum dolor sit amet\u{201d}",
            )],
        );
        let formats = [
            (OutputFormat::Text, "Plain text"),
            (OutputFormat::Html, "HTML paragraphs"),
        ];
        show_choice_row(ui, "Format", &mut self.format, &formats);
    }

    fn show_result(&mut self, ui: &mut egui::Ui) {
        let mut is_regenerate_clicked = false;
        let output = self.refresh_output();
        show_pane_header(ui, "Result", |ui| {
            show_copy_button(ui, "Copy", &output.text);
            is_regenerate_clicked = ui
                .add(icon_label_button(icons::ARROWS_CLOCKWISE, "Regenerate"))
                .clicked();
        });
        ui.weak(&output.summary);
        ui.add_space(6.0);
        let mut read_only = output.text.as_str();
        ui.add(
            egui::TextEdit::multiline(&mut read_only)
                .id_salt("lorem_result")
                .desired_width(f32::INFINITY),
        );
        if is_regenerate_clicked {
            self.seed = rand::random();
        }
    }
}

impl Tool for LoremIpsum {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("lorem_options_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| {
                        show_card(ui, "Options", |ui| {
                            show_form(ui, "lorem_options", |ui| self.show_options(ui));
                        });
                    });
                egui::ScrollArea::vertical()
                    .id_salt("lorem_result_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_result(ui));
            });
        });
    }
}

fn build_output(spec: LoremSpec, format: OutputFormat, seed: u64) -> LoremOutput {
    let paragraphs = generate_lorem(spec, &mut StdRng::seed_from_u64(seed));
    let words: usize = paragraphs
        .iter()
        .map(|paragraph| paragraph.split_whitespace().count())
        .sum();
    let characters: usize = paragraphs
        .iter()
        .map(|paragraph| paragraph.chars().count())
        .sum();
    let text = match format {
        OutputFormat::Text => format_as_text(&paragraphs),
        OutputFormat::Html => format_as_html(&paragraphs),
    };
    let summary = format!(
        "{} · {} · {}",
        count_noun(paragraphs.len(), "paragraph", "paragraphs"),
        count_noun(words, "word", "words"),
        count_noun(characters, "character", "characters"),
    );
    LoremOutput { text, summary }
}

fn count_noun(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}
