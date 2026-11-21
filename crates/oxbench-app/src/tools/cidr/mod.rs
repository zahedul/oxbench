//! IP / CIDR Calculator: what a network covers, splitting it into subnets, and merging lists.

mod bits;
mod merge_view;
mod split_view;

#[cfg(test)]
mod tests;

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::cidr::{
    Interface, NetworkDetails, check_containment, describe_network, parse_interface,
};

use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::form::{show_custom_row, show_form};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::tool_header::show_tool_header;
use crate::widgets::value_list::{ValueRow, show_value_list};
use crate::widgets::view_tabs::show_view_tabs;
use bits::show_bits;
use merge_view::MergeView;
use split_view::SplitView;

const SAMPLE_NETWORK: &str = "192.168.1.10/24";
const NETWORK_HINT: &str = "192.168.1.10/24, 10.0.0.0 255.0.0.0 or 2001:db8::/48";
const MIN_BODY_HEIGHT: f32 = 200.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CidrView {
    #[default]
    Subnet,
    Split,
    Merge,
}

impl CidrView {
    const ALL: [CidrView; 3] = [CidrView::Subnet, CidrView::Split, CidrView::Merge];

    const fn label(self) -> &'static str {
        match self {
            CidrView::Subnet => "Subnet",
            CidrView::Split => "Split",
            CidrView::Merge => "Merge",
        }
    }
}

#[derive(Default)]
pub struct CidrCalculator {
    view: CidrView,
    network_input: String,
    check_input: String,
    split: SplitView,
    merge: MergeView,
}

impl CidrCalculator {
    /// `None` while the input is blank.
    fn parsed_interface(&self) -> Option<CoreResult<Interface>> {
        if self.network_input.trim().is_empty() {
            return None;
        }
        Some(parse_interface(&self.network_input))
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        let mut is_clear_clicked = false;
        let mut is_sample_clicked = false;
        show_tool_header(
            ui,
            |ui| {
                let options = CidrView::ALL.map(|view| (view, view.label()));
                show_view_tabs(ui, &mut self.view, &options);
            },
            |ui| {
                is_clear_clicked = ui.add(icon_label_button(icons::ERASER, "Clear")).clicked();
                is_sample_clicked = ui
                    .add(icon_label_button(icons::CLIPBOARD_TEXT, "Sample"))
                    .clicked();
            },
        );
        if is_clear_clicked {
            self.clear_current_view();
        }
        if is_sample_clicked {
            self.load_sample();
        }
    }

    fn clear_current_view(&mut self) {
        if self.view == CidrView::Merge {
            self.merge.input.clear();
            return;
        }
        self.network_input.clear();
        self.check_input.clear();
    }

    fn load_sample(&mut self) {
        if self.view == CidrView::Merge {
            self.merge.load_sample();
            return;
        }
        SAMPLE_NETWORK.clone_into(&mut self.network_input);
    }

