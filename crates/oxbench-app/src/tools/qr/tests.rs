//! Drives the QR Generator headlessly.

use eframe::egui::{self, Color32};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::qr::ErrorCorrection;

use super::{ContentKind, QrGenerator};
use crate::style::install_style;
use crate::tool::Tool;

fn open_generator() -> Harness<'static, QrGenerator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, generator: &mut QrGenerator| generator.ui(ui),
            QrGenerator::default(),
        );
    harness.run();
    harness
}

#[test]
fn starts_with_a_hint() {
    let harness = open_generator();
    harness.get_by_label("Type text or a link to make a QR code.");
}

#[test]
fn text_shows_a_preview_with_its_size() {
    let mut harness = open_generator();
    "https://example.com".clone_into(&mut harness.state_mut().text);
    harness.run();
    harness.get_by_label("QR code preview");
    harness.get_by_label("Version 2 · 25 × 25 modules · Medium (15%) error correction");
    harness.get_by_label("19 bytes to encode.");
}

#[test]
fn wifi_codes_need_a_network_name() {
    let mut harness = open_generator();
    harness.get_by_label("Wi-Fi").click();
    harness.run();
    assert_eq!(harness.state().content_kind, ContentKind::Wifi);
    harness.get_by_label("Enter the network name to make a join code.");
    {
        let generator = harness.state_mut();
        "Home".clone_into(&mut generator.wifi.ssid);
        "hunter22".clone_into(&mut generator.wifi.password);
    }
    harness.run();
    assert_eq!(harness.state().payload(), "WIFI:T:WPA;S:Home;P:hunter22;;");
    harness.get_by_label("QR code preview");
}

#[test]
fn oversized_text_explains_the_limit() {
    let mut harness = open_generator();
    {
        let generator = harness.state_mut();
        generator.text = "x".repeat(2000);
        generator.correction = ErrorCorrection::High;
    }
    harness.run();
    harness.get_by_label(
        "Too much to fit: 2000 bytes, but a QR code holds at most 1273 at High (30%) error correction.",
    );
}

#[test]
fn warns_about_low_contrast_colours() {
    let mut harness = open_generator();
    "hi".clone_into(&mut harness.state_mut().text);
    harness.state_mut().light = Color32::from_rgb(40, 40, 40);
    harness.run();
    harness.get_by_label("Low contrast: scanners need a dark code on a light background.");
}
