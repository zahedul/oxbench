//! Hash Generator: text or a file on the left; every algorithm's digest on the right, with a
//! box to compare against a published checksum.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::hash::{
    DigestEncoding, HashAlgorithm, HashOutput, KeyEncoding, decode_key, encode_digest, hash_all,
    hash_file, hmac_all, matches_digest,
};

use crate::cached::Cached;
use crate::style::icons;
use crate::tasks::JobSlot;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::input_row::show_input_row;
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::tool_header::show_tool_header;
use crate::widgets::value_list::{ValueRow, show_value_list};
use crate::widgets::view_tabs::show_view_tabs;

const MIN_BODY_HEIGHT: f32 = 160.0;
const TEXT_EDITOR_ROWS: usize = 12;
const PROGRESS_REFRESH: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Source {
    #[default]
    Text,
    File,
}

/// The text, and the HMAC key with its encoding when HMAC is on.
type TextKey = (String, Option<(String, KeyEncoding)>);
type DigestResult = CoreResult<Vec<HashOutput>>;

struct FileHash {
    path: PathBuf,
    size: Option<u64>,
    bytes_read: Arc<AtomicU64>,
    job: JobSlot<DigestResult>,
}

#[derive(Default)]
pub struct HashGenerator {
    source: Source,
    text_input: String,
    is_hmac: bool,
    hmac_key: String,
    key_encoding: KeyEncoding,
    encoding: DigestEncoding,
    expected_input: String,
    text_digests: Option<Cached<TextKey, DigestResult>>,
    file: Option<FileHash>,
}

impl HashGenerator {
    fn refresh_text_digests(&mut self) {
        let hmac = self
            .is_hmac
            .then(|| (self.hmac_key.clone(), self.key_encoding));
        let key = (self.text_input.clone(), hmac);
        Cached::refresh(&mut self.text_digests, key, |(text, hmac)| match hmac {
            Some((key_text, encoding)) => {
                decode_key(key_text, *encoding).map(|key| hmac_all(&key, text.as_bytes()))
            }
            None => Ok(hash_all(text.as_bytes())),
        });
    }

