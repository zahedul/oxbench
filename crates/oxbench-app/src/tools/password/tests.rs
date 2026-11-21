//! Drives the Password Generator headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::{Mode, PasswordGenerator};
use crate::style::install_style;
use crate::tool::Tool;

fn open_generator() -> Harness<'static, PasswordGenerator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut PasswordGenerator| generator.ui(ui),
            PasswordGenerator::default(),
        );
    harness.run();
    harness
}

fn generated(harness: &Harness<'static, PasswordGenerator>) -> String {
    harness
        .state()
        .generated_password()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn starts_with_a_strong_password() {
    let harness = open_generator();
    let password = generated(&harness);
    assert_eq!(password.chars().count(), 20);
    harness.get_by_label("Strength: Very strong");
    harness.get_by_label("About 130 bits of randomness.");
}

#[test]
fn regenerate_and_options_make_new_passwords() {
    let mut harness = open_generator();
    let first = generated(&harness);
    harness.get_by_label_contains("Regenerate").click();
    harness.run();
    assert_ne!(generated(&harness), first);
    harness.get_by_label("!#$%").click();
    harness.run();
    assert!(generated(&harness).chars().all(char::is_alphanumeric));
}

#[test]
fn explains_when_nothing_is_chosen() {
    let mut harness = open_generator();
    {
        let spec = &mut harness.state_mut().password;
        spec.has_lowercase = false;
        spec.has_uppercase = false;
        spec.has_digits = false;
        spec.has_symbols = false;
    }
    harness.run();
    harness.get_by_label("Choose at least one kind of character.");
}

#[test]
fn makes_passphrases() {
    let mut harness = open_generator();
    harness.get_by_label("Passphrase").click();
    harness.run();
    assert_eq!(harness.state().mode, Mode::Passphrase);
    let phrase = generated(&harness);
    assert_eq!(phrase.split('-').count(), 5);
    harness.get_by_label("About 65 bits of randomness.");
    " ".clone_into(&mut harness.state_mut().passphrase.separator);
    harness.run();
    assert_eq!(generated(&harness).split(' ').count(), 5);
}

#[test]
fn rates_a_typed_password() {
    let mut harness = open_generator();
    harness.get_by_label("Looks for words, names, dates, keyboard runs and repeats.");
    "password1".clone_into(&mut harness.state_mut().check_input);
    harness.run();
    assert_eq!(harness.query_all_by_label("Strength: Very weak").count(), 1);
    harness.get_by_label_contains("common password");
}

#[test]
fn hides_the_checked_password_until_asked() {
    let mut harness = open_generator();
    harness.get_by_label("Show password").click();
    harness.run();
    assert!(harness.state().is_check_revealed);
    harness.get_by_label("Hide password");
}
