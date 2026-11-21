//! JWT Decoder: the token on the left with signature verification; its timing, header,
//! payload and claims on the right.

mod token_editor;

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::epoch::{DateTime, Utc, describe_relative, format_readable};
use oxbench_core::json::{FormatOptions, Value, format_pretty};
use oxbench_core::jwt::{
    ClaimRow, DecodedJwt, KeyKind, Payload, SecretEncoding, TokenTiming, check_timing, decode_jwt,
    describe_claims, key_kind_for, verify_signature,
};
use oxbench_core::{CoreError, CoreResult};

use crate::cached::Cached;
use crate::clock::Clock;
use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::code_editor::show_code_block;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::tool_header::show_tool_header;
use crate::widgets::value_list::{ValueRow, show_value_list};
use token_editor::show_token_editor;

/// HS256, signed with `SAMPLE_SECRET`, expiring on 2100-01-01.
const SAMPLE_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkFkYSBMb3ZlbGFjZSIsImVtYWlsIjoiYWRhQGV4YW1wbGUuY29tIiwicm9sZXMiOlsiYWRtaW4iLCJlZGl0b3IiXSwiaWF0IjoxNzAwMDAwMDAwLCJuYmYiOjE3MDAwMDAwMDAsImV4cCI6NDEwMjQ0NDgwMH0.\
5oZySYSEHtitx76e9haqbX5TeysbVWZhzvGAPy2JkwY";
const SAMPLE_SECRET: &str = "oxbench-sample-secret";

const MIN_BODY_HEIGHT: f32 = 160.0;
const KEY_EDITOR_ROWS: usize = 6;

/// A decoded token with everything the right-hand pane shows, computed once per token.
struct DecodedView {
    jwt: DecodedJwt,
    header_json: String,
    payload_text: String,
    claims: Vec<ClaimRow>,
}

type VerifyKey = (String, String, SecretEncoding);

pub struct JwtDecoder {
    clock: Clock,
    token_input: String,
    key_input: String,
    secret_encoding: SecretEncoding,
    decoded: Option<Cached<String, CoreResult<DecodedView>>>,
    verification: Option<Cached<VerifyKey, CoreResult<bool>>>,
}

impl Default for JwtDecoder {
    fn default() -> Self {
        Self::new(Clock::System)
    }
}

impl JwtDecoder {
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            token_input: String::new(),
            key_input: String::new(),
            secret_encoding: SecretEncoding::default(),
            decoded: None,
            verification: None,
        }
    }

    fn refresh_decoded(&mut self) {
        let is_fresh = self
            .decoded
            .as_ref()
            .is_some_and(|cached| cached.key == self.token_input);
        if is_fresh {
            return;
        }
        self.decoded = Some(Cached {
            key: self.token_input.clone(),
            value: decode_jwt(&self.token_input).map(build_view),
        });
    }

    fn decoded_view(&self) -> Option<&DecodedView> {
        self.decoded.as_ref()?.value.as_ref().ok()
    }

    fn refresh_verification(&mut self) {
        let Some(view) = self.decoded_view() else {
            self.verification = None;
            return;
        };
        let key = (
            view.jwt.token.clone(),
            self.key_input.clone(),
            self.secret_encoding,
        );
        let is_fresh = self
            .verification
            .as_ref()
            .is_some_and(|cached| cached.key == key);
        if is_fresh {
            return;
        }
        let value = verify_signature(&view.jwt, &self.key_input, self.secret_encoding);
        self.verification = Some(Cached { key, value });
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        show_tool_header(
            ui,
            |_| {},
            |ui| {
                if ui.add(icon_label_button(icons::ERASER, "Clear")).clicked() {
                    self.token_input.clear();
                    self.key_input.clear();
                }
                if ui
                    .add(icon_label_button(icons::CLIPBOARD_TEXT, "Sample"))
                    .on_hover_text("Load an HS256 token and its secret")
                    .clicked()
                {
                    SAMPLE_TOKEN.clone_into(&mut self.token_input);
                    SAMPLE_SECRET.clone_into(&mut self.key_input);
                    self.secret_encoding = SecretEncoding::Text;
                }
            },
        );
    }

    fn show_token_pane(&mut self, ui: &mut egui::Ui) {
        show_pane_header(ui, "Encoded token", |_| {});
        show_token_editor(ui, &mut self.token_input);
        ui.add_space(6.0);
        if self.token_input.trim().is_empty() {
            show_status(ui, StatusTone::Neutral, "Paste a token to decode it.");
        } else if let Some(Cached {
            value: Err(error), ..
        }) = &self.decoded
        {
            show_status(ui, StatusTone::Error, &error.to_string());
        }
        ui.add_space(12.0);
        self.show_verify_card(ui);
    }

    fn show_verify_card(&mut self, ui: &mut egui::Ui) {
        let algorithm = self
            .decoded_view()
            .map(|view| view.jwt.algorithm.clone().unwrap_or_default());
        show_card(ui, "Verify signature", |ui| {
            let Some(algorithm) = algorithm else {
                ui.weak("Decode a token to check its signature.");
                return;
            };
            let Some(key_kind) = key_kind_for(&algorithm) else {
                show_verification(
                    ui,
                    self.verification.as_ref().map(|cached| &cached.value),
                    "",
                );
                return;
            };
            match key_kind {
                KeyKind::Secret => {
                    ui.weak(format!("{algorithm} uses a shared secret."));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.key_input)
                            .id_salt("jwt_secret")
                            .font(egui::TextStyle::Monospace)
                            .hint_text("Secret")
                            .desired_width(f32::INFINITY),
                    );
                    let mut is_base64 = self.secret_encoding == SecretEncoding::Base64;
                    if ui.checkbox(&mut is_base64, "Secret is Base64").changed() {
                        self.secret_encoding = if is_base64 {
                            SecretEncoding::Base64
                        } else {
                            SecretEncoding::Text
                        };
                    }
                }
                KeyKind::PublicKey => {
                    ui.weak(format!(
                        "{algorithm} uses a public key: PEM, JWK or JWK Set."
                    ));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.key_input)
                            .id_salt("jwt_public_key")
                            .code_editor()
                            .hint_text("-----BEGIN PUBLIC KEY-----")
                            .desired_width(f32::INFINITY)
                            .desired_rows(KEY_EDITOR_ROWS),
                    );
                }
            }
            ui.add_space(4.0);
            if self.key_input.trim().is_empty() {
                return;
            }
            let wrong_key_message = match key_kind {
                KeyKind::Secret => "Signature does not match this secret.",
                KeyKind::PublicKey => "Signature does not match this key.",
            };
            show_verification(
                ui,
                self.verification.as_ref().map(|cached| &cached.value),
                wrong_key_message,
            );
        });
    }

    fn show_decoded_pane(&self, ui: &mut egui::Ui) {
        let Some(view) = self.decoded_view() else {
            show_pane_header(ui, "Decoded", |_| {});
            ui.weak("The header, payload and claims appear here.");
            return;
        };
        if let Payload::Json(payload) = &view.jwt.payload {
            show_timing(ui, payload, self.clock);
            ui.add_space(8.0);
        }
        show_pane_header(ui, "Header", |ui| {
            show_copy_button(ui, "Copy header", &view.header_json);
        });
        show_code_block(ui, "jwt_header", &view.header_json);
        ui.add_space(8.0);
        show_pane_header(ui, "Payload", |ui| {
            show_copy_button(ui, "Copy payload", &view.payload_text);
        });
        show_code_block(ui, "jwt_payload", &view.payload_text);
        if view.claims.is_empty() {
            return;
        }
        ui.add_space(8.0);
        show_pane_header(ui, "Claims", |_| {});
        let now = self.clock.now();
        let rows: Vec<ValueRow> = view
            .claims
            .iter()
            .map(|claim| describe_claim_row(claim, now))
            .collect();
        show_value_list(ui, &rows);
    }
}

