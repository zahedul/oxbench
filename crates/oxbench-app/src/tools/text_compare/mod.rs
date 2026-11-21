//! Text Compare: the two texts on top, their differences below, side by side or unified.

mod diff_view;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use eframe::egui;
use oxbench_core::diff::{
    CompareOptions, Comparison, DEFAULT_CONTEXT_LINES, DiffStats, DisplayItem, InlineGranularity,
    UnifiedItem, arrange_unified, compare_texts, fold_unchanged, format_unified_diff,
    list_all_rows,
};

use crate::style::icons;
use crate::style::palette::Palette;
use crate::tool::Tool;
use crate::widgets::code_editor::show_code_editor;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::icon_button::{icon_label_button, show_icon_button};
use crate::widgets::pane_header::show_pane_header;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::tool_header::show_tool_header;
use crate::widgets::view_tabs::show_view_tabs;
use diff_view::{DiffGeometry, show_side_by_side, show_unified};

const SAMPLE_ORIGINAL: &str = r#"[server]
host = "localhost"
port = 8080
workers = 4

[database]
url = "postgres://localhost/app"
pool_size = 10
timeout = 30

[logging]
level = "info"
format = "text"
"#;

const SAMPLE_CHANGED: &str = r#"[server]
host = "0.0.0.0"
port = 8080
workers = 8

[database]
url = "postgres://db.internal/app"
pool_size = 10

[logging]
level = "debug"
format = "json"
file = "/var/log/app.log"
"#;

const INPUT_HEIGHT_SHARE: f32 = 0.34;
const MIN_INPUT_HEIGHT: f32 = 110.0;
const MAX_INPUT_HEIGHT: f32 = 320.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Layout {
    #[default]
    SideBySide,
    Unified,
}

impl Layout {
    const ALL: [Layout; 2] = [Layout::SideBySide, Layout::Unified];

    const fn label(self) -> &'static str {
        match self {
            Layout::SideBySide => "Side by side",
            Layout::Unified => "Unified",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Original,
    Changed,
}

impl Pane {
    const fn title(self) -> &'static str {
        match self {
            Pane::Original => "Original",
            Pane::Changed => "Changed",
        }
    }

    const fn id_salt(self) -> &'static str {
        match self {
            Pane::Original => "compare_original",
            Pane::Changed => "compare_changed",
        }
    }

    const fn hint(self) -> &'static str {
        match self {
            Pane::Original => "Paste the original text, or drop a file",
            Pane::Changed => "Paste the changed text, or drop a file",
        }
    }

    const fn open_label(self) -> &'static str {
        match self {
            Pane::Original => "Open original file",
            Pane::Changed => "Open changed file",
        }
    }

    const fn default_file_name(self) -> &'static str {
        match self {
            Pane::Original => "original",
            Pane::Changed => "changed",
        }
    }
}

#[derive(Default)]
struct InputText {
    text: String,
    /// The file it was loaded from, used to name the sides of a copied patch.
    file_name: Option<String>,
}

/// The inputs a result was computed from.
#[derive(Clone, PartialEq)]
struct DiffInputs {
    original: String,
    changed: String,
    options: CompareOptions,
    should_fold_unchanged: bool,
}

/// Everything the differences pane shows, computed once per input change.
struct DiffResult {
    inputs: DiffInputs,
    comparison: Comparison,
    items: Vec<DisplayItem>,
    unified: Vec<UnifiedItem>,
    patch: String,
    geometry: DiffGeometry,
}

pub struct TextCompare {
    original: InputText,
    changed: InputText,
    options: CompareOptions,
    layout: Layout,
    should_fold_unchanged: bool,
    result: Option<DiffResult>,
    file_error: Option<String>,
    /// Where each input sat last frame, so a dropped file lands on the pane under the pointer.
    pane_rects: [Option<egui::Rect>; 2],
}

impl Default for TextCompare {
    fn default() -> Self {
        Self {
            original: InputText::default(),
            changed: InputText::default(),
            options: CompareOptions::default(),
            layout: Layout::default(),
            should_fold_unchanged: true,
            result: None,
            file_error: None,
            pane_rects: [None; 2],
        }
    }
}

impl TextCompare {
    fn input(&self, pane: Pane) -> &InputText {
        match pane {
            Pane::Original => &self.original,
            Pane::Changed => &self.changed,
        }
    }

    fn input_mut(&mut self, pane: Pane) -> &mut InputText {
        match pane {
            Pane::Original => &mut self.original,
            Pane::Changed => &mut self.changed,
        }
    }