    fn start_file_hash(&mut self, ctx: &egui::Context, path: PathBuf) {
        let size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());
        let bytes_read = Arc::new(AtomicU64::new(0));
        let mut job = JobSlot::default();
        let worker_path = path.clone();
        let worker_progress = Arc::clone(&bytes_read);
        job.start(ctx, move || {
            hash_file(&worker_path, |read| {
                worker_progress.store(read, Ordering::Relaxed);
            })
        });
        self.source = Source::File;
        self.file = Some(FileHash {
            path,
            size,
            bytes_read,
            job,
        });
    }

    fn take_dropped_file(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .first()
                .map(|file| file.path().to_path_buf())
        });
        if let Some(path) = dropped {
            self.start_file_hash(ctx, path);
        }
    }

    fn current_digests(&self) -> Option<&DigestResult> {
        match self.source {
            Source::Text => self.text_digests.as_ref().map(|cached| &cached.value),
            Source::File => self.file.as_ref()?.job.finished(),
        }
    }

    fn clear(&mut self) {
        self.text_input.clear();
        self.hmac_key.clear();
        self.expected_input.clear();
        self.file = None;
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        let mut is_clear_clicked = false;
        show_tool_header(
            ui,
            |ui| {
                show_view_tabs(
                    ui,
                    &mut self.source,
                    &[(Source::Text, "Text"), (Source::File, "File")],
                );
            },
            |ui| {
                is_clear_clicked = ui.add(icon_label_button(icons::ERASER, "Clear")).clicked();
            },
        );
        if is_clear_clicked {
            self.clear();
        }
    }

    fn show_input_pane(&mut self, ui: &mut egui::Ui) {
        match self.source {
            Source::Text => self.show_text_input(ui),
            Source::File => self.show_file_input(ui),
        }
    }

    fn show_text_input(&mut self, ui: &mut egui::Ui) {
        let byte_count = self.text_input.len();
        show_pane_header(ui, "Text", |ui| {
            ui.weak(format!("{} · UTF-8", describe_byte_count(byte_count)));
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.text_input)
                .id_salt("hash_text")
                .code_editor()
                .hint_text("Type or paste text to hash")
                .desired_width(f32::INFINITY)
                .desired_rows(TEXT_EDITOR_ROWS),
        );
        ui.add_space(12.0);
        show_card(ui, "HMAC", |ui| {
            ui.checkbox(&mut self.is_hmac, "Sign with a secret key (HMAC)");
            if !self.is_hmac {
                return;
            }
            ui.add_space(4.0);
            let key_input = egui::TextEdit::singleline(&mut self.hmac_key)
                .id_salt("hash_hmac_key")
                .font(egui::TextStyle::Monospace)
                .hint_text("Secret key");
            show_input_row(ui, key_input, |ui| {
                let combo = egui::ComboBox::from_id_salt("hash_key_encoding")
                    .selected_text(self.key_encoding.label())
                    .show_ui(ui, |ui| {
                        for encoding in KeyEncoding::ALL {
                            ui.selectable_value(&mut self.key_encoding, encoding, encoding.label());
                        }
                    });
                // Added after the dropdown because the row runs right to left.
                let label = ui.weak("Format");
                let _ = combo.response.labelled_by(label.id);
            });
            if let Some(Err(error)) = self.current_digests() {
                ui.add_space(4.0);
                show_status(ui, StatusTone::Error, &error.to_string());
            }
        });
    }

    fn show_file_input(&mut self, ui: &mut egui::Ui) {
        show_pane_header(ui, "File", |_| {});
        let is_hovering_files = ui.ctx().input(|input| !input.raw.hovered_files.is_empty());
        show_card(ui, "Hash a file", |ui| {
            if is_hovering_files {
                show_status(ui, StatusTone::Neutral, "Drop to hash this file.");
            } else {
                ui.weak("Drop a file anywhere on this window, or choose one.");
            }
            ui.add_space(6.0);
            let is_open_clicked = ui
                .add(icon_label_button(icons::FOLDER_OPEN, "Open file…"))
                .clicked();
            if is_open_clicked
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Choose a file to hash")
                    .pick_file()
            {
                self.start_file_hash(ui.ctx(), path);
            }
            let Some(file) = &self.file else {
                return;
            };
            ui.add_space(10.0);
            show_file_details(ui, file);
        });
    }

    fn show_digest_pane(&mut self, ui: &mut egui::Ui) {
        let encoding = &mut self.encoding;
        show_pane_header(ui, "Digests", |ui| {
            let combo = egui::ComboBox::from_id_salt("hash_output_encoding")
                .selected_text(encoding.label())
                .show_ui(ui, |ui| {
                    for choice in DigestEncoding::ALL {
                        ui.selectable_value(encoding, choice, choice.label());
                    }
                });
            // Added after the dropdown because the header's actions run right to left.
            let label = ui.weak("Output");
            let _ = combo.response.labelled_by(label.id);
        });
        let Some(Ok(digests)) = self.current_digests() else {
            ui.weak(self.describe_missing_digests());
            return;
        };
        let digests = digests.clone();
        ui.add(
            egui::TextEdit::singleline(&mut self.expected_input)
                .id_salt("hash_expected")
                .font(egui::TextStyle::Monospace)
                .hint_text("Paste a checksum to compare")
                .desired_width(f32::INFINITY),
        );
        let matched = find_match(&self.expected_input, &digests);
        if !self.expected_input.trim().is_empty() {
            ui.add_space(4.0);
            match matched {
                Some(algorithm) => show_status(
                    ui,
                    StatusTone::Success,
                    &format!("Matches {}.", self.describe_algorithm(algorithm)),
                ),
                None => show_status(ui, StatusTone::Error, "Matches none of these digests."),
            }
        }
        ui.add_space(8.0);
        let rows: Vec<ValueRow> = digests
            .iter()
            .map(|digest| {
                let mut label = self.describe_algorithm(digest.algorithm);
                if matched == Some(digest.algorithm) {
                    label.push_str(" · match");
                }
                ValueRow::new(label, encode_digest(&digest.bytes, self.encoding))
            })
            .collect();
        show_value_list(ui, &rows);
        ui.add_space(8.0);
        ui.weak("MD5 and SHA-1 are fine for checksums, but not for passwords or signatures.");
    }

    fn describe_missing_digests(&self) -> &'static str {
        if self.source == Source::Text {
            return "Fix the key to see the digests.";
        }
        match self.file.as_ref().map(|file| &file.job) {
            Some(JobSlot::Running(_)) => "Hashing the file…",
            Some(JobSlot::Finished(Err(_)) | JobSlot::Lost) => "The file could not be hashed.",
            Some(JobSlot::Idle | JobSlot::Finished(Ok(_))) | None => {
                "Choose a file to see its digests."
            }
        }
    }

    fn describe_algorithm(&self, algorithm: HashAlgorithm) -> String {
        let is_hmac = self.source == Source::Text && self.is_hmac;
        if is_hmac {
            return format!("HMAC-{}", algorithm.label());
        }
        algorithm.label().to_owned()
    }
}

