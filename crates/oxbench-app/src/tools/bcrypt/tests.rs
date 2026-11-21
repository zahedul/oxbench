//! Drives the Bcrypt tool headlessly. Tests use the lowest cost so they stay fast.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oxbench_core::bcrypt::MIN_COST;

use super::{BcryptTool, to_usize};
use crate::style::install_style;
use crate::tasks::{JobSlot, step_until};
use crate::tool::Tool;

/// From the `OpenWall` `crypt_blowfish` test vectors: the password is `U*U`.
const OPENWALL_HASH: &str = "$2a$05$CCCCCCCCCCCCCCCCCCCCC.E5YPO9kmyuRGyh0XouQYb4YMJKvyOeW";

fn open_tool() -> Harness<'static, BcryptTool> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, tool: &mut BcryptTool| tool.ui(ui),
            BcryptTool::default(),
        );
    harness.run();
    harness
}

/// Buttons share words with card titles, so find the button whose label ends with `text`.
/// Steps once rather than settling, since a started job shows a spinner that keeps repainting.
fn click_button(harness: &mut Harness<'static, BcryptTool>, text: &str) {
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.ends_with(text))
        })
        .unwrap_or_else(|| panic!("no button labelled {text}"))
        .click();
    harness.step();
}

fn verify(harness: &mut Harness<'static, BcryptTool>, password: &str, hash: &str) {
    {
        let state = harness.state_mut();
        password.clone_into(&mut state.verify_password);
        hash.clone_into(&mut state.verify_hash);
    }
    harness.run();
    click_button(harness, "Verify");
    step_until(harness, |state| !state.verify_job.is_running());
}

#[test]
fn hashes_in_the_background_and_checks_the_result() {
    let mut harness = open_tool();
    {
        let state = harness.state_mut();
        "hunter2".clone_into(&mut state.hash_input);
        state.cost = to_usize(MIN_COST);
    }
    harness.run();
    click_button(&mut harness, "Hash");
    step_until(&mut harness, |state| !state.hash_job.is_running());
    let JobSlot::Finished(outcome) = &harness.state().hash_job else {
        panic!("hashing did not finish");
    };
    let hash = outcome.hash.as_ref().unwrap().clone();
    assert!(hash.starts_with("$2b$04$"));
    harness.get_by_label(&hash);
    harness.get_by_label_contains("at cost 4.");
    click_button(&mut harness, "Check it");
    harness.run();
    assert_eq!(harness.state().verify_hash, hash);
    harness.get_by_label("4 (16 rounds)");
    click_button(&mut harness, "Verify");
    step_until(&mut harness, |state| !state.verify_job.is_running());
    harness.get_by_label("The password matches this hash.");
}

#[test]
fn verifies_a_known_hash() {
    let mut harness = open_tool();
    verify(&mut harness, "U*U", OPENWALL_HASH);
    harness.get_by_label("The password matches this hash.");
    harness.get_by_label("$2a$");
    harness.get_by_label("CCCCCCCCCCCCCCCCCCCCC.");
}

#[test]
fn reports_a_wrong_password_until_the_input_changes() {
    let mut harness = open_tool();
    verify(&mut harness, "U*V", OPENWALL_HASH);
    harness.get_by_label("The password does not match.");
    "U*U".clone_into(&mut harness.state_mut().verify_password);
    harness.run();
    assert!(
        harness
            .query_by_label("The password does not match.")
            .is_none()
    );
}

#[test]
fn explains_a_malformed_hash() {
    let mut harness = open_tool();
    "not a hash".clone_into(&mut harness.state_mut().verify_hash);
    harness.run();
    harness.get_by_label("A bcrypt hash looks like $2b$12$ followed by 53 characters.");
    let verify_buttons = harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .filter(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.ends_with("Verify"))
        })
        .count();
    assert_eq!(verify_buttons, 0);
}

#[test]
fn warns_about_long_passwords() {
    let mut harness = open_tool();
    harness.state_mut().hash_input = "a".repeat(73);
    harness.run();
    harness.get_by_label("Bcrypt uses only the first 72 bytes; the rest is ignored.");
}
