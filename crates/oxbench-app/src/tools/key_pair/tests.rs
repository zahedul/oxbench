//! Drives the Key Pair Generator headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oxbench_core::keypair::KeyAlgorithm;

use super::{KeyPairGenerator, PrivateFormat};
use crate::style::install_style;
use crate::tasks::step_until;
use crate::tool::Tool;

fn open_generator() -> Harness<'static, KeyPairGenerator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut KeyPairGenerator| generator.ui(ui),
            KeyPairGenerator::default(),
        );
    harness.step();
    step_until(&mut harness, |generator| generator.key_pair().is_some());
    harness
}

fn click_generate(harness: &mut Harness<'static, KeyPairGenerator>) {
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.ends_with("Generate"))
        })
        .unwrap_or_else(|| panic!("no Generate button"))
        .click();
    harness.step();
}

/// Text boxes don't expose their contents to queries, so this reads what the pane shows.
fn shown_private_key(harness: &Harness<'static, KeyPairGenerator>) -> String {
    let generator = harness.state();
    let key_pair = generator.key_pair().unwrap();
    match generator.private_format {
        PrivateFormat::OpenSsh => key_pair.private_openssh.clone(),
        PrivateFormat::Pkcs8 => key_pair.private_pem.clone(),
    }
}

#[test]
fn opens_with_an_ed25519_key() {
    let harness = open_generator();
    let key_pair = harness.state().key_pair().unwrap();
    assert_eq!(key_pair.algorithm, KeyAlgorithm::Ed25519);
    assert!(key_pair.public_openssh.starts_with("ssh-ed25519 "));
    harness.get_by_label(&key_pair.fingerprint);
    assert!(shown_private_key(&harness).starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
}

#[test]
fn generates_the_chosen_algorithm_with_the_comment() {
    let mut harness = open_generator();
    {
        let generator = harness.state_mut();
        generator.algorithm = KeyAlgorithm::EcdsaP256;
        "me@laptop".clone_into(&mut generator.comment);
    }
    harness.run();
    click_generate(&mut harness);
    step_until(&mut harness, |generator| {
        generator
            .key_pair()
            .is_some_and(|key_pair| key_pair.algorithm == KeyAlgorithm::EcdsaP256)
    });
    let public_key = harness.state().key_pair().unwrap().public_openssh.clone();
    assert!(public_key.starts_with("ecdsa-sha2-nistp256 "));
    assert!(public_key.ends_with(" me@laptop"));
}

#[test]
fn the_private_key_switches_between_formats() {
    let mut harness = open_generator();
    harness.get_by_label("PKCS #8 PEM").click();
    harness.run();
    assert_eq!(harness.state().private_format, PrivateFormat::Pkcs8);
    assert!(shown_private_key(&harness).starts_with("-----BEGIN PRIVATE KEY-----"));
}