impl Tool for JwtDecoder {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.refresh_decoded();
        self.refresh_verification();
        self.show_header(ui);
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("jwt_token_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_token_pane(ui));
                egui::ScrollArea::vertical()
                    .id_salt("jwt_decoded_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_decoded_pane(ui));
            });
        });
        // Text typed this frame is decoded on the next one, so ask for it straight away.
        if self
            .decoded
            .as_ref()
            .is_some_and(|cached| cached.key != self.token_input)
        {
            ui.ctx().request_repaint();
        }
    }
}

fn build_view(jwt: DecodedJwt) -> DecodedView {
    let header_json = pretty_json(&jwt.header);
    let (payload_text, claims) = match &jwt.payload {
        Payload::Json(value) => (pretty_json(value), describe_claims(value)),
        Payload::Text(text) => (text.clone(), Vec::new()),
    };
    DecodedView {
        jwt,
        header_json,
        payload_text,
        claims,
    }
}

/// Formatting a parsed value cannot fail in practice; the compact form is the fallback.
fn pretty_json(value: &Value) -> String {
    format_pretty(value, FormatOptions::default()).unwrap_or_else(|_| value.to_string())
}

fn show_verification(
    ui: &mut egui::Ui,
    result: Option<&CoreResult<bool>>,
    wrong_key_message: &str,
) {
    match result {
        Some(Ok(true)) => show_status(ui, StatusTone::Success, "Signature verified."),
        Some(Ok(false)) => show_status(ui, StatusTone::Error, wrong_key_message),
        Some(Err(CoreError::InvalidInput(message) | CoreError::InvalidJson { message, .. })) => {
            show_status(ui, StatusTone::Error, message);
        }
        None => {}
    }
}

fn show_timing(ui: &mut egui::Ui, payload: &Value, clock: Clock) {
    let now = clock.now();
    let (tone, text) = match check_timing(payload, now) {
        TokenTiming::NoExpiry => (
            StatusTone::Neutral,
            "No expiry: the token has no exp claim.".to_owned(),
        ),
        TokenTiming::Active { expires_at } => (
            StatusTone::Success,
            format!(
                "Active · expires {} · {}",
                describe_relative(expires_at, now),
                format_readable(expires_at.fixed_offset())
            ),
        ),
        TokenTiming::Expired { expired_at } => (
            StatusTone::Error,
            format!(
                "Expired {} · {}",
                describe_relative(expired_at, now),
                format_readable(expired_at.fixed_offset())
            ),
        ),
        TokenTiming::NotYetValid { valid_from } => (
            StatusTone::Warning,
            format!(
                "Not valid yet · starts {} · {}",
                describe_relative(valid_from, now),
                format_readable(valid_from.fixed_offset())
            ),
        ),
    };
    show_status(ui, tone, &text);
}

fn describe_claim_row(claim: &ClaimRow, now: DateTime<Utc>) -> ValueRow {
    let label = match claim.meaning {
        Some(meaning) => format!("{} · {meaning}", claim.name),
        None => claim.name.clone(),
    };
    let value = match claim.time {
        Some(time) => format!(
            "{} · {} ({})",
            claim.value,
            format_readable(time.fixed_offset()),
            describe_relative(time, now)
        ),
        None => claim.value.clone(),
    };
    ValueRow::new(label, value).copying(&claim.value)
}
