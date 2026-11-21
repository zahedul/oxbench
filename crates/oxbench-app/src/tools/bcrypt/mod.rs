//! Bcrypt: hash a password on the left, check a password against a hash on the right. Both
//! run on a background thread because bcrypt is slow on purpose.

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::bcrypt::{
    BcryptParts, DEFAULT_COST, MAX_COST, MAX_PASSWORD_BYTES, MIN_COST, hash_password, is_truncated,
    parse_bcrypt_hash, verify_password,
};

use crate::style::icons;
use crate::tasks::JobSlot;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::form::{show_form, show_slider_row};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::input_row::show_secret_field;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::value_list::{ValueRow, show_value_list};

const MIN_BODY_HEIGHT: f32 = 160.0;

struct HashOutcome {
    hash: CoreResult<String>,
    cost: u32,
    elapsed: Duration,
}

struct VerifyOutcome {
    /// The password and hash that were checked, so a stale answer is not shown after edits.
    inputs: (String, String),
    is_match: CoreResult<bool>,
}

pub struct BcryptTool {
    hash_input: String,
    is_hash_input_revealed: bool,
    cost: usize,
    hash_job: JobSlot<HashOutcome>,
    verify_password: String,
    is_verify_password_revealed: bool,
    verify_hash: String,
    verify_job: JobSlot<VerifyOutcome>,
}

impl Default for BcryptTool {
    fn default() -> Self {
        Self {
            hash_input: String::new(),
            is_hash_input_revealed: false,
            cost: to_usize(DEFAULT_COST),
            hash_job: JobSlot::default(),
            verify_password: String::new(),
            is_verify_password_revealed: false,
            verify_hash: String::new(),
            verify_job: JobSlot::default(),
        }
    }
}

impl BcryptTool {
    fn start_hash(&mut self, ctx: &egui::Context) {
        let password = self.hash_input.clone();
        let cost = u32::try_from(self.cost).unwrap_or(DEFAULT_COST);
        self.hash_job.start(ctx, move || {
            let started = Instant::now();
            let hash = hash_password(&password, cost);
            HashOutcome {
                hash,
                cost,
                elapsed: started.elapsed(),
            }
        });
    }

    fn start_verify(&mut self, ctx: &egui::Context) {
        let inputs = (self.verify_password.clone(), self.verify_hash.clone());
        self.verify_job.start(ctx, move || {
            let is_match = verify_password(&inputs.0, &inputs.1);
            VerifyOutcome { inputs, is_match }
        });
    }

    /// Copies the password and its new hash into the verify card.
    fn check_new_hash(&mut self, hash: &str) {
        self.verify_password.clone_from(&self.hash_input);
        hash.clone_into(&mut self.verify_hash);
        self.verify_job = JobSlot::Idle;
    }

