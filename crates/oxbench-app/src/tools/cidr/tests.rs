//! Drives the IP / CIDR Calculator headlessly.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::{CidrCalculator, CidrView};
use crate::style::install_style;
use crate::tool::Tool;

fn open_calculator() -> Harness<'static, CidrCalculator> {
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, calculator: &mut CidrCalculator| calculator.ui(ui),
            CidrCalculator::default(),
        );
    harness.run();
    harness
}

fn enter_network(harness: &mut Harness<'static, CidrCalculator>, network: &str) {
    network.clone_into(&mut harness.state_mut().network_input);
    harness.run();
}

#[test]
fn starts_with_a_hint() {
    let harness = open_calculator();
    harness.get_by_label(
        "Enter an address with a prefix or netmask to see the network it belongs to.",
    );
}

#[test]
fn sample_describes_its_network() {
    let mut harness = open_calculator();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    harness.get_by_label("192.168.1.0/24");
    harness.get_by_label("Private (RFC 1918)");
    harness.get_by_label("255.255.255.0");
    harness.get_by_label("192.168.1.255");
    harness.get_by_label("192.168.1.1 – 192.168.1.254");
    harness.get_by_label("254");
    harness.get_by_label("1.168.192.in-addr.arpa");
    harness.get_by_label("24 network bits, 8 host bits.");
}

#[test]
fn netmask_input_works_like_a_prefix() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "10.1.2.3 255.255.0.0");
    harness.get_by_label("10.1.0.0/16");
    harness.get_by_label("65,534");
}

#[test]
fn bad_input_explains_itself() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "10.0.0.0/40");
    harness.get_by_label("/40 is too long: IPv4 prefixes go up to /32.");
}

#[test]
fn ipv6_networks_show_their_expanded_form() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "2001:db8::1/48");
    harness.get_by_label("2001:db8::/48");
    harness.get_by_label("Documentation");
    // Once in the bits view, once as a copyable value.
    let expanded = "2001:0db8:0000:0000:0000:0000:0000:0001";
    assert_eq!(harness.get_all_by_label(expanded).count(), 2);
    harness.get_by_label("1,208,925,819,614,629,174,706,176 (2^80)");
}

#[test]
fn checks_whether_an_address_is_inside() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "192.168.1.0/24");
    "192.168.1.77".clone_into(&mut harness.state_mut().check_input);
    harness.run();
    harness.get_by_label("192.168.1.77 is in 192.168.1.0/24, at address 77 of 256.");
    "10.0.0.0/8".clone_into(&mut harness.state_mut().check_input);
    harness.run();
    harness.get_by_label("10.0.0.0/8 isn't in 192.168.1.0/24.");
}

#[test]
fn split_lists_equal_subnets() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "192.168.1.0/24");
    harness.get_by_label("Split").click();
    harness.run();
    harness.get_by_label("4 subnets of 64 addresses, 62 usable hosts each.");
    harness.get_by_label("192.168.1.192/26");
    harness.get_by_label("192.168.1.65 – 192.168.1.126");
}

#[test]
fn a_single_address_cannot_be_split() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "10.0.0.1");
    harness.state_mut().view = CidrView::Split;
    harness.run();
    harness.get_by_label("A /32 network holds a single address, so it can't be split.");
}

#[test]
fn merge_sample_combines_neighbours_and_ranges() {
    let mut harness = open_calculator();
    harness.get_by_label("Merge").click();
    harness.run();
    harness.get_by_label_contains("Sample").click();
    harness.run();
    harness.get_by_label("6 entries merged into 3 networks.");
    assert_eq!(
        harness.state().merge.merged_text(),
        Some("10.0.0.0/28\n192.168.0.0/23\n192.168.2.0/24")
    );
}

#[test]
fn merge_reports_lines_it_cannot_read() {
    let mut harness = open_calculator();
    harness.state_mut().view = CidrView::Merge;
    "10.0.0.0/24\nbanana".clone_into(&mut harness.state_mut().merge.input);
    harness.run();
    harness.get_by_label("1 entry merged into 1 network.");
    harness.get_by_label("Line 2: \"banana\" isn't an IP address.");
}

#[test]
fn clear_only_empties_the_current_view() {
    let mut harness = open_calculator();
    enter_network(&mut harness, "10.0.0.0/8");
    harness.state_mut().merge.load_sample();
    harness.get_by_label_contains("Clear").click();
    harness.run();
    assert_eq!(harness.state().network_input, "");
    assert_ne!(harness.state().merge.input, "");
}
