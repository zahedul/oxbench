//! Drives the Hash Generator headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::hash::DigestEncoding;

use super::{HashGenerator, Source, describe_byte_count, describe_progress};
use crate::style::install_style;
use crate::tasks::step_until;
use crate::tool::Tool;

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn open_generator() -> Harness<'static, HashGenerator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut HashGenerator| generator.ui(ui),
            HashGenerator::default(),
        );
    harness.run();
    harness
}

fn type_text(harness: &mut Harness<'static, HashGenerator>, text: &str) {
    text.clone_into(&mut harness.state_mut().text_input);
    harness.run();
}

#[test]
fn hashes_text_with_every_algorithm() {
    let mut harness = open_generator();
    type_text(&mut harness, "abc");
    harness.get_by_label("3 bytes · UTF-8");
    harness.get_by_label(ABC_SHA256);
    harness.get_by_label("900150983cd24fb0d6963f7d28e17f72");
    harness.get_by_label("SHA3-512");
    harness.get_by_label("BLAKE3");
}

#[test]
fn empty_text_still_has_digests() {
    let harness = open_generator();
    harness.get_by_label("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn switches_the_output_encoding() {
    let mut harness = open_generator();
    type_text(&mut harness, "abc");
    harness.state_mut().encoding = DigestEncoding::Base64;
    harness.run();
    harness.get_by_label("ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
    harness.state_mut().encoding = DigestEncoding::HexUpper;
    harness.run();
    harness.get_by_label(&ABC_SHA256.to_uppercase());
}

#[test]
fn compares_a_pasted_checksum() {
    let mut harness = open_generator();
    type_text(&mut harness, "abc");
    harness.state_mut().expected_input = format!("sha256:{}", ABC_SHA256.to_uppercase());
    harness.run();
    harness.get_by_label("Matches SHA-256.");
    harness.get_by_label("SHA-256 · match");
    harness.state_mut().expected_input = "deadbeef".to_owned();
    harness.run();
    harness.get_by_label("Matches none of these digests.");
}

#[test]
fn signs_with_an_hmac_key() {
    let mut harness = open_generator();
    type_text(&mut harness, "what do ya want for nothing?");
    harness
        .get_by_label("Sign with a secret key (HMAC)")
        .click();
    harness.run();
    "Jefe".clone_into(&mut harness.state_mut().hmac_key);
    harness.run();
    harness.get_by_label("HMAC-SHA-256");
    harness.get_by_label("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    assert!(harness.query_by_label("BLAKE3").is_none());
}

#[test]
fn explains_a_bad_hmac_key() {
    let mut harness = open_generator();
    {
        let state = harness.state_mut();
        state.is_hmac = true;
        state.key_encoding = oxbench_core::hash::KeyEncoding::Hex;
        "xyz".clone_into(&mut state.hmac_key);
    }
    harness.run();
    harness.get_by_label("The key is not valid hex: use pairs of 0-9 and a-f.");
    harness.get_by_label("Fix the key to see the digests.");
}

#[test]
fn hashes_a_file_in_the_background() {
    let path = std::env::temp_dir().join(format!("oxbench-app-hash-{}.txt", std::process::id()));
    std::fs::write(&path, b"abc").unwrap();
    let mut harness = open_generator();
    let ctx = harness.ctx.clone();
    harness.state_mut().start_file_hash(&ctx, path.clone());
    step_until(&mut harness, |state| {
        state
            .file
            .as_ref()
            .is_some_and(|file| !file.job.is_running())
    });
    std::fs::remove_file(&path).unwrap();
    assert_eq!(harness.state().source, Source::File);
    harness.get_by_label("Hashed.");
    harness.get_by_label("3 bytes");
    harness.get_by_label(ABC_SHA256);
}

#[test]
fn reports_a_missing_file() {
    let mut harness = open_generator();
    let ctx = harness.ctx.clone();
    let missing = std::env::temp_dir().join("oxbench-no-such-file.bin");
    harness.state_mut().start_file_hash(&ctx, missing);
    step_until(&mut harness, |state| {
        state
            .file
            .as_ref()
            .is_some_and(|file| !file.job.is_running())
    });
    harness.get_by_label_contains("Could not open");
    harness.get_by_label("The file could not be hashed.");
}

#[test]
fn clear_empties_everything() {
    let mut harness = open_generator();
    type_text(&mut harness, "abc");
    harness.get_by_label_contains("Clear").click();
    harness.run();
    assert_eq!(harness.state().text_input, "");
    assert!(harness.state().file.is_none());
}

#[test]
fn describes_sizes_and_progress() {
    assert_eq!(describe_byte_count(1_u64), "1 byte");
    assert_eq!(describe_byte_count(1023_u64), "1023 bytes");
    assert_eq!(describe_byte_count(1536_u64), "1.5 KB");
    assert_eq!(describe_byte_count(5 * 1024 * 1024_u64), "5.0 MB");
    assert_eq!(describe_progress(50, Some(200)), "Hashing… 25%");
    assert_eq!(describe_progress(2048, None), "Hashing… 2.0 KB");
}