impl Tool for HashGenerator {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.take_dropped_file(ui.ctx());
        if let Some(file) = &mut self.file {
            file.job.poll();
            if file.job.is_running() {
                ui.ctx().request_repaint_after(PROGRESS_REFRESH);
            }
        }
        if self.source == Source::Text {
            self.refresh_text_digests();
        }
        self.show_header(ui);
        let body_height = ui.available_height().max(MIN_BODY_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("hash_input_pane")
                    .auto_shrink(false)
                    .show(&mut columns[0], |ui| self.show_input_pane(ui));
                egui::ScrollArea::vertical()
                    .id_salt("hash_digest_pane")
                    .auto_shrink(false)
                    .show(&mut columns[1], |ui| self.show_digest_pane(ui));
            });
        });
    }
}

fn show_file_details(ui: &mut egui::Ui, file: &FileHash) {
    let size_text = file
        .size
        .map_or_else(|| "Unknown".to_owned(), describe_byte_count);
    let rows = [
        ValueRow::new("Name", describe_file_name(&file.path)),
        ValueRow::new("Size", size_text),
    ];
    show_value_list(ui, &rows);
    ui.add_space(6.0);
    match &file.job {
        JobSlot::Running(_) => {
            let read = file.bytes_read.load(Ordering::Relaxed);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(describe_progress(read, file.size));
            });
        }
        JobSlot::Finished(Ok(_)) => show_status(ui, StatusTone::Success, "Hashed."),
        JobSlot::Finished(Err(error)) => show_status(ui, StatusTone::Error, &error.to_string()),
        JobSlot::Lost => show_status(ui, StatusTone::Error, "Hashing stopped unexpectedly."),
        JobSlot::Idle => {}
    }
}

fn find_match(expected: &str, digests: &[HashOutput]) -> Option<HashAlgorithm> {
    digests
        .iter()
        .find(|digest| matches_digest(expected, &digest.bytes))
        .map(|digest| digest.algorithm)
}

fn describe_file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "shown to one decimal; precision loss starts beyond 2^53 bytes"
)]
fn describe_byte_count(bytes: impl TryInto<u64>) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let bytes: u64 = bytes.try_into().unwrap_or(u64::MAX);
    if bytes < 1024 {
        let noun = if bytes == 1 { "byte" } else { "bytes" };
        return format!("{bytes} {noun}");
    }
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for candidate in UNITS {
        value /= 1024.0;
        unit = candidate;
        if value < 1024.0 {
            break;
        }
    }
    format!("{value:.1} {unit}")
}

fn describe_progress(read: u64, size: Option<u64>) -> String {
    match size {
        Some(total) if total > 0 => {
            let percent = read.saturating_mul(100) / total;
            format!("Hashing… {percent}%")
        }
        _ => format!("Hashing… {}", describe_byte_count(read)),
    }
}
