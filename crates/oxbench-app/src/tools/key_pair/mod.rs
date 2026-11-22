//! Key Pair Generator: Ed25519, ECDSA and RSA keys in OpenSSH and PEM formats, made locally.

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::keypair::{KeyAlgorithm, KeyPair, generate_key_pair};

use crate::save_file::{Readers, SaveOutcome, save_with_dialog};
use crate::style::icons;
use crate::tasks::JobSlot;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::code_editor::show_code_block;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{show_choice_row, show_custom_row, show_form};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::value_list::{ValueRow, show_value_list};
use crate::widgets::view_tabs::show_view_tabs;

const MIN_BODY_HEIGHT: f32 = 200.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PrivateFormat {
    #[default]
    OpenSsh,
    Pkcs8,
}

struct KeyOutcome {
    key_pair: CoreResult<KeyPair>,
    took: Duration,
}

#[derive(Default)]
pub struct KeyPairGenerator {
    algorithm: KeyAlgorithm,
    comment: String,
    private_format: PrivateFormat,
    job: JobSlot<KeyOutcome>,
    /// The algorithm of the running or finished job, for its progress message.
    job_algorithm: KeyAlgorithm,
    save_outcome: Option<SaveOutcome>,
}

impl KeyPairGenerator {
    fn start_generating(&mut self, ctx: &egui::Context) {
        let algorithm = self.algorithm;
        let comment = self.comment.clone();
        self.job_algorithm = algorithm;
        self.save_outcome = None;
        self.job.start(ctx, move || {
            let started = Instant::now();
            let key_pair = generate_key_pair(algorithm, &comment, &mut rand::rng());
            KeyOutcome {
                key_pair,
                took: started.elapsed(),
            }
        });
    }

    #[cfg(test)]
    fn key_pair(&self) -> Option<&KeyPair> {
        self.job.finished()?.key_pair.as_ref().ok()
    }

    fn show_generate_card(&mut self, ui: &mut egui::Ui) {
        let mut is_generate_clicked = false;
        show_card(ui, "Generate", |ui| {
            show_form(ui, "key_pair_options", |ui| {
                let algorithms = KeyAlgorithm::ALL.map(|algorithm| (algorithm, algorithm.label()));
                show_choice_row(ui, "Algorithm", &mut self.algorithm, &algorithms);
                show_custom_row(ui, "Comment", |ui, label_id| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.comment)
                            .id_salt("key_pair_comment")
                            .hint_text("you@laptop")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(label_id);
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let is_running = self.job.is_running();
                is_generate_clicked = ui
                    .add_enabled(!is_running, icon_label_button(icons::KEY, "Generate"))
                    .clicked();
                if is_running {
                    ui.spinner();
                    ui.weak(describe_progress(self.job_algorithm));
                } else if let Some(outcome) = self.job.finished() {
                    ui.weak(describe_duration(outcome.took));
                }
            });
            ui.add_space(8.0);
            ui.weak(
                "Keys are made on this computer and never leave it. Private keys are not \
                 encrypted: keep them to yourself, or add a passphrase with ssh-keygen -p.",
            );
        });
        if is_generate_clicked {
            self.start_generating(ui.ctx());
        }
    }

    fn show_keys(&mut self, ui: &mut egui::Ui) {
        let key_pair = match &self.job {
            JobSlot::Finished(KeyOutcome {
                key_pair: Ok(key_pair),
                ..
            }) => key_pair.clone(),
            JobSlot::Finished(KeyOutcome {
                key_pair: Err(error),
                ..
            }) => {
                show_status(ui, StatusTone::Error, &error.to_string());
                return;
            }
            JobSlot::Lost => {
                show_status(
                    ui,
                    StatusTone::Error,
                    "Key generation stopped unexpectedly.",
                );
                return;
            }
            JobSlot::Idle | JobSlot::Running(_) => return,
        };
        if let Some(outcome) = &self.save_outcome {
            let tone = match outcome {
                SaveOutcome::Saved { .. } => StatusTone::Success,
                SaveOutcome::Failed { .. } => StatusTone::Error,
            };
            show_status(ui, tone, &outcome.describe());
            ui.add_space(6.0);
        }
        self.show_public_keys(ui, &key_pair);
        ui.add_space(16.0);
        self.show_private_key(ui, &key_pair);
    }

