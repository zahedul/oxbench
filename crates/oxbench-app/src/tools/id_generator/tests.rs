//! Drives the UUID / ULID tool headlessly. The clock is pinned to 1700000000.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::epoch::DateTime;
use oxbench_core::id::{IdKind, Namespace};

use super::IdGenerator;
use crate::clock::Clock;
use crate::style::install_style;
use crate::tool::Tool;

fn open_generator() -> Harness<'static, IdGenerator> {
    let pinned = DateTime::from_timestamp(1_700_000_000, 0).unwrap_or_default();
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut IdGenerator| generator.ui(ui),
            IdGenerator::new(Clock::Fixed(pinned)),
        );
    harness.run();
    harness
}

fn generated_ids(harness: &Harness<'static, IdGenerator>) -> Vec<String> {
    harness
        .state()
        .generated_text()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn starts_with_one_v4_uuid() {
    let harness = open_generator();
    let ids = generated_ids(&harness);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].len(), 36);
    assert_eq!(&ids[0][14..15], "4");
}

#[test]
fn regenerate_makes_a_new_id() {
    let mut harness = open_generator();
    let first = generated_ids(&harness);
    harness.get_by_label_contains("Regenerate").click();
    harness.run();
    assert_ne!(generated_ids(&harness), first);
}

#[test]
fn generates_many_in_the_chosen_format() {
    let mut harness = open_generator();
    {
        let state = harness.state_mut();
        state.kind = IdKind::UuidV7;
        state.count = 5;
        state.format.is_uppercase = true;
        state.format.has_hyphens = false;
    }
    harness.run();
    let ids = generated_ids(&harness);
    assert_eq!(ids.len(), 5);
    assert!(
        ids.iter()
            .all(|id| id.len() == 32 && id == &id.to_uppercase())
    );
    assert!(ids.iter().all(|id| &id[12..13] == "7"));
}

#[test]
fn name_based_ids_follow_the_inputs() {
    let mut harness = open_generator();
    harness.state_mut().kind = IdKind::UuidV5;
    harness.run();
    harness.get_by_label("Name");
    harness.get_by_label("The same namespace and name always give the same UUID.");
    assert!(harness.query_by_label_contains("Regenerate").is_none());
    assert!(harness.query_by_label("How many").is_none());
    "python.org".clone_into(&mut harness.state_mut().name);
    harness.run();
    assert_eq!(
        generated_ids(&harness),
        ["886313e1-3b8a-5372-9b90-0c9aee199e5d"]
    );
}

#[test]
fn explains_a_bad_custom_namespace() {
    let mut harness = open_generator();
    {
        let state = harness.state_mut();
        state.kind = IdKind::UuidV3;
        state.namespace = Namespace::Custom;
        "not a uuid".clone_into(&mut state.custom_namespace);
    }
    harness.run();
    harness.get_by_label("The custom namespace must be a UUID.");
}

#[test]
fn switching_to_ulid_uses_uppercase() {
    let mut harness = open_generator();
    let previous = harness.state().kind;
    harness.state_mut().kind = IdKind::Ulid;
    harness.state_mut().change_kind(previous);
    harness.run();
    let ids = generated_ids(&harness);
    assert_eq!(ids[0].len(), 26);
    assert_eq!(ids[0], ids[0].to_uppercase());
    assert!(harness.query_by_label("Hyphens").is_none());
}

#[test]
fn inspects_a_pasted_uuid() {
    let mut harness = open_generator();
    "0190a5d2-ac96-774b-bcce-b302099a8057".clone_into(&mut harness.state_mut().inspect_input);
    harness.run();
    harness.get_by_label("UUID version 7 · Unix time, sortable");
    harness.get_by_label("RFC 9562");
    harness.get_by_label_contains("Friday, 12 July 2024");
    harness.get_by_label_contains("UTC (in ");
}

#[test]
fn explains_an_unreadable_id() {
    let mut harness = open_generator();
    "hello".clone_into(&mut harness.state_mut().inspect_input);
    harness.run();
    harness.get_by_label_contains("Paste a UUID (such as");
}
