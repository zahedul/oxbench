use eframe::egui;

/// An editable monospace text area that fills the space it is given.
pub fn show_code_editor(
    ui: &mut egui::Ui,
    id_salt: &str,
    text: &mut String,
    hint: &str,
) -> egui::Response {
    egui::ScrollArea::both()
        .id_salt(id_salt)
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(text)
                    .id_salt(id_salt)
                    .code_editor()
                    .hint_text(hint)
                    .desired_width(f32::INFINITY)
                    .min_size(ui.available_size()),
            )
        })
        .inner
}

/// Read-only monospace text sized to its content, for stacking several blocks in one scroll area.
pub fn show_code_block(ui: &mut egui::Ui, id_salt: &str, text: &str) {
    let mut read_only = text;
    ui.add(
        egui::TextEdit::multiline(&mut read_only)
            .id_salt(id_salt)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(1),
    );
}

/// Read-only monospace text that can still be selected and copied.
pub fn show_code_view(ui: &mut egui::Ui, id_salt: &str, text: &str) {
    let mut read_only = text;
    egui::ScrollArea::both()
        .id_salt(id_salt)
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut read_only)
                    .id_salt(id_salt)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .min_size(ui.available_size()),
            );
        });
}
