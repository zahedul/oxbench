//! Drives Lorem Ipsum headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::lorem::LoremUnit;

use super::{LoremIpsum, OutputFormat};
use crate::style::install_style;
use crate::tool::Tool;

fn open_lorem() -> Harness<'static, LoremIpsum> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, lorem: &mut LoremIpsum| lorem.ui(ui),
            LoremIpsum::default(),
        );
    harness.run();
    harness
}

fn output_text(harness: &Harness<'static, LoremIpsum>) -> String {
    harness
        .state()
        .output
        .as_ref()
        .map_or_else(String::new, |cached| cached.value.text.clone())
}

#[test]
fn starts_with_three_classic_paragraphs() {
    let harness = open_lorem();
    let text = output_text(&harness);
    assert!(text.starts_with("Lorem ipsum dolor sit amet, consectetur adipiscing elit"));
    assert_eq!(text.split("\n\n").count(), 3);
    harness.get_by_label_contains("3 paragraphs · ");
}

#[test]
fn regenerate_writes_new_text() {
    let mut harness = open_lorem();
    let first = output_text(&harness);
    harness.get_by_label_contains("Regenerate").click();
    harness.run();
    assert_ne!(output_text(&harness), first);
}

#[test]
fn words_and_html_follow_the_options() {
    let mut harness = open_lorem();
    {
        let lorem = harness.state_mut();
        lorem.spec.unit = LoremUnit::Words;
        lorem.spec.count = 12;
        lorem.format = OutputFormat::Html;
    }
    harness.run();
    let text = output_text(&harness);
    assert!(text.starts_with("<p>Lorem ipsum"));
    assert!(text.ends_with("</p>"));
    harness.get_by_label_contains("12 words");
}
