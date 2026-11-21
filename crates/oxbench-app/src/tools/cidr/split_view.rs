//! Splits the network into equal subnets with a longer prefix.

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::cidr::{
    Interface, IpNet, MAX_LISTED_SUBNETS, SplitPlan, describe_network, split_network,
};

use crate::cached::Cached;
use crate::style::palette::Palette;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{show_form, show_slider_row};
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};

/// A new network starts out split into four.
const DEFAULT_EXTRA_BITS: usize = 2;
const NUMBER_COLUMN_WIDTH: f32 = 56.0;
const SUBNET_COLUMN_WIDTH: f32 = 230.0;
const RANGE_COLUMN_WIDTH: f32 = 320.0;

/// A split with the text "Copy all" copies, built once per split.
struct SplitListing {
    plan: SplitPlan,
    copy_text: String,
}

#[derive(Default)]
pub struct SplitView {
    /// `None` until a prefix is chosen, so each network starts `DEFAULT_EXTRA_BITS` longer.
    new_prefix_len: Option<usize>,
    listing: Option<Cached<(IpNet, u8), CoreResult<SplitListing>>>,
}

impl SplitView {
    pub fn show(&mut self, ui: &mut egui::Ui, network: IpNet) {
        let current = usize::from(network.prefix_len());
        let max = usize::from(network.max_prefix_len());
        if current == max {
            ui.weak(format!(
                "A /{max} network holds a single address, so it can't be split."
            ));
            return;
        }
        let mut new_prefix_len = self
            .new_prefix_len
            .unwrap_or(current + DEFAULT_EXTRA_BITS)
            .clamp(current + 1, max);
        let mut is_changed = false;
        show_form(ui, "cidr_split_form", |ui| {
            is_changed =
                show_slider_row(ui, "Subnet prefix", &mut new_prefix_len, current + 1..=max);
        });
        if is_changed {
            self.new_prefix_len = Some(new_prefix_len);
        }
        ui.add_space(10.0);
        let prefix_len = u8::try_from(new_prefix_len).unwrap_or(network.max_prefix_len());
        let listing = Cached::refresh(
            &mut self.listing,
            (network, prefix_len),
            |&(network, prefix_len)| split_network(network, prefix_len).map(build_listing),
        );
        match listing {
            Ok(listing) => show_listing(ui, listing, network),
            Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
        }
    }
}

fn build_listing(plan: SplitPlan) -> SplitListing {
    let copy_text = plan
        .subnets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    SplitListing { plan, copy_text }
}

fn show_listing(ui: &mut egui::Ui, listing: &SplitListing, network: IpNet) {
    let plan = &listing.plan;
    let usable_each = plan
        .subnets
        .first()
        .filter(|_| network.addr().is_ipv4())
        .map_or_else(String::new, |first| {
            let usable = describe_network(subnet_interface(*first)).usable_hosts;
            format!(", {usable} usable hosts each")
        });
    let summary = format!(
        "{} subnets of {} addresses{usable_each}.",
        plan.subnet_count, plan.addresses_each
    );
    show_pane_header(ui, "Subnets", |ui| {
        let response = show_copy_button(ui, "Copy all", &listing.copy_text);
        if plan.is_truncated {
            response.on_hover_text(format!("Copies the first {MAX_LISTED_SUBNETS}"));
        }
    });
    show_status(ui, StatusTone::Neutral, &summary);
    if plan.is_truncated {
        ui.weak(format!("Showing the first {MAX_LISTED_SUBNETS}."));
    }
    ui.add_space(6.0);
    show_subnet_table(ui, &plan.subnets, network.addr().is_ipv4());
}

fn show_subnet_table(ui: &mut egui::Ui, subnets: &[IpNet], is_ipv4: bool) {
    let palette = Palette::of(ui);
    let range_title = if is_ipv4 {
        "Host range"
    } else {
        "Address range"
    };
    ui.horizontal(|ui| {
        show_cell(ui, NUMBER_COLUMN_WIDTH, "#", palette.text_weak);
        show_cell(ui, SUBNET_COLUMN_WIDTH, "Subnet", palette.text_weak);
        show_cell(ui, RANGE_COLUMN_WIDTH, range_title, palette.text_weak);
        if is_ipv4 {
            show_cell(ui, RANGE_COLUMN_WIDTH, "Broadcast", palette.text_weak);
        }
    });
    let row_height = ui.spacing().interact_size.y;
    egui::ScrollArea::both()
        .id_salt("cidr_subnets")
        .auto_shrink(false)
        .show_rows(ui, row_height, subnets.len(), |ui, rows| {
            for index in rows {
                let Some(subnet) = subnets.get(index) else {
                    continue;
                };
                let details = describe_network(subnet_interface(*subnet));
                let range = format!("{} – {}", details.first_host, details.last_host);
                ui.horizontal(|ui| {
                    show_cell(
                        ui,
                        NUMBER_COLUMN_WIDTH,
                        &(index + 1).to_string(),
                        palette.text_weak,
                    );
                    show_cell(
                        ui,
                        SUBNET_COLUMN_WIDTH,
                        &subnet.to_string(),
                        palette.text_strong,
                    );
                    show_cell(ui, RANGE_COLUMN_WIDTH, &range, palette.text);
                    if is_ipv4 {
                        show_cell(
                            ui,
                            RANGE_COLUMN_WIDTH,
                            &details.last_address.to_string(),
                            palette.text,
                        );
                    }
                });
            }
        });
}

fn subnet_interface(subnet: IpNet) -> Interface {
    Interface {
        address: subnet.network(),
        network: subnet,
    }
}

fn show_cell(ui: &mut egui::Ui, width: f32, text: &str, color: egui::Color32) {
    let height = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.label(egui::RichText::new(text).monospace().color(color));
        },
    );
}
