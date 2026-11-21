//! Drives the JWT Decoder headlessly. The clock is pinned to 1700000000
//! (Tuesday, 14 November 2023 22:13:20 UTC).

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::epoch::DateTime;
use oxbench_core::jwt::SecretEncoding;

use super::token_editor::color_token;
use super::{JwtDecoder, SAMPLE_SECRET, SAMPLE_TOKEN};
use crate::clock::Clock;
use crate::style::install_style;
use crate::style::palette::DARK;
use crate::tool::Tool;

/// HS256 with secret `your-256-bit-secret`; it has `iat` but no `exp`.
const NO_EXPIRY_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
/// `{"alg":"RS256"}.{"sub":"42"}` with a placeholder signature.
const RS256_TOKEN: &str = "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiI0MiJ9.c2lnbmF0dXJl";

fn open_decoder_at(seconds: i64) -> Harness<'static, JwtDecoder> {
    let pinned = DateTime::from_timestamp(seconds, 0).unwrap_or_default();
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, decoder: &mut JwtDecoder| decoder.ui(ui),
            JwtDecoder::new(Clock::Fixed(pinned)),
        );
    harness.run();
    harness
}

fn open_decoder() -> Harness<'static, JwtDecoder> {
    open_decoder_at(1_700_000_000)
}

fn set_inputs(harness: &mut Harness<'static, JwtDecoder>, token: &str, key: &str) {
    token.clone_into(&mut harness.state_mut().token_input);
    key.clone_into(&mut harness.state_mut().key_input);
    harness.run();
}

fn load_sample(harness: &mut Harness<'static, JwtDecoder>) {
    harness.get_by_label_contains("Sample").click();
    harness.run();
}

#[test]
fn starts_with_hints() {
    let harness = open_decoder();
    harness.get_by_label("Paste a token to decode it.");
    harness.get_by_label("Decode a token to check its signature.");
    harness.get_by_label("The header, payload and claims appear here.");
}

#[test]
fn sample_decodes_and_verifies() {
    let mut harness = open_decoder();
    load_sample(&mut harness);
    assert_eq!(harness.state().token_input, SAMPLE_TOKEN);
    assert_eq!(harness.state().key_input, SAMPLE_SECRET);
    harness.get_by_label("Signature verified.");
    harness.get_by_label_contains("Active · expires in 76 years · Friday, 1 January 2100");
    harness.get_by_label("Ada Lovelace");
    harness.get_by_label("name · Name");
    harness.get_by_label(r#"["admin","editor"]"#);
    harness.get_by_label("exp · Expires");
    harness.get_by_label_contains("4102444800 · Friday, 1 January 2100 00:00:00 UTC (in 76 years)");
    let header = harness
        .state()
        .decoded_view()
        .map(|view| view.header_json.clone());
    assert_eq!(
        header.as_deref(),
        Some("{\n  \"alg\": \"HS256\",\n  \"typ\": \"JWT\"\n}")
    );
}

#[test]
fn a_wrong_secret_does_not_match() {
    let mut harness = open_decoder();
    set_inputs(&mut harness, SAMPLE_TOKEN, "not the secret");
    harness.get_by_label("Signature does not match this secret.");
}

#[test]
fn a_base64_secret_needs_the_option() {
    let mut harness = open_decoder();
    // "oxbench-sample-secret" in Base64.
    set_inputs(&mut harness, SAMPLE_TOKEN, "b3hiZW5jaC1zYW1wbGUtc2VjcmV0");
    harness.get_by_label("Signature does not match this secret.");
    harness.get_by_label("Secret is Base64").click();
    harness.run();
    assert_eq!(harness.state().secret_encoding, SecretEncoding::Base64);
    harness.get_by_label("Signature verified.");
}

#[test]
fn reports_expired_tokens() {
    let mut harness = open_decoder_at(4_200_000_000);
    load_sample(&mut harness);
    harness.get_by_label_contains("Expired 3 years ago");
}

#[test]
fn reports_tokens_that_are_not_valid_yet() {
    let mut harness = open_decoder_at(1_600_000_000);
    load_sample(&mut harness);
    harness.get_by_label_contains("Not valid yet · starts in 3 years");
}

#[test]
fn a_token_without_exp_never_expires() {
    let mut harness = open_decoder();
    set_inputs(&mut harness, NO_EXPIRY_TOKEN, "your-256-bit-secret");
    harness.get_by_label("No expiry: the token has no exp claim.");
    harness.get_by_label("Signature verified.");
    harness.get_by_label_contains("1516239022 · Thursday, 18 January 2018");
}

#[test]
fn public_key_algorithms_ask_for_a_key() {
    let mut harness = open_decoder();
    set_inputs(&mut harness, RS256_TOKEN, "");
    harness.get_by_label("RS256 uses a public key: PEM, JWK or JWK Set.");
    assert!(harness.query_by_label("Signature verified.").is_none());
    set_inputs(&mut harness, RS256_TOKEN, "not a key");
    harness.get_by_label_contains("RS256 accepts");
}

#[test]
fn unsigned_tokens_are_explained() {
    let mut harness = open_decoder();
    set_inputs(&mut harness, "eyJhbGciOiJub25lIn0.eyJzdWIiOiI0MiJ9.", "");
    harness.get_by_label_contains("This token is unsigned");
}

#[test]
fn malformed_tokens_show_the_reason() {
    let mut harness = open_decoder();
    set_inputs(&mut harness, "abc.def", "");
    harness.get_by_label("A JWT has three parts separated by dots; this has 2.");
    harness.get_by_label("Decode a token to check its signature.");
}

#[test]
fn clear_empties_both_inputs() {
    let mut harness = open_decoder();
    load_sample(&mut harness);
    harness.get_by_label_contains("Clear").click();
    harness.run();
    assert_eq!(harness.state().token_input, "");
    assert_eq!(harness.state().key_input, "");
    harness.get_by_label("Paste a token to decode it.");
}

#[test]
fn colors_each_token_part() {
    let font = egui::FontId::monospace(13.0);
    let job = color_token("aa.bb.cc", &DARK, &font);
    let colors: Vec<egui::Color32> = job
        .sections
        .iter()
        .map(|section| section.format.color)
        .collect();
    assert_eq!(
        colors,
        [
            DARK.error,
            DARK.text_weak,
            DARK.accent,
            DARK.text_weak,
            DARK.syntax_string
        ]
    );
    assert_eq!(job.text, "aa.bb.cc");
}
