//! Epoch Converter: the current Unix time, timestamp ↔ date in any zone, period boundaries
//! and second counts as days, hours and minutes.

mod zone_picker;

#[cfg(test)]
mod tests;

use std::time::Duration;

use eframe::egui;
use oxbench_core::epoch::{
    DateTime, TimeUnit, Utc, Zone, compute_period_bounds, describe_calendar, describe_relative,
    detect_local_zone_name, format_iso8601, format_readable, format_rfc2822, format_timestamp,
    parse_calendar_date, parse_date_time, parse_seconds, parse_timestamp, split_duration,
};

use crate::clock::Clock;
use crate::style::icons;
use crate::style::palette::Palette;
use crate::tool::Tool;
use crate::widgets::card::show_card;
use crate::widgets::copy_button::show_copy_icon_button;
use crate::widgets::icon_button::icon_label_button;
use crate::widgets::input_row;
use crate::widgets::status::{StatusTone, show_status};
use crate::widgets::value_list::{ValueRow, show_value_list};
use zone_picker::{describe_zone, show_zone_picker};

/// Below this width the cards stack in one column.
const TWO_COLUMN_MIN_WIDTH: f32 = 820.0;
const DATE_INPUT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const CALENDAR_DATE_FORMAT: &str = "%Y-%m-%d";

pub struct EpochConverter {
    clock: Clock,
    /// Used for readable output, for dates typed without an offset, and for period boundaries.
    zone: Zone,
    local_zone_name: Option<String>,
    zone_filter: String,
    timestamp_input: String,
    /// `None` detects the unit from the number's size.
    timestamp_unit: Option<TimeUnit>,
    date_input: String,
    period_input: String,
    duration_input: String,
}

impl Default for EpochConverter {
    fn default() -> Self {
        Self::new(Clock::System, Zone::Local)
    }
}

impl EpochConverter {
    /// Starts with every input filled from the current time, so each card shows a result at once.
    pub fn new(clock: Clock, zone: Zone) -> Self {
        let now = clock.now();
        let zoned_now = zone.to_zoned(now);
        Self {
            clock,
            zone,
            local_zone_name: detect_local_zone_name(),
            zone_filter: String::new(),
            timestamp_input: format_timestamp(now, TimeUnit::Seconds),
            timestamp_unit: None,
            date_input: zoned_now.format(DATE_INPUT_FORMAT).to_string(),
            period_input: zoned_now.format(CALENDAR_DATE_FORMAT).to_string(),
            duration_input: "90061".to_owned(),
        }
    }

    fn zone_name(&self) -> String {
        describe_zone(self.zone, self.local_zone_name.as_deref())
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui, now: DateTime<Utc>) {
        ui.horizontal(|ui| {
            show_zone_picker(
                ui,
                &mut self.zone,
                &mut self.zone_filter,
                self.local_zone_name.as_deref(),
                now,
            );
        });
        ui.add_space(12.0);
    }

    fn show_current_time(&self, ui: &mut egui::Ui, now: DateTime<Utc>) {
        let palette = Palette::of(ui);
        show_card(ui, "Current Unix time", |ui| {
            ui.horizontal(|ui| {
                let seconds = format_timestamp(now, TimeUnit::Seconds);
                ui.label(
                    egui::RichText::new(&seconds)
                        .monospace()
                        .size(30.0)
                        .color(palette.accent),
                );
                show_copy_icon_button(ui, "Copy current seconds", &seconds);
                ui.add_space(24.0);
                let milliseconds = format_timestamp(now, TimeUnit::Milliseconds);
                ui.label(
                    egui::RichText::new(&milliseconds)
                        .monospace()
                        .size(18.0)
                        .color(palette.text),
                );
                ui.weak("ms");
                show_copy_icon_button(ui, "Copy current milliseconds", &milliseconds);
            });
            ui.label(
                egui::RichText::new(format_readable(self.zone.to_zoned(now)))
                    .color(palette.text_weak),
            );
        });
    }

