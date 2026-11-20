use std::time::Duration;

use eframe::egui;

use crate::style::icons;
use crate::widgets::icon_button::icon_label_button;

const FEEDBACK_SECONDS: f64 = 1.5;

/// Copies `text` to the clipboard and briefly shows "Copied". Disabled when there is nothing to copy.
pub fn show_copy_button(ui: &mut egui::Ui, label: &str, text: &str) -> egui::Response {
    let feedback_id = ui.scope_id().with(("copy_button", label));
    let now = ui.input(|input| input.time);
    let copied_at: Option<f64> = ui.data(|data| data.get_temp(feedback_id));
    let is_showing_feedback = copied_at.is_some_and(|time| now - time < FEEDBACK_SECONDS);
    let button = if is_showing_feedback {
        icon_label_button(icons::CHECK_CIRCLE, "Copied")
    } else {
        icon_label_button(icons::COPY, label)
    };

    let response = ui.add_enabled(!text.is_empty(), button);
    if response.clicked() {
        ui.ctx().copy_text(text.to_owned());
        ui.data_mut(|data| data.insert_temp(feedback_id, now));
    }
    if is_showing_feedback || response.clicked() {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(FEEDBACK_SECONDS));
    }
    response
}
