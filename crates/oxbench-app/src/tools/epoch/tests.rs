//! Drives the Epoch Converter headlessly. The clock is pinned to 1700000000
//! (Tuesday, 14 November 2023 22:13:20 UTC).

use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oxbench_core::epoch::{DateTime, TimeUnit, Tz, Zone};

use super::EpochConverter;
use crate::clock::Clock;
use crate::style::install_style;
use crate::tool::Tool;

const PINNED_SECONDS: i64 = 1_700_000_000;

fn open_converter(zone: Zone) -> Harness<'static, EpochConverter> {
    let pinned = DateTime::from_timestamp(PINNED_SECONDS, 0).unwrap_or_default();
    let mut harness = Harness::builder()
        .with_context_setup(install_style)
        .with_size(eframe::egui::vec2(1200.0, 900.0))
        .build_ui_state(
            |ui, converter: &mut EpochConverter| converter.ui(ui),
            EpochConverter::new(Clock::Fixed(pinned), zone),
        );
    harness.run();
    harness
}

fn tokyo() -> Zone {
    Zone::Named(Tz::Asia__Tokyo)
}

fn rerun_with(
    harness: &mut Harness<'static, EpochConverter>,
    edit: impl FnOnce(&mut EpochConverter),
) {
    edit(harness.state_mut());
    harness.run();
}

#[test]
fn starts_with_the_current_time_and_filled_cards() {
    let harness = open_converter(Zone::Utc);
    harness.get_by_label("Current Unix time");
    // The current time, and the date card, which starts filled with the current time.
    assert_eq!(harness.query_all_by_label("1700000000000").count(), 2);
    harness.get_by_label("now");
    harness.get_by_label("1 day, 1 hour, 1 minute and 1 second");
}

#[test]
fn detects_milliseconds_and_shows_every_format() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| {
        converter.timestamp_input = "1700000000123".to_owned();
    });
    harness.get_by_label("Read as milliseconds.");
    harness.get_by_label("2023-11-14T22:13:20.123Z");
    harness.get_by_label("Tue, 14 Nov 2023 22:13:20 +0000");
    harness.get_by_label("Day 318 of 2023 · ISO week 46 of 2023");
}

#[test]
fn a_chosen_unit_overrides_detection() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| {
        converter.timestamp_unit = Some(TimeUnit::Milliseconds);
    });
    harness.get_by_label("1970-01-20T16:13:20Z");
    assert!(harness.query_by_label("Read as seconds.").is_none());
}

#[test]
fn shows_the_chosen_zone_beside_utc() {
    let mut harness = open_converter(tokyo());
    rerun_with(&mut harness, |converter| {
        converter.timestamp_input = "0".to_owned();
    });
    harness.get_by_label("Thursday, 1 January 1970 00:00:00 UTC");
    harness.get_by_label("Thursday, 1 January 1970 09:00:00 UTC+09:00");
    harness.get_by_label("1970-01-01T09:00:00+09:00");
    harness.get_by_label("53 years ago");
    assert!(harness.query_all_by_label("Asia/Tokyo").count() >= 1);
}

#[test]
fn reports_an_unreadable_timestamp() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| {
        converter.timestamp_input = "abc".to_owned();
    });
    harness.get_by_label_contains("\"abc\" is not a number.");
}

#[test]
fn now_button_refills_the_timestamp() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| converter.timestamp_input.clear());
    harness.get_by_label("Type a Unix timestamp, such as 1700000000.");
    let now_buttons: Vec<_> = harness.query_all_by_label_contains("Now").collect();
    now_buttons[0].click();
    harness.run();
    assert_eq!(harness.state().timestamp_input, "1700000000");
}

#[test]
fn reads_dates_in_the_chosen_zone_unless_they_carry_an_offset() {
    let mut harness = open_converter(tokyo());
    rerun_with(&mut harness, |converter| {
        converter.date_input = "2024-01-01 09:00:00".to_owned();
    });
    harness.get_by_label("1704067200");
    harness.get_by_label("1704067200000");
    harness.get_by_label("2024-01-01T00:00:00Z");

    rerun_with(&mut harness, |converter| {
        converter.date_input = "2024-01-01T09:00:00Z".to_owned();
    });
    harness.get_by_label("1704099600");
}

#[test]
fn reports_an_unreadable_date() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| {
        converter.date_input = "next tuesday".to_owned();
    });
    harness.get_by_label_contains("Could not read this date.");
}

#[test]
fn lists_day_month_and_year_boundaries() {
    let mut harness = open_converter(Zone::Utc);
    rerun_with(&mut harness, |converter| {
        converter.period_input = "2024-02-15".to_owned();
    });
    for timestamp in [
        "1707955200",
        "1708041599",
        "1706745600",
        "1709251199",
        "1704067200",
        "1735689599",
    ] {
        harness.get_by_label(timestamp);
    }
}

#[test]
fn breaks_seconds_into_a_duration() {
    let mut harness = open_converter(Zone::Utc);
    harness.get_by_label("1d 01:01:01");
    harness.get_by_label("1501.017");
    harness.get_by_label("25.017");
    harness.get_by_label("1.042");
    rerun_with(&mut harness, |converter| {
        converter.duration_input = "1.5".to_owned();
    });
    harness.get_by_label_contains("is not a whole number of seconds.");
}

#[test]
fn zone_picker_filters_and_picks_a_zone() {
    let mut harness = open_converter(Zone::Utc);
    harness
        .get_all_by_role(Role::ComboBox)
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.state_mut().zone_filter = "kolkata".to_owned();
    harness.run();
    assert!(harness.query_by_label_contains("Europe/London").is_none());
    harness
        .get_by_label_contains("Asia/Kolkata  (UTC+05:30)")
        .click();
    harness.run();
    assert_eq!(harness.state().zone, Zone::Named(Tz::Asia__Kolkata));
    assert_eq!(harness.state().zone_filter, "");
    assert!(
        harness
            .query_by_label_contains("Asia/Kolkata  (UTC+05:30)")
            .is_none()
    );
}
