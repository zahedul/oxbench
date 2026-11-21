//! The address and mask written out bit by bit, with network bits in the accent colour.

use std::net::IpAddr;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32};
use oxbench_core::cidr::Interface;

use crate::style::palette::Palette;
use crate::widgets::form::{show_custom_row, show_form};

const IPV4_BITS: usize = 32;
const BITS_PER_HEX_DIGIT: usize = 4;

pub fn show_bits(ui: &mut egui::Ui, interface: Interface) {
    let palette = Palette::of(ui);
    let network = interface.network;
    let prefix_len = usize::from(network.prefix_len());
    let host_bits = usize::from(network.max_prefix_len()) - prefix_len;
    let colors = BitColors {
        network: palette.accent,
        host: palette.text_weak,
        separator: palette.border_strong,
    };
    show_form(ui, "cidr_bits", |ui| match interface.address {
        IpAddr::V4(address) => {
            let netmask = u32::MAX
                .checked_shl(u32::try_from(host_bits).unwrap_or(u32::MAX))
                .unwrap_or(0);
            show_bit_row(
                ui,
                "Address",
                &binary_digits(u32::from(address)),
                prefix_len,
                1,
                colors,
            );
            show_bit_row(
                ui,
                "Netmask",
                &binary_digits(netmask),
                prefix_len,
                1,
                colors,
            );
        }
        IpAddr::V6(address) => {
            let digits = format!("{:032x}", u128::from(address));
            show_bit_row(
                ui,
                "Address",
                &digits,
                prefix_len,
                BITS_PER_HEX_DIGIT,
                colors,
            );
        }
    });
    ui.add_space(6.0);
    ui.weak(format!("{prefix_len} network bits, {host_bits} host bits."));
}

#[derive(Clone, Copy)]
struct BitColors {
    network: Color32,
    host: Color32,
    separator: Color32,
}

fn binary_digits(value: u32) -> String {
    format!("{value:0IPV4_BITS$b}")
}

/// `digits` each stand for `bits_per_digit` bits; a digit is coloured as network when its
/// first bit is.
fn show_bit_row(
    ui: &mut egui::Ui,
    label: &str,
    digits: &str,
    prefix_len: usize,
    bits_per_digit: usize,
    colors: BitColors,
) {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let group_size = if bits_per_digit == 1 { 8 } else { 4 };
    let separator = if bits_per_digit == 1 { "." } else { ":" };
    let mut job = LayoutJob::default();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && index % group_size == 0 {
            job.append(
                separator,
                0.0,
                TextFormat::simple(font.clone(), colors.separator),
            );
        }
        let is_network_bit = index * bits_per_digit < prefix_len;
        let color = if is_network_bit {
            colors.network
        } else {
            colors.host
        };
        job.append(
            &digit.to_string(),
            0.0,
            TextFormat::simple(font.clone(), color),
        );
    }
    show_custom_row(ui, label, |ui, label_id| {
        ui.add(egui::Label::new(job).wrap()).labelled_by(label_id);
    });
}