    fn is_result_fresh(&self) -> bool {
        self.result.as_ref().is_some_and(|result| {
            let inputs = &result.inputs;
            inputs.original == self.original.text
                && inputs.changed == self.changed.text
                && inputs.options == self.options
                && inputs.should_fold_unchanged == self.should_fold_unchanged
        })
    }

    fn refresh_result(&mut self) {
        if self.is_result_fresh() {
            return;
        }
        let inputs = DiffInputs {
            original: self.original.text.clone(),
            changed: self.changed.text.clone(),
            options: self.options,
            should_fold_unchanged: self.should_fold_unchanged,
        };
        let patch_names = [Pane::Original, Pane::Changed].map(|pane| {
            self.input(pane)
                .file_name
                .clone()
                .unwrap_or_else(|| pane.default_file_name().to_owned())
        });
        self.result = Some(compute_result(inputs, &patch_names));
    }

    fn load_file(&mut self, pane: Pane, path: &Path) {
        let file_name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.file_error = Some(format!("Couldn't open {file_name}: {error}."));
                return;
            }
        };
        let Ok(text) = String::from_utf8(bytes) else {
            self.file_error = Some(format!(
                "{file_name} isn't UTF-8 text, so it can't be compared."
            ));
            return;
        };
        self.file_error = None;
        *self.input_mut(pane) = InputText {
            text,
            file_name: Some(file_name),
        };
    }

    fn take_dropped_files(&mut self, ctx: &egui::Context) {
        let (paths, pointer): (Vec<PathBuf>, _) = ctx.input(|input| {
            let paths = input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect();
            (paths, input.pointer.latest_pos())
        });
        match paths.as_slice() {
            [] => {}
            [path] => self.load_file(self.choose_drop_target(pointer), path),
            [original, changed, ..] => {
                self.load_file(Pane::Original, original);
                self.load_file(Pane::Changed, changed);
            }
        }
    }

    /// The pane under the pointer; otherwise the first empty one.
    fn choose_drop_target(&self, pointer: Option<egui::Pos2>) -> Pane {
        let [original_rect, changed_rect] = self.pane_rects;
        let is_over = |rect: Option<egui::Rect>| {
            pointer.is_some_and(|pos| rect.is_some_and(|rect| rect.contains(pos)))
        };
        if is_over(changed_rect) {
            return Pane::Changed;
        }
        if is_over(original_rect) {
            return Pane::Original;
        }
        if !self.original.text.is_empty() && self.changed.text.is_empty() {
            return Pane::Changed;
        }
        Pane::Original
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        show_tool_header(
            ui,
            |ui| {
                let options = Layout::ALL.map(|layout| (layout, layout.label()));
                show_view_tabs(ui, &mut self.layout, &options);
            },
            |ui| {
                if ui.add(icon_label_button(icons::ERASER, "Clear")).clicked() {
                    self.original = InputText::default();
                    self.changed = InputText::default();
                    self.file_error = None;
                }
                if ui
                    .add(icon_label_button(icons::ARROWS_LEFT_RIGHT, "Swap"))
                    .on_hover_text("Swap the original and changed texts")
                    .clicked()
                {
                    std::mem::swap(&mut self.original, &mut self.changed);
                }
                if ui
                    .add(icon_label_button(icons::CLIPBOARD_TEXT, "Sample"))
                    .clicked()
                {
                    self.original = sample_input(SAMPLE_ORIGINAL);
                    self.changed = sample_input(SAMPLE_CHANGED);
                    self.file_error = None;
                }
            },
        );
    }

    fn show_inputs(&mut self, ui: &mut egui::Ui) {
        let height =
            (ui.available_height() * INPUT_HEIGHT_SHARE).clamp(MIN_INPUT_HEIGHT, MAX_INPUT_HEIGHT);
        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
            ui.columns(2, |columns| {
                self.pane_rects[0] = Some(self.show_input(&mut columns[0], Pane::Original));
                self.pane_rects[1] = Some(self.show_input(&mut columns[1], Pane::Changed));
            });
        });
        ui.add_space(8.0);
    }

    fn show_input(&mut self, ui: &mut egui::Ui, pane: Pane) -> egui::Rect {
        let palette = Palette::of(ui);
        let mut is_open_clicked = false;
        let file_name = self.input(pane).file_name.as_deref();
        show_pane_header(ui, pane.title(), |ui| {
            is_open_clicked = show_icon_button(ui, icons::FOLDER_OPEN, pane.open_label()).clicked();
            if let Some(name) = file_name {
                ui.label(
                    egui::RichText::new(name)
                        .size(12.0)
                        .color(palette.text_weak),
                );
            }
        });
        let input = self.input_mut(pane);
        let response = show_code_editor(ui, pane.id_salt(), &mut input.text, pane.hint());
        if is_open_clicked
            && let Some(path) = rfd::FileDialog::new()
                .set_title(pane.open_label())
                .pick_file()
        {
            self.load_file(pane, &path);
        }
        response.rect
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.should_ignore_case, "Ignore case");
            ui.checkbox(
                &mut self.options.should_ignore_whitespace,
                "Ignore whitespace",
            )
            .on_hover_text("Treat runs of spaces and tabs as one, and ignore them at line ends");
            ui.checkbox(&mut self.should_fold_unchanged, "Only changed lines")
                .on_hover_text(format!(
                    "Fold unchanged lines, keeping {DEFAULT_CONTEXT_LINES} around each change"
                ));
            ui.separator();
            let label = ui.label("Highlight");
            let combo = egui::ComboBox::from_id_salt("compare_granularity")
                .selected_text(self.options.inline_granularity.label())
                .show_ui(ui, |ui| {
                    for granularity in InlineGranularity::ALL {
                        ui.selectable_value(
                            &mut self.options.inline_granularity,
                            granularity,
                            granularity.label(),
                        );
                    }
                });
            // Names the dropdown for screen readers, since its visible label sits beside it.
            let _ = combo.response.labelled_by(label.id);
        });
        ui.add_space(8.0);
    }

    fn show_differences(&mut self, ui: &mut egui::Ui) {
        let Some(result) = &self.result else {
            return;
        };
        let stats = result.comparison.stats;
        let has_input = !self.original.text.is_empty() || !self.changed.text.is_empty();
        show_pane_header(ui, "Differences", |ui| {
            if !has_input {
                return;
            }
            show_copy_button(ui, "Copy patch", &result.patch)
                .on_hover_text("Copy the differences as a unified diff");
            show_summary(ui, stats);
        });
        if let Some(error) = &self.file_error {
            show_status(ui, StatusTone::Error, error);
            ui.add_space(4.0);
        }
        if !has_input {
            ui.weak("Paste or open two texts to see what changed.");
            return;
        }
        if !stats.has_differences() {
            show_status(
                ui,
                StatusTone::Success,
                &describe_no_differences(self.options),
            );
            if self.should_fold_unchanged {
                return;
            }
            ui.add_space(4.0);
        }
        let was_fold_clicked = match self.layout {
            Layout::SideBySide => {
                show_side_by_side(ui, &result.comparison.rows, &result.items, result.geometry)
            }
            Layout::Unified => show_unified(
                ui,
                &result.comparison.rows,
                &result.unified,
                result.geometry,
            ),
        };
        if was_fold_clicked {
            self.should_fold_unchanged = false;
        }
    }
}