    fn show_network_input(&mut self, ui: &mut egui::Ui) {
        show_form(ui, "cidr_network_form", |ui| {
            show_custom_row(ui, "Network", |ui, label_id| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.network_input)
                        .id_salt("cidr_network")
                        .font(egui::TextStyle::Monospace)
                        .hint_text(NETWORK_HINT)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label_id);
            });
        });
        ui.add_space(10.0);
    }

    fn show_subnet_view(&mut self, ui: &mut egui::Ui, interface: Interface) {
        let details = describe_network(interface);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("cidr_details_pane")
                .auto_shrink(false)
                .show(&mut columns[0], |ui| {
                    show_card(ui, "Network", |ui| {
                        show_value_list(ui, &list_detail_rows(&details));
                    });
                });
            egui::ScrollArea::vertical()
                .id_salt("cidr_side_pane")
                .auto_shrink(false)
                .show(&mut columns[1], |ui| {
                    self.show_check_card(ui, &details);
                    ui.add_space(12.0);
                    show_card(ui, "Bits", |ui| show_bits(ui, interface));
                    ui.add_space(12.0);
                    show_card(ui, "Other formats", |ui| {
                        show_value_list(ui, &list_format_rows(&details));
                    });
                });
        });
    }

    fn show_check_card(&mut self, ui: &mut egui::Ui, details: &NetworkDetails) {
        let network = details.interface.network;
        show_card(ui, "Is it in this network?", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.check_input)
                    .id_salt("cidr_check")
                    .font(egui::TextStyle::Monospace)
                    .hint_text("An address or a network")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            if self.check_input.trim().is_empty() {
                ui.weak(format!("Checks whether it falls inside {network}."));
                return;
            }
            match check_containment(network, &self.check_input) {
                Ok(containment) => {
                    let checked = describe_checked(containment.checked);
                    let Some(offset) = containment.offset else {
                        show_status(
                            ui,
                            StatusTone::Warning,
                            &format!("{checked} isn't in {network}."),
                        );
                        return;
                    };
                    let text = format!(
                        "{checked} is in {network}, at address {offset} of {}.",
                        details.total_addresses
                    );
                    show_status(ui, StatusTone::Success, &text);
                }
                Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
            }
        });
    }
}

impl Tool for CidrCalculator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.show_header(ui);
        if self.view == CidrView::Merge {
            self.merge.show(ui);
            return;
        }
        self.show_network_input(ui);
        let interface = match self.parsed_interface() {
            None => {
                ui.weak(
                    "Enter an address with a prefix or netmask to see the network it belongs to.",
                );
                return;
            }
            Some(Err(error)) => {
                show_status(ui, StatusTone::Error, &error.to_string());
                return;
            }
            Some(Ok(interface)) => interface,
        };
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(
            egui::vec2(ui.available_width(), body_height),
            |ui| match self.view {
                CidrView::Subnet => self.show_subnet_view(ui, interface),
                CidrView::Split => self.split.show(ui, interface.network),
                CidrView::Merge => {}
            },
        );
    }
}

fn list_detail_rows(details: &NetworkDetails) -> Vec<ValueRow> {
    let interface = details.interface;
    let is_ipv4 = interface.address.is_ipv4();
    let mut rows = vec![
        ValueRow::new("Address", interface.address.to_string()),
        ValueRow::new("Network", interface.network.to_string()),
        ValueRow::new("Type", details.kind),
        ValueRow::new("Netmask", details.netmask.to_string()),
        ValueRow::new("Wildcard mask", details.wildcard.to_string()),
        ValueRow::new("First address", details.first_address.to_string()),
    ];
    let last_label = if is_ipv4 { "Broadcast" } else { "Last address" };
    rows.push(ValueRow::new(last_label, details.last_address.to_string()));
    if is_ipv4 {
        let range = format!("{} – {}", details.first_host, details.last_host);
        let copied = format!("{}-{}", details.first_host, details.last_host);
        rows.push(ValueRow::new("Host range", range).copying(&copied));
    }
    rows.push(ValueRow::new("Addresses", &details.total_addresses));
    if is_ipv4 {
        rows.push(ValueRow::new("Usable hosts", &details.usable_hosts));
    }
    rows.push(ValueRow::new("Reverse DNS", &details.reverse_zone));
    rows
}

fn list_format_rows(details: &NetworkDetails) -> Vec<ValueRow> {
    let mut rows = Vec::new();
    if let Some(expanded) = &details.expanded {
        rows.push(ValueRow::new("Expanded", expanded));
    }
    rows.push(ValueRow::new("Integer", &details.integer));
    rows.push(ValueRow::new("Hex", &details.hex));
    if let Some(class) = details.class {
        rows.push(
            ValueRow::new("Class", format!("{class} (historic)")).copying(&class.to_string()),
        );
    }
    rows
}

/// A bare address reads better without its single-address prefix.
fn describe_checked(checked: Interface) -> String {
    let network = checked.network;
    if network.prefix_len() == network.max_prefix_len() {
        return checked.address.to_string();
    }
    network.to_string()
}
