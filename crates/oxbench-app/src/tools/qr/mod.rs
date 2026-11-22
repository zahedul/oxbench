//! QR Generator: text or a Wi-Fi network, previewed live and exported as PNG or SVG.

mod preview;

#[cfg(test)]
mod tests;

use eframe::egui::{self, Color32};
use oxbench_core::CoreResult;
use oxbench_core::qr::{
    DEFAULT_QUIET_ZONE, ErrorCorrection, QrMatrix, QrStyle, Rgb, WifiNetwork, WifiSecurity,
    build_wifi_payload, encode_qr, render_png, render_svg,
};

use crate::cached::Cached;
use crate::save_file::{Readers, SaveOutcome, save_with_dialog};
use crate::style::icons;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::form::{
    show_checkbox_row, show_choice_row, show_custom_row, show_form, show_slider_row,
};
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::input_row::show_secret_field;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::view_tabs::show_view_tabs;
use preview::{show_preview, to_color_image};

const MIN_BODY_HEIGHT: f32 = 200.0;
const MAX_QUIET_ZONE: usize = 10;
const MIN_EXPORT_SIZE: usize = 128;
const MAX_EXPORT_SIZE: usize = 2048;
const DEFAULT_EXPORT_SIZE: usize = 512;
/// Below this contrast ratio many phone cameras struggle to read a code.
const MIN_CONTRAST_RATIO: f32 = 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ContentKind {
    #[default]
    Text,
    Wifi,
}

pub struct QrGenerator {
    content_kind: ContentKind,
    text: String,
    wifi: WifiNetwork,
    is_password_revealed: bool,
    correction: ErrorCorrection,
    quiet_zone: usize,
    dark: Color32,
    light: Color32,
    export_size: usize,
    matrix: Option<Cached<(String, ErrorCorrection), CoreResult<QrMatrix>>>,
    save_outcome: Option<SaveOutcome>,
}

impl Default for QrGenerator {
    fn default() -> Self {
        Self {
            content_kind: ContentKind::default(),
            text: String::new(),
            wifi: WifiNetwork::default(),
            is_password_revealed: false,
            correction: ErrorCorrection::default(),
            quiet_zone: DEFAULT_QUIET_ZONE,
            dark: Color32::BLACK,
            light: Color32::WHITE,
            export_size: DEFAULT_EXPORT_SIZE,
            matrix: None,
            save_outcome: None,
        }
    }
}

impl QrGenerator {
    /// What the code encodes; empty when there is nothing to show yet.
    fn payload(&self) -> String {
        match self.content_kind {
            ContentKind::Text => self.text.clone(),
            ContentKind::Wifi if self.wifi.ssid.is_empty() => String::new(),
            ContentKind::Wifi => build_wifi_payload(&self.wifi),
        }
    }

    fn style(&self) -> QrStyle {
        QrStyle {
            quiet_zone: self.quiet_zone,
            dark: to_rgb(self.dark),
            light: to_rgb(self.light),
        }
    }

    fn refresh_matrix(&mut self) {
        let key = (self.payload(), self.correction);
        Cached::refresh(&mut self.matrix, key, |(payload, correction)| {
            encode_qr(payload, *correction)
        });
    }