    fn show_hash_card(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Hash a password", |ui| {
            show_secret_field(
                ui,
                "bcrypt_hash_password",
                &mut self.hash_input,
                "Password",
                &mut self.is_hash_input_revealed,
            );
            if is_truncated(&self.hash_input) {
                ui.add_space(4.0);
                show_status(
                    ui,
                    StatusTone::Warning,
                    &format!(
                        "Bcrypt uses only the first {MAX_PASSWORD_BYTES} bytes; the rest is ignored."
                    ),
                );
            }
            ui.add_space(8.0);
            show_form(ui, "bcrypt_options", |ui| {
                show_slider_row(
                    ui,
                    "Cost",
                    &mut self.cost,
                    to_usize(MIN_COST)..=to_usize(MAX_COST),
                );
            });
            ui.weak("Each step doubles the time. 10 to 12 suits most sites.");
            ui.add_space(8.0);
            let is_running = self.hash_job.is_running();
            ui.horizontal(|ui| {
                let button = icon_label_button(icons::LOCK_KEY, "Hash");
                if ui.add_enabled(!is_running, button).clicked() {
                    self.start_hash(ui.ctx());
                }
                if is_running {
                    ui.spinner();
                    ui.weak("Hashing…");
                }
            });
            self.show_hash_outcome(ui);
        });
    }

    fn show_hash_outcome(&mut self, ui: &mut egui::Ui) {
        let new_hash = match &self.hash_job {
            JobSlot::Finished(outcome) => match &outcome.hash {
                Ok(hash) => {
                    ui.add_space(8.0);
                    show_value_list(ui, &[ValueRow::new("Hash", hash)]);
                    ui.weak(format!(
                        "Took {} ms at cost {}.",
                        outcome.elapsed.as_millis(),
                        outcome.cost
                    ));
                    Some(hash.clone())
                }
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    None
                }
            },
            JobSlot::Lost => {
                show_status(ui, StatusTone::Error, "Hashing stopped unexpectedly.");
                None
            }
            JobSlot::Idle | JobSlot::Running(_) => None,
        };
        let Some(hash) = new_hash else {
            return;
        };
        let is_check_clicked = ui
            .horizontal(|ui| {
                ui.add(icon_label_button(icons::SHIELD_CHECK, "Check it"))
                    .on_hover_text("Copy the password and hash into Verify")
                    .clicked()
            })
            .inner;
        if is_check_clicked {
            self.check_new_hash(&hash);
        }
    }

    fn show_verify_card(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Verify", |ui| {
            show_secret_field(
                ui,
                "bcrypt_verify_password",
                &mut self.verify_password,
                "Password",
                &mut self.is_verify_password_revealed,
            );
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.verify_hash)
                    .id_salt("bcrypt_verify_hash")
                    .font(egui::TextStyle::Monospace)
                    .hint_text("$2b$12$…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            if self.verify_hash.trim().is_empty() {
                ui.weak("Paste a bcrypt hash to see its cost and salt.");
                return;
            }
            let parts = match parse_bcrypt_hash(&self.verify_hash) {
                Ok(parts) => parts,
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    return;
                }
            };
            show_hash_parts(ui, &parts);
            ui.add_space(8.0);
            let is_running = self.verify_job.is_running();
            ui.horizontal(|ui| {
                let button = icon_label_button(icons::SHIELD_CHECK, "Verify");
                if ui.add_enabled(!is_running, button).clicked() {
                    self.start_verify(ui.ctx());
                }
                if is_running {
                    ui.spinner();
                    ui.weak("Checking…");
                }
            });
            self.show_verify_outcome(ui);
        });
    }

    fn show_verify_outcome(&self, ui: &mut egui::Ui) {
        match &self.verify_job {
            JobSlot::Finished(outcome) => {
                let is_current = outcome.inputs.0 == self.verify_password
                    && outcome.inputs.1 == self.verify_hash;
                if !is_current {
                    return;
                }
                ui.add_space(6.0);
                match &outcome.is_match {
                    Ok(true) => {
                        show_status(ui, StatusTone::Success, "The password matches this hash.");
                    }
                    Ok(false) => show_status(ui, StatusTone::Error, "The password does not match."),
                    Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
                }
            }
            JobSlot::Lost => show_status(ui, StatusTone::Error, "Checking stopped unexpectedly."),
            JobSlot::Idle | JobSlot::Running(_) => {}
        }
    }
}

impl Tool for BcryptTool {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.hash_job.poll();
        self.verify_job.poll();
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("bcrypt_hash_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_hash_card(ui));
                egui::ScrollArea::vertical()
                    .id_salt("bcrypt_verify_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_verify_card(ui));
            });
        });
    }
}

fn show_hash_parts(ui: &mut egui::Ui, parts: &BcryptParts) {
    let rows = [
        ValueRow::new("Version", format!("${}$", parts.version)),
        ValueRow::new(
            "Cost",
            format!("{} ({} rounds)", parts.cost, 1_u64 << parts.cost),
        ),
        ValueRow::new("Salt", &parts.salt),
        ValueRow::new("Checksum", &parts.checksum),
    ];
    show_value_list(ui, &rows);
}

/// Costs are at most 31, so the conversion is exact.
fn to_usize(cost: u32) -> usize {
    usize::try_from(cost).unwrap_or(usize::MAX)
}