impl Tool for TextCompare {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.take_dropped_files(ui.ctx());
        self.show_header(ui);
        self.show_inputs(ui);
        self.show_toolbar(ui);
        self.refresh_result();
        self.show_differences(ui);
    }
}

fn sample_input(text: &str) -> InputText {
    InputText {
        text: text.to_owned(),
        file_name: None,
    }
}

fn compute_result(inputs: DiffInputs, patch_names: &[String; 2]) -> DiffResult {
    let comparison = compare_texts(&inputs.original, &inputs.changed, inputs.options);
    let rows = &comparison.rows;
    let items = if inputs.should_fold_unchanged {
        fold_unchanged(rows, DEFAULT_CONTEXT_LINES)
    } else {
        list_all_rows(rows)
    };
    let unified = arrange_unified(rows, &items);
    let [left_name, right_name] = patch_names;
    let patch = format_unified_diff(rows, left_name, right_name, DEFAULT_CONTEXT_LINES);
    let geometry = DiffGeometry::measure(rows);
    DiffResult {
        inputs,
        comparison,
        items,
        unified,
        patch,
        geometry,
    }
}

fn show_summary(ui: &mut egui::Ui, stats: DiffStats) {
    let palette = Palette::of(ui);
    // Laid out right to left, so the last-read count comes first.
    let counts = [
        (stats.changed, "changed", palette.warning),
        (stats.removed, "removed", palette.error),
        (stats.added, "added", palette.success),
    ];
    for (count, word, color) in counts {
        let color = if count == 0 { palette.text_weak } else { color };
        ui.label(
            egui::RichText::new(format!("{count} {word}"))
                .size(12.0)
                .color(color),
        );
    }
}

fn describe_no_differences(options: CompareOptions) -> String {
    let ignored = match (options.should_ignore_case, options.should_ignore_whitespace) {
        (false, false) => return "The texts are identical.".to_owned(),
        (true, false) => "case",
        (false, true) => "whitespace",
        (true, true) => "case and whitespace",
    };
    format!("No differences, ignoring {ignored}.")
}
