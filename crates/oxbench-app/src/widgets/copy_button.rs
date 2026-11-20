use std::time::Duration;

use eframe::egui;

use crate::style::icons;
use crate::widgets::icon_button::{icon_label_button, show_icon_button};

const FEEDBACK_SECONDS: f64 = 1.5;

/// Copies `text` to the clipboard and briefly shows "Copied". Disabled when there is nothing to copy.
pub fn show_copy_button(ui: &mut egui::Ui, label: &str, text: &str) -> egui::Response {
    let feedback = CopyFeedback::read(ui, "copy_button", label);
    let button = if feedback.is_showing {
        icon_label_button(icons::CHECK_CIRCLE, "Copied")
    } else {
        icon_label_button(icons::COPY, label)
    };
    let response = ui.add_enabled(!text.is_empty(), button);
    feedback.handle_click(ui, &response, text);
    response
}

/// A compact copy button for rows of values. `label` is the tooltip and the accessible name.
pub fn show_copy_icon_button(ui: &mut egui::Ui, label: &str, text: &str) -> egui::Response {
    let feedback = CopyFeedback::read(ui, "copy_icon_button", label);
    let icon = if feedback.is_showing {
        icons::CHECK_CIRCLE
    } else {
        icons::COPY
    };
    let response = show_icon_button(ui, icon, label);
    feedback.handle_click(ui, &response, text);
    response
}

/// Remembers when a button last copied, so it can show a check mark for a moment.
struct CopyFeedback {
    id: egui::Id,
    now: f64,
    is_showing: bool,
}

impl CopyFeedback {
    fn read(ui: &egui::Ui, kind: &str, label: &str) -> Self {
        let id = ui.scope_id().with(kind).with(label);
        let now = ui.input(|input| input.time);
        let copied_at: Option<f64> = ui.data(|data| data.get_temp(id));
        Self {
            id,
            now,
            is_showing: copied_at.is_some_and(|time| now - time < FEEDBACK_SECONDS),
        }
    }

    fn handle_click(&self, ui: &egui::Ui, response: &egui::Response, text: &str) {
        if response.clicked() {
            ui.ctx().copy_text(text.to_owned());
            ui.data_mut(|data| data.insert_temp(self.id, self.now));
        }
        if self.is_showing || response.clicked() {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(FEEDBACK_SECONDS));
        }
    }
}