    fn show_timestamp_to_date(&mut self, ui: &mut egui::Ui, now: DateTime<Utc>) {
        show_card(ui, "Timestamp to date", |ui| {
            let unit = &mut self.timestamp_unit;
            let is_now_clicked =
                show_input_row(ui, &mut self.timestamp_input, "1700000000", |ui| {
                    let is_clicked = ui.add(icon_label_button(icons::CLOCK, "Now")).clicked();
                    show_unit_picker(ui, unit);
                    is_clicked
                });
            if is_now_clicked {
                self.timestamp_input = format_timestamp(now, TimeUnit::Seconds);
                self.timestamp_unit = None;
            }
            ui.add_space(4.0);
            let parsed = match parse_timestamp(&self.timestamp_input, self.timestamp_unit) {
                Ok(parsed) => parsed,
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    return;
                }
            };
            if parsed.is_unit_detected {
                ui.weak(format!("Read as {}.", parsed.unit.label().to_lowercase()));
            }
            let utc = parsed.instant.fixed_offset();
            let zoned = self.zone.to_zoned(parsed.instant);
            let mut rows = vec![ValueRow::new("UTC", format_readable(utc))];
            if self.zone != Zone::Utc {
                rows.push(ValueRow::new(self.zone_name(), format_readable(zoned)));
            }
            rows.extend([
                ValueRow::new("ISO 8601", format_iso8601(zoned)),
                ValueRow::new("RFC 2822", format_rfc2822(utc)),
                ValueRow::new("Relative", describe_relative(parsed.instant, now)),
                ValueRow::new("Calendar", describe_calendar(zoned)),
            ]);
            show_value_list(ui, &rows);
        });
    }

    fn show_date_to_timestamp(&mut self, ui: &mut egui::Ui, now: DateTime<Utc>) {
        let zone_name = self.zone_name();
        show_card(ui, "Date to timestamp", |ui| {
            let is_now_clicked =
                show_input_row(ui, &mut self.date_input, "2024-03-15 14:30:00", |ui| {
                    ui.add(icon_label_button(icons::CLOCK, "Now")).clicked()
                });
            if is_now_clicked {
                self.date_input = self
                    .zone
                    .to_zoned(now)
                    .format(DATE_INPUT_FORMAT)
                    .to_string();
            }
            ui.weak(format!(
                "Read in {zone_name} unless it has its own offset, such as Z or +02:00."
            ));
            ui.add_space(4.0);
            let instant = match parse_date_time(&self.date_input, self.zone) {
                Ok(instant) => instant,
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    return;
                }
            };
            let rows = [
                ValueRow::new("Seconds", format_timestamp(instant, TimeUnit::Seconds)),
                ValueRow::new(
                    "Milliseconds",
                    format_timestamp(instant, TimeUnit::Milliseconds),
                ),
                ValueRow::new("ISO 8601 (UTC)", format_iso8601(instant.fixed_offset())),
                ValueRow::new(zone_name, format_readable(self.zone.to_zoned(instant))),
            ];
            show_value_list(ui, &rows);
        });
    }

    fn show_period_bounds(&mut self, ui: &mut egui::Ui, now: DateTime<Utc>) {
        let zone_name = self.zone_name();
        show_card(ui, "Start and end of period", |ui| {
            let is_today_clicked = show_input_row(ui, &mut self.period_input, "2024-03-15", |ui| {
                ui.add(icon_label_button(icons::CLOCK, "Today")).clicked()
            });
            if is_today_clicked {
                self.period_input = self
                    .zone
                    .to_zoned(now)
                    .format(CALENDAR_DATE_FORMAT)
                    .to_string();
            }
            ui.weak(format!(
                "Boundaries in {zone_name}. End is the last second of the period."
            ));
            ui.add_space(4.0);
            let bounds = parse_calendar_date(&self.period_input)
                .and_then(|date| compute_period_bounds(date, self.zone));
            let bounds = match bounds {
                Ok(bounds) => bounds,
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    return;
                }
            };
            let periods = [
                ("Day", bounds.day),
                ("Month", bounds.month),
                ("Year", bounds.year),
            ];
            let rows: Vec<ValueRow> = periods
                .into_iter()
                .flat_map(|(name, period)| {
                    [
                        ValueRow::new(
                            format!("{name} start"),
                            format_timestamp(period.start, TimeUnit::Seconds),
                        ),
                        ValueRow::new(
                            format!("{name} end"),
                            format_timestamp(period.end, TimeUnit::Seconds),
                        ),
                    ]
                })
                .collect();
            show_value_list(ui, &rows);
        });
    }

    fn show_duration(&mut self, ui: &mut egui::Ui) {
        show_card(ui, "Seconds to duration", |ui| {
            show_input_row(ui, &mut self.duration_input, "90061", |_| {});
            ui.add_space(4.0);
            let total_seconds = match parse_seconds(&self.duration_input) {
                Ok(total_seconds) => total_seconds,
                Err(error) => {
                    show_status(ui, StatusTone::Error, &error.to_string());
                    return;
                }
            };
            let parts = split_duration(total_seconds);
            let rows = [
                ValueRow::new("Duration", parts.describe()),
                ValueRow::new("Clock", parts.format_clock()),
                ValueRow::new("Minutes", format_fraction(total_seconds, 60)),
                ValueRow::new("Hours", format_fraction(total_seconds, 3_600)),
                ValueRow::new("Days", format_fraction(total_seconds, 86_400)),
            ];
            show_value_list(ui, &rows);
        });
    }
}

