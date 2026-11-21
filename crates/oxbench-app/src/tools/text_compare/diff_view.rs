//! Paints compared lines row by row. Only the rows on screen are laid out, so long texts stay fast.

use std::ops::Range;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Align2, Color32, FontId, Role, Sense, WidgetInfo};
use oxbench_core::diff::{DiffLine, DiffRow, DisplayItem, RowKind, Side, UnifiedItem};

use crate::style::palette::Palette;

const ROW_PADDING_Y: f32 = 3.0;
const NUMBER_PADDING_X: f32 = 10.0;
const MARKER_WIDTH_CHARS: f32 = 2.0;
const TEXT_PADDING_RIGHT: f32 = 16.0;
const FOLD_TEXT_INSET: f32 = 12.0;
/// Matches egui's default tab width, so long lines with tabs are measured as they are drawn.
const TAB_WIDTH_CHARS: usize = 4;

/// Content sizes, measured once per comparison rather than every frame.
#[derive(Clone, Copy, Debug)]
pub struct DiffGeometry {
    widest_line_chars: usize,
    number_digits: usize,
}

impl DiffGeometry {
    pub fn measure(rows: &[DiffRow]) -> Self {
        let lines = rows
            .iter()
            .flat_map(|row| [row.left.as_ref(), row.right.as_ref()])
            .flatten();
        let mut widest_line_chars = 0;
        let mut largest_number = 0;
        for line in lines {
            widest_line_chars = widest_line_chars.max(count_display_chars(&line.text));
            largest_number = largest_number.max(line.number);
        }
        Self {
            widest_line_chars,
            number_digits: largest_number.to_string().len(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Plain,
    Removed,
    Added,
    /// The other side has a line here and this one doesn't.
    Missing,
}

/// Font measurements and column widths for this frame.
struct Columns {
    font: FontId,
    row_height: f32,
    number_width: f32,
    marker_width: f32,
    text_width: f32,
}

impl Columns {
    #[allow(
        clippy::cast_precision_loss,
        reason = "character counts and digit counts are far below f32's exact integer range"
    )]
    fn measure(ui: &egui::Ui, geometry: DiffGeometry) -> Self {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let (char_width, text_height) = ui
            .ctx()
            .fonts_mut(|fonts| (fonts.glyph_width(&font, '0'), fonts.row_height(&font)));
        Self {
            font,
            row_height: text_height + ROW_PADDING_Y * 2.0,
            number_width: geometry.number_digits as f32 * char_width + NUMBER_PADDING_X * 2.0,
            marker_width: MARKER_WIDTH_CHARS * char_width,
            text_width: geometry.widest_line_chars as f32 * char_width + TEXT_PADDING_RIGHT,
        }
    }

    fn line_width(&self, number_columns: usize) -> f32 {
        #[allow(
            clippy::cast_precision_loss,
            reason = "there are at most two number columns"
        )]
        let numbers = number_columns as f32 * self.number_width;
        numbers + self.marker_width + self.text_width
    }
}

/// Original on the left, changed on the right. Returns whether a fold was clicked.
pub fn show_side_by_side(
    ui: &mut egui::Ui,
    rows: &[DiffRow],
    items: &[DisplayItem],
    geometry: DiffGeometry,
) -> bool {
    let columns = Columns::measure(ui, geometry);
    let half_width = columns.line_width(1).max(ui.available_width() / 2.0);
    let palette = Palette::of(ui);
    let mut was_fold_clicked = false;
    show_virtual_rows(
        ui,
        "compare_side_by_side",
        &columns,
        items.len(),
        |ui, range| {
            for item in items.get(range).unwrap_or_default() {
                let row_size = egui::vec2(half_width * 2.0, columns.row_height);
                let row_index = match *item {
                    DisplayItem::Row(row_index) => row_index,
                    DisplayItem::Folded { count } => {
                        was_fold_clicked |= show_fold(ui, row_size, count);
                        continue;
                    }
                };
                let Some(row) = rows.get(row_index) else {
                    continue;
                };
                let (rect, response) = ui.allocate_exact_size(row_size, Sense::hover());
                response.widget_info(|| WidgetInfo::labeled(Role::Label, true, describe_row(row)));
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let (left_rect, right_rect) = rect.split_left_right_at_x(rect.center().x);
                let (left_tone, right_tone) = side_tones(row.kind);
                let left_numbers = [row.left.as_ref().map(|line| line.number)];
                let right_numbers = [row.right.as_ref().map(|line| line.number)];
                paint_line(
                    ui,
                    left_rect,
                    &left_numbers,
                    row.left.as_ref(),
                    left_tone,
                    &columns,
                );
                paint_line(
                    ui,
                    right_rect,
                    &right_numbers,
                    row.right.as_ref(),
                    right_tone,
                    &columns,
                );
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    egui::Stroke::new(1.0, palette.border),
                );
            }
        },
    );
    was_fold_clicked
}

