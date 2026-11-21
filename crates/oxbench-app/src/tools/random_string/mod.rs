//! Random String: Nano IDs, custom alphabets, and random bytes as hex or Base64.

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::random_string::{
    MAX_COUNT, MAX_LENGTH, RandomFormat, RandomSpec, entropy_bits, generate_random_strings,
};

use crate::cached::Cached;
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

const MIN_BODY_HEIGHT: f32 = 160.0;

#[derive(Clone, PartialEq)]
struct GenerateKey {
    format: RandomFormat,
    length: usize,
    count: usize,
    custom_alphabet: String,
    is_excluding_look_alikes: bool,
}

impl GenerateKey {
    fn spec(&self) -> RandomSpec<'_> {
        RandomSpec {
            format: self.format,
            length: self.length,
            custom_alphabet: &self.custom_alphabet,
            is_excluding_look_alikes: self.is_excluding_look_alikes,
        }
    }
}

pub struct RandomStringGenerator {
    format: RandomFormat,
    length: usize,
    count: usize,
    custom_alphabet: String,
    is_excluding_look_alikes: bool,
    generated: Option<Cached<GenerateKey, CoreResult<Vec<String>>>>,
}

impl Default for RandomStringGenerator {
    fn default() -> Self {
        let format = RandomFormat::default();
        Self {
            format,
            length: format.default_length(),
            count: 1,
            custom_alphabet: "abcdef0123456789".to_owned(),
            is_excluding_look_alikes: false,
            generated: None,
        }
    }
}

impl RandomStringGenerator {
    fn current_key(&self) -> GenerateKey {
        GenerateKey {
            format: self.format,
            length: self.length,
            count: self.count,
            custom_alphabet: self.custom_alphabet.clone(),
            is_excluding_look_alikes: self.is_excluding_look_alikes,
        }
    }

    fn refresh_generated(&mut self) {
        let key = self.current_key();
        Cached::refresh(&mut self.generated, key, |key| {
            generate_random_strings(&key.spec(), key.count, &mut rand::rng())
        });
    }

    fn generated_text(&self) -> String {
        match self.generated.as_ref().map(|cached| &cached.value) {
            Some(Ok(strings)) => strings.join("\n"),
            _ => String::new(),
        }
    }

    fn show_options(&mut self, ui: &mut egui::Ui) {
        let formats: Vec<(RandomFormat, &str)> =
            RandomFormat::ALL.iter().map(|f| (*f, f.label())).collect();
        if show_choice_row(ui, "Format", &mut self.format, &formats) {
            self.length = self.format.default_length();
        }
        if self.format == RandomFormat::Custom {
            show_custom_row(ui, "Alphabet", |ui, label_id| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.custom_alphabet)
                        .id_salt("random_alphabet")
                        .font(egui::TextStyle::Monospace)
                        .hint_text("Characters to pick from")
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label_id);
            });
        }
        let length_label = if self.format.is_byte_based() {
            "Bytes"
        } else {
            "Characters"
        };
        show_number_row(ui, length_label, &mut self.length, 1..=MAX_LENGTH);
        if !self.format.is_byte_based() {
            show_checkbox_row(
                ui,
                "Readability",
                &mut [(&mut self.is_excluding_look_alikes, "Leave out 0 O o 1 l I")],
            );
        }
        show_number_row(ui, "How many", &mut self.count, 1..=MAX_COUNT);
    }

    fn show_result(&mut self, ui: &mut egui::Ui) {
        let text = self.generated_text();
        let mut is_regenerate_clicked = false;
        show_pane_header(ui, "Result", |ui| {
            show_copy_button(ui, "Copy all", &text);
            is_regenerate_clicked = ui
                .add(icon_label_button(icons::ARROWS_CLOCKWISE, "Regenerate"))
                .clicked();
        });
        if is_regenerate_clicked {
            self.generated = None;
            self.refresh_generated();
        }
        match self.generated.as_ref().map(|cached| &cached.value) {
            Some(Ok(_)) => show_code_block(ui, "random_result", &self.generated_text()),
            Some(Err(error)) => show_status(ui, StatusTone::Error, &error.to_string()),
            None => {}
        }
    }
}

impl Tool for RandomStringGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.refresh_generated();
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("random_options_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| {
                        show_card(ui, "Options", |ui| {
                            show_form(ui, "random_options", |ui| self.show_options(ui));
                            ui.add_space(8.0);
                            if let Ok(bits) = entropy_bits(&self.current_key().spec()) {
                                ui.weak(describe_entropy(bits));
                            }
                        });
                    });
                egui::ScrollArea::vertical()
                    .id_salt("random_result_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_result(ui));
            });
        });
    }
}

fn describe_entropy(bits: f64) -> String {
    // 120 bits puts guessing out of reach; UUID v4 has 122 and Nano ID 126.
    let verdict = if bits >= 120.0 {
        "enough for secrets and API keys"
    } else if bits >= 64.0 {
        "fine for IDs; use 120 or more for secrets"
    } else {
        "guessable; use for codes that expire quickly"
    };
    format!("About {bits:.0} bits of randomness each: {verdict}.")
}