impl Tool for EpochConverter {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let now = self.clock.now();
        if self.clock.is_ticking() {
            let until_next_second = 1_000 - u64::from(now.timestamp_subsec_millis());
            ui.ctx()
                .request_repaint_after(Duration::from_millis(until_next_second));
        }
        egui::ScrollArea::vertical()
            .id_salt("epoch_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                self.show_toolbar(ui, now);
                self.show_current_time(ui, now);
                ui.add_space(12.0);
                if ui.available_width() < TWO_COLUMN_MIN_WIDTH {
                    self.show_timestamp_to_date(ui, now);
                    ui.add_space(12.0);
                    self.show_date_to_timestamp(ui, now);
                    ui.add_space(12.0);
                    self.show_period_bounds(ui, now);
                    ui.add_space(12.0);
                    self.show_duration(ui);
                    return;
                }
                ui.columns(2, |columns| {
                    self.show_timestamp_to_date(&mut columns[0], now);
                    columns[0].add_space(12.0);
                    self.show_period_bounds(&mut columns[0], now);
                    self.show_date_to_timestamp(&mut columns[1], now);
                    columns[1].add_space(12.0);
                    self.show_duration(&mut columns[1]);
                });
            });
    }
}

/// A monospace input that fills the row after `add_trailing` places its controls on the right.
/// Controls go right to left, so add the rightmost first.
fn show_input_row<R>(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    add_trailing: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let input = egui::TextEdit::singleline(text)
        .font(egui::TextStyle::Monospace)
        .hint_text(hint);
    input_row::show_input_row(ui, input, add_trailing).1
}

fn show_unit_picker(ui: &mut egui::Ui, unit: &mut Option<TimeUnit>) {
    // Added before its label because the row is laid out right to left.
    let combo = egui::ComboBox::from_id_salt("epoch_unit")
        .selected_text(unit.map_or("Auto", TimeUnit::label))
        .show_ui(ui, |ui| {
            ui.selectable_value(unit, None, "Auto");
            for choice in TimeUnit::ALL {
                ui.selectable_value(unit, Some(choice), choice.label());
            }
        });
    let label = ui.label("Unit");
    // Names the dropdown for screen readers, since its visible label sits beside it.
    let _ = combo.response.labelled_by(label.id);
}

/// `90061 / 60` as `1501.017`, trimmed of trailing zeros.
#[allow(
    clippy::cast_precision_loss,
    reason = "shown to three decimals; precision loss starts beyond 2^53 seconds"
)]
fn format_fraction(total_seconds: i64, unit_seconds: i64) -> String {
    let value = total_seconds as f64 / unit_seconds as f64;
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}