/// One column, with removed lines above the lines that replaced them. Returns whether a fold
/// was clicked.
pub fn show_unified(
    ui: &mut egui::Ui,
    rows: &[DiffRow],
    items: &[UnifiedItem],
    geometry: DiffGeometry,
) -> bool {
    let columns = Columns::measure(ui, geometry);
    let width = columns.line_width(2).max(ui.available_width());
    let mut was_fold_clicked = false;
    show_virtual_rows(ui, "compare_unified", &columns, items.len(), |ui, range| {
        for item in items.get(range).unwrap_or_default() {
            let row_size = egui::vec2(width, columns.row_height);
            let (row_index, side) = match *item {
                UnifiedItem::Line { row, side } => (row, side),
                UnifiedItem::Folded { count } => {
                    was_fold_clicked |= show_fold(ui, row_size, count);
                    continue;
                }
            };
            let Some(row) = rows.get(row_index) else {
                continue;
            };
            let (rect, response) = ui.allocate_exact_size(row_size, Sense::hover());
            let (line, tone, numbers) = unified_line(row, side);
            response.widget_info(|| {
                WidgetInfo::labeled(Role::Label, true, describe_unified_line(tone, line))
            });
            if ui.is_rect_visible(rect) {
                paint_line(ui, rect, &numbers, line, tone, &columns);
            }
        }
    });
    was_fold_clicked
}

fn show_virtual_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    columns: &Columns,
    count: usize,
    add_rows: impl FnOnce(&mut egui::Ui, Range<usize>),
) {
    ui.scope(|ui| {
        // Rows sit flush, so tints read as continuous blocks.
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        egui::ScrollArea::both()
            .id_salt(id_salt)
            .auto_shrink(false)
            .show_rows(ui, columns.row_height, count, add_rows);
    });
}

const fn side_tones(kind: RowKind) -> (Tone, Tone) {
    match kind {
        RowKind::Equal => (Tone::Plain, Tone::Plain),
        RowKind::Changed => (Tone::Removed, Tone::Added),
        RowKind::Removed => (Tone::Removed, Tone::Missing),
        RowKind::Added => (Tone::Missing, Tone::Added),
    }
}

/// The line a unified view shows, its tone, and its old and new line numbers.
fn unified_line(row: &DiffRow, side: Side) -> (Option<&DiffLine>, Tone, [Option<usize>; 2]) {
    let left_number = row.left.as_ref().map(|line| line.number);
    let right_number = row.right.as_ref().map(|line| line.number);
    if row.kind == RowKind::Equal {
        return (row.left.as_ref(), Tone::Plain, [left_number, right_number]);
    }
    match side {
        Side::Left => (row.left.as_ref(), Tone::Removed, [left_number, None]),
        Side::Right => (row.right.as_ref(), Tone::Added, [None, right_number]),
    }
}

