use chrono::{DateTime, Datelike, FixedOffset, SecondsFormat, Timelike};

/// `2023-11-14T22:13:20Z`, with milliseconds or finer only when present.
pub fn format_iso8601(time: DateTime<FixedOffset>) -> String {
    time.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// `Tue, 14 Nov 2023 22:13:20 +0000`, the format used in email and HTTP headers.
pub fn format_rfc2822(time: DateTime<FixedOffset>) -> String {
    time.to_rfc2822()
}

/// `Tuesday, 14 November 2023 22:13:20.250 UTC+06:00`, for reading at a glance.
pub fn format_readable(time: DateTime<FixedOffset>) -> String {
    let fraction = if time.nanosecond() == 0 { "" } else { "%.3f" };
    let pattern = format!("%A, %-d %B %Y %H:%M:%S{fraction}");
    format!(
        "{} {}",
        time.format(&pattern),
        format_offset(time.offset().local_minus_utc())
    )
}

/// `UTC`, `UTC+05:30` or `UTC-08:00`.
pub fn format_offset(offset_seconds: i32) -> String {
    if offset_seconds == 0 {
        return "UTC".to_owned();
    }
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let minutes = offset_seconds.unsigned_abs() / 60;
    format!("UTC{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

/// `Day 318 of 2023 · ISO week 46`.
pub fn describe_calendar(time: DateTime<FixedOffset>) -> String {
    let iso_week = time.iso_week();
    format!(
        "Day {} of {} · ISO week {} of {}",
        time.ordinal(),
        time.year(),
        iso_week.week(),
        iso_week.year()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn formats_iso_with_z_and_optional_fraction() {
        assert_eq!(
            format_iso8601(time("2023-11-14T22:13:20+00:00")),
            "2023-11-14T22:13:20Z"
        );
        assert_eq!(
            format_iso8601(time("2023-11-15T04:13:20.250+06:00")),
            "2023-11-15T04:13:20.250+06:00"
        );
    }

    #[test]
    fn formats_rfc2822() {
        assert_eq!(
            format_rfc2822(time("2023-11-14T22:13:20+00:00")),
            "Tue, 14 Nov 2023 22:13:20 +0000"
        );
    }

    #[test]
    fn formats_readable_dates_with_the_offset() {
        assert_eq!(
            format_readable(time("2023-11-14T22:13:20+00:00")),
            "Tuesday, 14 November 2023 22:13:20 UTC"
        );
        assert_eq!(
            format_readable(time("2023-11-14T14:13:20.5-08:00")),
            "Tuesday, 14 November 2023 14:13:20.500 UTC-08:00"
        );
    }

    #[test]
    fn formats_offsets_with_minutes() {
        assert_eq!(format_offset(0), "UTC");
        assert_eq!(format_offset(19_800), "UTC+05:30");
        assert_eq!(format_offset(-34_200), "UTC-09:30");
    }

    #[test]
    fn describes_the_day_and_iso_week() {
        assert_eq!(
            describe_calendar(time("2023-11-14T22:13:20+00:00")),
            "Day 318 of 2023 · ISO week 46 of 2023"
        );
        assert_eq!(
            describe_calendar(time("2021-01-01T00:00:00+00:00")),
            "Day 1 of 2021 · ISO week 53 of 2020"
        );
    }
}