    fn show_public_keys(&mut self, ui: &mut egui::Ui, key_pair: &KeyPair) {
        let file_name = format!("{}.pub", key_pair.algorithm.default_file_name());
        let mut outcome = None;
        show_pane_header(ui, "Public key · OpenSSH", |ui| {
            show_copy_button(ui, "Copy public key", &key_pair.public_openssh);
            if ui
                .add(icon_label_button(icons::DOWNLOAD_SIMPLE, "Save"))
                .on_hover_text(format!("Save as {file_name}"))
                .clicked()
            {
                let contents = format!("{}\n", key_pair.public_openssh);
                outcome = save_with_dialog(
                    "Save public key",
                    &file_name,
                    None,
                    contents.as_bytes(),
                    Readers::Anyone,
                );
            }
        });
        show_code_block(ui, "key_pair_public_openssh", &key_pair.public_openssh);
        ui.add_space(6.0);
        show_value_list(ui, &[ValueRow::new("Fingerprint", &key_pair.fingerprint)]);
        ui.add_space(12.0);
        show_pane_header(ui, "Public key · PEM", |ui| {
            show_copy_button(ui, "Copy PEM public key", &key_pair.public_pem);
        });
        show_code_block(ui, "key_pair_public_pem", key_pair.public_pem.trim_end());
        if outcome.is_some() {
            self.save_outcome = outcome;
        }
    }

    fn show_private_key(&mut self, ui: &mut egui::Ui, key_pair: &KeyPair) {
        let (text, file_name) = match self.private_format {
            PrivateFormat::OpenSsh => (
                &key_pair.private_openssh,
                key_pair.algorithm.default_file_name().to_owned(),
            ),
            PrivateFormat::Pkcs8 => (&key_pair.private_pem, "private-key.pem".to_owned()),
        };
        let mut outcome = None;
        show_pane_header(ui, "Private key", |ui| {
            show_copy_button(ui, "Copy private key", text);
            if ui
                .add(icon_label_button(icons::DOWNLOAD_SIMPLE, "Save"))
                .on_hover_text(format!("Save as {file_name}, readable only by you"))
                .clicked()
            {
                outcome = save_with_dialog(
                    "Save private key",
                    &file_name,
                    None,
                    text.as_bytes(),
                    Readers::OwnerOnly,
                );
            }
        });
        let formats = [
            (PrivateFormat::OpenSsh, "OpenSSH"),
            (PrivateFormat::Pkcs8, "PKCS #8 PEM"),
        ];
        show_view_tabs(ui, &mut self.private_format, &formats);
        ui.add_space(6.0);
        show_code_block(ui, "key_pair_private", text.trim_end());
        if outcome.is_some() {
            self.save_outcome = outcome;
        }
    }
}

impl Tool for KeyPairGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.job.poll();
        // Opens with a key ready to copy; Ed25519 takes well under a millisecond.
        if matches!(self.job, JobSlot::Idle) {
            self.start_generating(ui.ctx());
        }
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("key_pair_options_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_generate_card(ui));
                egui::ScrollArea::vertical()
                    .id_salt("key_pair_keys_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_keys(ui));
            });
        });
    }
}

fn describe_progress(algorithm: KeyAlgorithm) -> String {
    if algorithm.is_slow() {
        return format!(
            "Generating a {} key; finding large primes can take a few seconds…",
            algorithm.label()
        );
    }
    format!("Generating a {} key…", algorithm.label())
}

fn describe_duration(took: Duration) -> String {
    match took.as_millis() {
        0 => "Took under 1 ms.".to_owned(),
        millis => format!("Took {millis} ms."),
    }
}