fn paint_line(
    ui: &egui::Ui,
    rect: egui::Rect,
    numbers: &[Option<usize>],
    line: Option<&DiffLine>,
    tone: Tone,
    columns: &Columns,
) {
    let palette = Palette::of(ui);
    let painter = ui.painter_at(rect);
    let (background, highlight, marker, marker_color) = match tone {
        Tone::Plain => (None, Color32::TRANSPARENT, "", palette.text_weak),
        Tone::Removed => (
            Some(palette.diff_removed),
            palette.diff_removed_strong,
            "-",
            palette.error,
        ),
        Tone::Added => (
            Some(palette.diff_added),
            palette.diff_added_strong,
            "+",
            palette.success,
        ),
        Tone::Missing => (
            Some(palette.panel),
            Color32::TRANSPARENT,
            "",
            palette.text_weak,
        ),
    };
    if let Some(background) = background {
        painter.rect_filled(rect, 0, background);
    }
    let center_y = rect.center().y;
    let mut x = rect.left();
    for number in numbers {
        x += columns.number_width;
        if let Some(number) = number {
            painter.text(
                egui::pos2(x - NUMBER_PADDING_X, center_y),
                Align2::RIGHT_CENTER,
                number.to_string(),
                columns.font.clone(),
                palette.text_weak,
            );
        }
    }
    painter.text(
        egui::pos2(x + columns.marker_width / 2.0, center_y),
        Align2::CENTER_CENTER,
        marker,
        columns.font.clone(),
        marker_color,
    );
    x += columns.marker_width;
    let Some(line) = line else {
        return;
    };
    let galley = painter.layout_job(build_line_job(line, &columns.font, palette.text, highlight));
    let top = center_y - galley.size().y / 2.0;
    painter.galley(egui::pos2(x, top), galley, palette.text);
}

fn build_line_job(line: &DiffLine, font: &FontId, color: Color32, highlight: Color32) -> LayoutJob {
    let plain = TextFormat::simple(font.clone(), color);
    let marked = TextFormat {
        background: highlight,
        ..plain.clone()
    };
    let mut job = LayoutJob::default();
    let mut cursor = 0;
    for range in &line.highlights {
        job.append(
            text_between(&line.text, cursor..range.start),
            0.0,
            plain.clone(),
        );
        job.append(text_between(&line.text, range.clone()), 0.0, marked.clone());
        cursor = range.end;
    }
    job.append(
        text_between(&line.text, cursor..line.text.len()),
        0.0,
        plain,
    );
    job
}

fn text_between(text: &str, range: Range<usize>) -> &str {
    text.get(range).unwrap_or_default()
}

/// A fold standing in for unchanged lines; clicking it shows every line. Returns whether it was clicked.
fn show_fold(ui: &mut egui::Ui, size: egui::Vec2, count: usize) -> bool {
    let palette = Palette::of(ui);
    let text = describe_fold(count);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &text));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let (background, color) = if response.hovered() {
            (palette.surface_hover, palette.text_strong)
        } else {
            (palette.surface, palette.text_weak)
        };
        painter.rect_filled(rect, 0, background);
        painter.text(
            egui::pos2(rect.left() + FOLD_TEXT_INSET, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(12.5),
            color,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn describe_fold(count: usize) -> String {
    if count == 1 {
        return "Show 1 unchanged line".to_owned();
    }
    format!("Show {count} unchanged lines")
}

fn describe_row(row: &DiffRow) -> String {
    let left = line_text(row.left.as_ref());
    let right = line_text(row.right.as_ref());
    match row.kind {
        RowKind::Equal => format!("Unchanged: {left}"),
        RowKind::Changed => format!("Changed: {left} → {right}"),
        RowKind::Removed => format!("Removed: {left}"),
        RowKind::Added => format!("Added: {right}"),
    }
}

fn describe_unified_line(tone: Tone, line: Option<&DiffLine>) -> String {
    let text = line_text(line);
    match tone {
        Tone::Removed => format!("Removed: {text}"),
        Tone::Added => format!("Added: {text}"),
        Tone::Plain | Tone::Missing => format!("Unchanged: {text}"),
    }
}

fn line_text(line: Option<&DiffLine>) -> &str {
    line.map_or("", |line| line.text.as_str())
}

fn count_display_chars(text: &str) -> usize {
    text.chars()
        .map(|character| {
            if character == '\t' {
                TAB_WIDTH_CHARS
            } else {
                1
            }
        })
        .sum()
}
