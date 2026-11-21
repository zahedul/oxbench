//! Merges a list of networks, addresses and ranges into the fewest networks covering them.

use eframe::egui;
use oxbench_core::cidr::{MergeResult, merge_networks};

use crate::cached::Cached;
use crate::widgets::code_editor::{show_code_editor, show_code_view};
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};

const SAMPLE_LIST: &str = "192.168.0.0/24
192.168.1.0/24
192.168.2.0/25
192.168.2.128/25
10.0.0.0/30
10.0.0.4 - 10.0.0.15
";
const INPUT_HINT: &str =
    "One or more per line, separated by commas:\n10.0.0.0/24\n10.0.1.7\n10.0.2.10 - 10.0.2.40";
const MIN_BODY_HEIGHT: f32 = 200.0;
/// More problems than this are summarised, so the merged list stays in view.
const MAX_LISTED_PROBLEMS: usize = 5;

struct MergeListing {
    result: MergeResult,
    merged_text: String,
}

#[derive(Default)]
pub struct MergeView {
    pub input: String,
    listing: Option<Cached<String, MergeListing>>,
}

impl MergeView {
    pub fn load_sample(&mut self) {
        SAMPLE_LIST.clone_into(&mut self.input);
    }

    #[cfg(test)]
    pub fn merged_text(&self) -> Option<&str> {
        Some(&self.listing.as_ref()?.value.merged_text)
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
            ui.columns(2, |columns| {
                show_pane_header(&mut columns[0], "Networks", |_| {});
                show_code_editor(
                    &mut columns[0],
                    "cidr_merge_input",
                    &mut self.input,
                    INPUT_HINT,
                );
                self.show_result(&mut columns[1]);
            });
        });
    }

    fn show_result(&mut self, ui: &mut egui::Ui) {
        let is_empty = self.input.trim().is_empty();
        let listing = Cached::refresh(&mut self.listing, self.input.clone(), |input| {
            build_listing(merge_networks(input))
        });
        show_pane_header(ui, "Merged", |ui| {
            show_copy_button(ui, "Copy merged", &listing.merged_text);
        });
        if is_empty {
            ui.weak("Paste networks, addresses or ranges on the left. Overlapping and neighbouring ones are combined.");
            return;
        }
        let result = &listing.result;
        if result.input_count > 0 {
            show_status(ui, StatusTone::Success, &describe_merge(result));
        }
        show_problems(ui, &result.problems);
        ui.add_space(6.0);
        show_code_view(ui, "cidr_merge_output", &listing.merged_text);
    }
}

fn build_listing(result: MergeResult) -> MergeListing {
    let merged_text = result
        .networks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    MergeListing {
        result,
        merged_text,
    }
}

fn describe_merge(result: &MergeResult) -> String {
    format!(
        "{} merged into {}.",
        count_noun(result.input_count, "entry", "entries"),
        count_noun(result.networks.len(), "network", "networks")
    )
}

fn show_problems(ui: &mut egui::Ui, problems: &[String]) {
    for problem in problems.iter().take(MAX_LISTED_PROBLEMS) {
        show_status(ui, StatusTone::Error, problem);
    }
    let hidden = problems.len().saturating_sub(MAX_LISTED_PROBLEMS);
    if hidden > 0 {
        ui.weak(format!(
            "{} more can't be read.",
            count_noun(hidden, "line", "lines")
        ));
    }
}

fn count_noun(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}
