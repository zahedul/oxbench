use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use super::zone::Zone;
use crate::{CoreError, CoreResult};

/// `%.f` also matches when there is no fraction.
const DATE_TIME_FORMATS: [&str; 8] = [
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%d %H:%M",
    "%Y/%m/%d %H:%M:%S%.f",
    "%Y/%m/%d %H:%M",
    "%d %B %Y %H:%M:%S",
    "%B %d, %Y %H:%M:%S",
];

const DATE_FORMATS: [&str; 6] = [
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%d %B %Y",
    "%B %d, %Y",
    "%d %b %Y",
    "%b %d, %Y",
];

/// Reads a date typed by a person. A date with its own offset (`Z`, `+02:00`, RFC 2822) keeps it;
/// anything else is read as wall-clock time in `zone`. A date alone means midnight.
pub fn parse_date_time(input: &str, zone: Zone) -> CoreResult<DateTime<Utc>> {
    let text = input.trim();
    if text.is_empty() {
        return Err(CoreError::InvalidInput(
            "Type a date, such as 2024-03-15 14:30:00.".to_owned(),
        ));
    }
    if let Some(instant) = parse_with_offset(text) {
        return Ok(instant);
    }
    let Some(naive) = parse_wall_clock(text) else {
        return Err(CoreError::InvalidInput(
            "Could not read this date. Try 2024-03-15 14:30:00, 2024-03-15T14:30:00Z or \
             Fri, 15 Mar 2024 14:30:00 +0000."
                .to_owned(),
        ));
    };
    zone.resolve(naive)
}

fn parse_with_offset(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .or_else(|_| DateTime::parse_from_rfc2822(text))
        .or_else(|_| DateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.f %z"))
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

fn parse_wall_clock(text: &str) -> Option<NaiveDateTime> {
    let date_time = DATE_TIME_FORMATS
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok());
    if date_time.is_some() {
        return date_time;
    }
    DATE_FORMATS
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(text, format).ok())
        .and_then(|date| date.and_hms_opt(0, 0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(input: &str, zone: Zone) -> i64 {
        parse_date_time(input, zone).unwrap().timestamp()
    }

    #[test]
    fn reads_iso_dates_in_the_given_zone() {
        assert_eq!(timestamp("2023-11-14 22:13:20", Zone::Utc), 1_700_000_000);
        assert_eq!(timestamp("2023-11-14T22:13:20", Zone::Utc), 1_700_000_000);
        assert_eq!(timestamp("2023/11/14 22:13", Zone::Utc), 1_699_999_980);
        let dhaka = Zone::Named(chrono_tz::Asia::Dhaka);
        assert_eq!(timestamp("2023-11-15 04:13:20", dhaka), 1_700_000_000);
    }

    #[test]
    fn an_explicit_offset_wins_over_the_zone() {
        let tokyo = Zone::Named(chrono_tz::Asia::Tokyo);
        assert_eq!(timestamp("2023-11-14T22:13:20Z", tokyo), 1_700_000_000);
        assert_eq!(timestamp("2023-11-15T00:13:20+02:00", tokyo), 1_700_000_000);
        assert_eq!(
            timestamp("Tue, 14 Nov 2023 22:13:20 +0000", tokyo),
            1_700_000_000
        );
        assert_eq!(timestamp("2023-11-14 23:13:20 +0100", tokyo), 1_700_000_000);
    }

    #[test]
    fn keeps_fractional_seconds() {
        let instant = parse_date_time("2023-11-14 22:13:20.125", Zone::Utc).unwrap();
        assert_eq!(instant.timestamp_subsec_millis(), 125);
    }

    #[test]
    fn a_date_alone_means_midnight() {
        assert_eq!(timestamp("2024-01-01", Zone::Utc), 1_704_067_200);
        assert_eq!(timestamp("1 January 2024", Zone::Utc), 1_704_067_200);
        assert_eq!(timestamp("Jan 1, 2024", Zone::Utc), 1_704_067_200);
    }

    #[test]
    fn rejects_unreadable_and_impossible_dates() {
        assert!(parse_date_time("", Zone::Utc).is_err());
        assert!(parse_date_time("next tuesday", Zone::Utc).is_err());
        assert!(parse_date_time("2023-02-30", Zone::Utc).is_err());
    }
}
