//! Password Generator: passwords and passphrases on the left; a strength check for any
//! password on the right. Everything runs locally.

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::password::{
    MAX_WORD_COUNT, MIN_PASSWORD_LENGTH, MIN_WORD_COUNT, PassphraseSpec, PasswordSpec, Strength,
    check_strength, generate_passphrase, generate_password, passphrase_entropy_bits,
    password_entropy_bits,
};

use crate::cached::Cached;
use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::code_editor::show_code_block;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{show_checkbox_row, show_custom_row, show_form, show_slider_row};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::input_row::show_secret_field;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::strength_meter::show_strength_meter;
use crate::widgets::view_tabs::show_view_tabs;

const MIN_BODY_HEIGHT: f32 = 160.0;
/// The slider stops here; longer passwords add nothing a person would notice.
const MAX_SLIDER_LENGTH: usize = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Password,
    Passphrase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PassphraseOptions {
    word_count: usize,
    separator: String,
    is_capitalized: bool,
    has_number: bool,
}

impl PassphraseOptions {
    fn spec(&self) -> PassphraseSpec<'_> {
        PassphraseSpec {
            word_count: self.word_count,
            separator: &self.separator,
            is_capitalized: self.is_capitalized,
            has_number: self.has_number,
        }
    }
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        let spec = PassphraseSpec::default();
        Self {
            word_count: spec.word_count,
            separator: spec.separator.to_owned(),
            is_capitalized: spec.is_capitalized,
            has_number: spec.has_number,
        }
    }
}

#[derive(Clone, PartialEq)]
enum GenerateKey {
    Password(PasswordSpec),
    Passphrase(PassphraseOptions),
}

#[derive(Default)]
pub struct PasswordGenerator {
    mode: Mode,
    password: PasswordSpec,
    passphrase: PassphraseOptions,
    generated: Option<Cached<GenerateKey, CoreResult<String>>>,
    generated_strength: Option<Cached<String, Strength>>,
    check_input: String,
    is_check_revealed: bool,
    checked_strength: Option<Cached<String, Strength>>,
}

impl PasswordGenerator {
    fn current_key(&self) -> GenerateKey {
        match self.mode {
            Mode::Password => GenerateKey::Password(self.password),
            Mode::Passphrase => GenerateKey::Passphrase(self.passphrase.clone()),
        }
    }

    fn refresh(&mut self) {
        let key = self.current_key();
        let generated = Cached::refresh(&mut self.generated, key, |key| {
            let mut rng = rand::rng();
            match key {
                GenerateKey::Password(spec) => generate_password(spec, &mut rng),
                GenerateKey::Passphrase(options) => generate_passphrase(&options.spec(), &mut rng),
            }
        });
        if let Ok(password) = generated {
            Cached::refresh(&mut self.generated_strength, password.clone(), |password| {
                check_strength(password)
            });
        }
        if !self.check_input.is_empty() {
            Cached::refresh(
                &mut self.checked_strength,
                self.check_input.clone(),
                |password| check_strength(password),
            );
        }
    }

    fn generated_password(&self) -> Option<&str> {
        match &self.generated.as_ref()?.value {
            Ok(password) => Some(password),
            Err(_) => None,
        }
    }

    fn entropy_bits(&self) -> CoreResult<f64> {
        match self.mode {
            Mode::Password => password_entropy_bits(&self.password),
            Mode::Passphrase => passphrase_entropy_bits(&self.passphrase.spec()),
        }
    }