    fn show_content_card(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Content", |ui| {
            let kinds = [
                (ContentKind::Text, "Text or link"),
                (ContentKind::Wifi, "Wi-Fi"),
            ];
            show_view_tabs(ui, &mut self.content_kind, &kinds);
            ui.add_space(8.0);
            match self.content_kind {
                ContentKind::Text => {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .id_salt("qr_text")
                            .hint_text("https://example.com")
                            .desired_rows(4)
                            .desired_width(f32::INFINITY),
                    );
                }
                ContentKind::Wifi => self.show_wifi_form(ui),
            }
            ui.add_space(6.0);
            let bytes = self.payload().len();
            if bytes > 0 {
                ui.weak(format!("{bytes} bytes to encode."));
            }
        });
    }

    fn show_wifi_form(&mut self, ui: &mut egui::Ui) {
        show_form(ui, "qr_wifi", |ui| {
            show_custom_row(ui, "Network", |ui, label_id| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.wifi.ssid)
                        .id_salt("qr_wifi_ssid")
                        .hint_text("Network name (SSID)")
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label_id);
            });
            let securities = WifiSecurity::ALL.map(|security| (security, security.label()));
            show_choice_row(ui, "Security", &mut self.wifi.security, &securities);
            if self.wifi.security != WifiSecurity::Open {
                show_custom_row(ui, "Password", |ui, label_id| {
                    show_secret_field(
                        ui,
                        "qr_wifi_password",
                        &mut self.wifi.password,
                        "Network password",
                        &mut self.is_password_revealed,
                    )
                    .labelled_by(label_id);
                });
            }
            show_checkbox_row(
                ui,
                "Visibility",
                &mut [(&mut self.wifi.is_hidden, "Hidden network")],
            );
        });
    }

    fn show_style_card(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Style", |ui| {
            show_form(ui, "qr_style", |ui| {
                let levels = ErrorCorrection::ALL.map(|level| (level, level.label()));
                show_choice_row(ui, "Error correction", &mut self.correction, &levels);
                show_slider_row(ui, "Margin", &mut self.quiet_zone, 0..=MAX_QUIET_ZONE);
                show_custom_row(ui, "Colours", |ui, _| {
                    ui.horizontal(|ui| {
                        ui.color_edit_button_srgba(&mut self.dark);
                        ui.label("Code");
                        ui.add_space(12.0);
                        ui.color_edit_button_srgba(&mut self.light);
                        ui.label("Background");
                    });
                });
                show_slider_row(
                    ui,
                    "PNG size",
                    &mut self.export_size,
                    MIN_EXPORT_SIZE..=MAX_EXPORT_SIZE,
                );
            });
            ui.add_space(6.0);
            ui.weak("Higher error correction survives smudges and logos, but makes a denser code.");
            if contrast_ratio(self.dark, self.light) < MIN_CONTRAST_RATIO {
                show_status(
                    ui,
                    StatusTone::Warning,
                    "Low contrast: scanners need a dark code on a light background.",
                );
            }
        });
    }

    fn show_preview_pane(&mut self, ui: &mut egui::Ui) {
        let style = self.style();
        let export_size = self.export_size;
        let Some(cached) = &self.matrix else {
            return;
        };
        let matrix = match &cached.value {
            Ok(matrix) => matrix,
            Err(_) if cached.key.0.is_empty() => {
                show_pane_header(ui, "Preview", |_| {});
                ui.weak(match self.content_kind {
                    ContentKind::Text => "Type text or a link to make a QR code.",
                    ContentKind::Wifi => "Enter the network name to make a join code.",
                });
                return;
            }
            Err(error) => {
                show_pane_header(ui, "Preview", |_| {});
                show_status(ui, StatusTone::Error, &error.to_string());
                return;
            }
        };
        show_pane_header(ui, "Preview", |ui| {
            if ui
                .add(icon_label_button(icons::COPY, "Copy image"))
                .clicked()
            {
                ui.ctx()
                    .copy_image(to_color_image(matrix, style, export_size));
            }
        });
        show_preview(ui, matrix, style);
        ui.add_space(8.0);
        ui.weak(format!(
            "Version {} · {} × {} modules · {} error correction",
            matrix.version,
            matrix.width,
            matrix.width,
            self.correction.label()
        ));
        ui.add_space(10.0);
        let outcome = show_export_buttons(ui, matrix, style, export_size);
        if outcome.is_some() {
            self.save_outcome = outcome;
        }
        if let Some(outcome) = &self.save_outcome {
            let tone = match outcome {
                SaveOutcome::Saved { .. } => StatusTone::Success,
                SaveOutcome::Failed { .. } => StatusTone::Error,
            };
            ui.add_space(6.0);
            show_status(ui, tone, &outcome.describe());
        }
    }
}

impl Tool for QrGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.refresh_matrix();
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("qr_options_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| {
                        self.show_content_card(ui);
                        ui.add_space(12.0);
                        self.show_style_card(ui);
                    });
                egui::ScrollArea::vertical()
                    .id_salt("qr_preview_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_preview_pane(ui));
            });
        });
    }
}

/// Save PNG, Save SVG and Copy SVG. Returns what a save did; `None` if nothing was saved.
fn show_export_buttons(
    ui: &mut egui::Ui,
    matrix: &QrMatrix,
    style: QrStyle,
    export_size: usize,
) -> Option<SaveOutcome> {
    let svg = render_svg(matrix, style, pixel_size(export_size));
    ui.horizontal(|ui| {
        let mut outcome = None;
        if ui
            .add(icon_label_button(icons::DOWNLOAD_SIMPLE, "Save PNG"))
            .on_hover_text(format!("About {export_size} × {export_size} pixels"))
            .clicked()
        {
            outcome = save_png(matrix, style, export_size);
        }
        if ui
            .add(icon_label_button(icons::DOWNLOAD_SIMPLE, "Save SVG"))
            .clicked()
        {
            outcome = save_with_dialog(
                "Save QR code",
                "qr-code.svg",
                Some(("SVG image", &["svg"])),
                svg.as_bytes(),
                Readers::Anyone,
            );
        }
        show_copy_button(ui, "Copy SVG", &svg);
        outcome
    })
    .inner
}

/// `None` when the save dialog was cancelled.
fn save_png(matrix: &QrMatrix, style: QrStyle, export_size: usize) -> Option<SaveOutcome> {
    let bytes = match render_png(matrix, style, pixel_size(export_size)) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Some(SaveOutcome::Failed {
                message: error.to_string(),
            });
        }
    };
    save_with_dialog(
        "Save QR code",
        "qr-code.png",
        Some(("PNG image", &["png"])),
        &bytes,
        Readers::Anyone,
    )
}

fn pixel_size(export_size: usize) -> u32 {
    u32::try_from(export_size).unwrap_or(u32::MAX)
}

fn to_rgb(color: Color32) -> Rgb {
    [color.r(), color.g(), color.b()]
}

/// The WCAG contrast ratio, from 1 (none) to 21 (black on white).
fn contrast_ratio(first: Color32, second: Color32) -> f32 {
    let (first, second) = (relative_luminance(first), relative_luminance(second));
    let (lighter, darker) = if first > second {
        (first, second)
    } else {
        (second, first)
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Color32) -> f32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}
