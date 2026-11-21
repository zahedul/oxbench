//! Drives the Random String tool headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::random_string::RandomFormat;

use super::{RandomStringGenerator, describe_entropy};
use crate::style::install_style;
use crate::tool::Tool;

fn open_generator() -> Harness<'static, RandomStringGenerator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut RandomStringGenerator| generator.ui(ui),
            RandomStringGenerator::default(),
        );
    harness.run();
    harness
}

fn generated(harness: &Harness<'static, RandomStringGenerator>) -> Vec<String> {
    harness
        .state()
        .generated_text()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn starts_with_a_nano_id() {
    let harness = open_generator();
    let strings = generated(&harness);
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0].chars().count(), 21);
    harness.get_by_label("Characters");
    harness.get_by_label("About 126 bits of randomness each: enough for secrets and API keys.");
}

#[test]
fn regenerate_makes_a_new_string() {
    let mut harness = open_generator();
    let first = generated(&harness);
    harness.get_by_label_contains("Regenerate").click();
    harness.run();
    assert_ne!(generated(&harness), first);
}

#[test]
fn byte_formats_count_bytes() {
    let mut harness = open_generator();
    {
        let state = harness.state_mut();
        state.format = RandomFormat::Hex;
        state.length = 32;
        state.count = 3;
    }
    harness.run();
    harness.get_by_label("Bytes");
    assert!(harness.query_by_label("Readability").is_none());
    let strings = generated(&harness);
    assert_eq!(strings.len(), 3);
    assert!(strings.iter().all(|s| s.len() == 64));
    harness.get_by_label_contains("About 256 bits");
}

#[test]
fn custom_alphabets_are_respected_and_checked() {
    let mut harness = open_generator();
    {
        let state = harness.state_mut();
        state.format = RandomFormat::Custom;
        "xy".clone_into(&mut state.custom_alphabet);
        state.length = 40;
    }
    harness.run();
    assert!(generated(&harness)[0].chars().all(|c| c == 'x' || c == 'y'));
    "zz".clone_into(&mut harness.state_mut().custom_alphabet);
    harness.run();
    harness.get_by_label("The alphabet needs at least two different characters.");
}

#[test]
fn describes_how_strong_the_strings_are() {
    assert!(describe_entropy(20.0).contains("guessable"));
    assert!(describe_entropy(80.0).contains("fine for IDs"));
    assert!(describe_entropy(128.0).contains("secrets and API keys"));
}