    fn show_generate_pane(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Generate", |ui| {
            show_view_tabs(
                ui,
                &mut self.mode,
                &[
                    (Mode::Password, "Password"),
                    (Mode::Passphrase, "Passphrase"),
                ],
            );
            ui.add_space(10.0);
            match self.mode {
                Mode::Password => show_form(ui, "password_options", |ui| {
                    show_password_options(ui, &mut self.password);
                }),
                Mode::Passphrase => show_form(ui, "passphrase_options", |ui| {
                    show_passphrase_options(ui, &mut self.passphrase);
                }),
            }
        });
        ui.add_space(12.0);
        let text = self.generated_password().unwrap_or_default().to_owned();
        let mut is_regenerate_clicked = false;
        show_pane_header(ui, "Result", |ui| {
            show_copy_button(ui, "Copy", &text);
            is_regenerate_clicked = ui
                .add(icon_label_button(icons::ARROWS_CLOCKWISE, "Regenerate"))
                .clicked();
        });
        if is_regenerate_clicked {
            self.generated = None;
            self.refresh();
        }
        if let Some(Cached {
            value: Err(error), ..
        }) = &self.generated
        {
            show_status(ui, StatusTone::Error, &error.to_string());
            return;
        }
        show_code_block(ui, "password_result", &text);
        ui.add_space(8.0);
        if let Some(strength) = self.generated_strength.as_ref().map(|cached| &cached.value) {
            show_strength(ui, strength);
        }
        if let Ok(bits) = self.entropy_bits() {
            ui.weak(format!("About {bits:.0} bits of randomness."));
        }
    }

    fn show_check_pane(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Check a password", |ui| {
            show_secret_field(
                ui,
                "password_check",
                &mut self.check_input,
                "Type a password to rate it",
                &mut self.is_check_revealed,
            );
            ui.add_space(8.0);
            let strength = self
                .checked_strength
                .as_ref()
                .filter(|cached| !self.check_input.is_empty() && cached.key == self.check_input)
                .map(|cached| &cached.value);
            match strength {
                Some(strength) => show_strength(ui, strength),
                None => {
                    ui.weak("Looks for words, names, dates, keyboard runs and repeats.");
                }
            }
            ui.add_space(6.0);
            ui.weak("Checked on this computer; the password goes nowhere.");
        });
    }
}

impl Tool for PasswordGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.refresh();
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("password_generate_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_generate_pane(ui));
                egui::ScrollArea::vertical()
                    .id_salt("password_check_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_check_pane(ui));
            });
        });
        // Text typed this frame is rated on the next one, so ask for it straight away.
        let is_check_stale = !self.check_input.is_empty()
            && self
                .checked_strength
                .as_ref()
                .is_none_or(|cached| cached.key != self.check_input);
        if is_check_stale {
            ui.ctx().request_repaint();
        }
    }
}

fn show_password_options(ui: &mut egui::Ui, spec: &mut PasswordSpec) {
    show_slider_row(
        ui,
        "Length",
        &mut spec.length,
        MIN_PASSWORD_LENGTH..=MAX_SLIDER_LENGTH,
    );
    show_checkbox_row(
        ui,
        "Include",
        &mut [
            (&mut spec.has_lowercase, "a–z"),
            (&mut spec.has_uppercase, "A–Z"),
            (&mut spec.has_digits, "0–9"),
            (&mut spec.has_symbols, "!#$%"),
        ],
    );
    show_checkbox_row(
        ui,
        "Readability",
        &mut [(&mut spec.is_excluding_look_alikes, "Leave out 0 O o 1 l I")],
    );
}

fn show_passphrase_options(ui: &mut egui::Ui, options: &mut PassphraseOptions) {
    show_slider_row(
        ui,
        "Words",
        &mut options.word_count,
        MIN_WORD_COUNT..=MAX_WORD_COUNT,
    );
    show_custom_row(ui, "Separator", |ui, label_id| {
        ui.add(
            egui::TextEdit::singleline(&mut options.separator)
                .id_salt("passphrase_separator")
                .font(egui::TextStyle::Monospace)
                .desired_width(60.0),
        )
        .labelled_by(label_id);
    });
    show_checkbox_row(
        ui,
        "Style",
        &mut [
            (&mut options.is_capitalized, "Capitalize"),
            (&mut options.has_number, "Add a digit"),
        ],
    );
}

fn show_strength(ui: &mut egui::Ui, strength: &Strength) {
    show_strength_meter(ui, strength.score, strength.label);
    ui.weak(format!(
        "Cracking it offline at 10,000 guesses a second: {}.",
        strength.crack_time
    ));
    if let Some(warning) = &strength.warning {
        show_status(ui, StatusTone::Warning, warning);
    }
    for suggestion in &strength.suggestions {
        ui.weak(format!("• {suggestion}"));
    }
}
