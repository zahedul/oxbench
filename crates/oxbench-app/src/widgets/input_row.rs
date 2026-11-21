use eframe::egui;

use crate::style::icons;
use crate::widgets::icon_button::show_icon_button;

/// A single-line input that fills the row after `add_trailing` places its controls on the
/// right. Controls go right to left, so add the rightmost first.
pub fn show_input_row<R>(
    ui: &mut egui::Ui,
    input: egui::TextEdit<'_>,
    add_trailing: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    // A fixed height: a right-to-left layout would otherwise take all the height it is given.
    let row_size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        row_size,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let trailing = add_trailing(ui);
            let response = ui.add(input.desired_width(ui.available_width()));
            (response, trailing)
        },
    )
    .inner
}

/// A password input with a button that shows or hides what was typed.
pub fn show_secret_field(
    ui: &mut egui::Ui,
    id_salt: &str,
    text: &mut String,
    hint: &str,
    is_revealed: &mut bool,
) -> egui::Response {
    let input = egui::TextEdit::singleline(text)
        .id_salt(id_salt)
        .font(egui::TextStyle::Monospace)
        .hint_text(hint)
        .password(!*is_revealed);
    let (response, is_toggled) = show_input_row(ui, input, |ui| {
        let (icon, label) = if *is_revealed {
            (icons::EYE_SLASH, "Hide password")
        } else {
            (icons::EYE, "Show password")
        };
        show_icon_button(ui, icon, label).clicked()
    });
    if is_toggled {
        *is_revealed = !*is_revealed;
    }
    response
}
