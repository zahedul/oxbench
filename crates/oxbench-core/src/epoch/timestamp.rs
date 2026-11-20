use chrono::{DateTime, Utc};

use crate::{CoreError, CoreResult};

const NANOS_PER_SECOND: i128 = 1_000_000_000;
/// Longer numbers cannot be a real date in any unit, and keep the arithmetic far from overflow.
const MAX_DIGITS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl TimeUnit {
    pub const ALL: [TimeUnit; 4] = [
        TimeUnit::Seconds,
        TimeUnit::Milliseconds,
        TimeUnit::Microseconds,
        TimeUnit::Nanoseconds,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            TimeUnit::Seconds => "Seconds",
            TimeUnit::Milliseconds => "Milliseconds",
            TimeUnit::Microseconds => "Microseconds",
            TimeUnit::Nanoseconds => "Nanoseconds",
        }
    }

    const fn nanos_per_unit(self) -> i128 {
        match self {
            TimeUnit::Seconds => NANOS_PER_SECOND,
            TimeUnit::Milliseconds => 1_000_000,
            TimeUnit::Microseconds => 1_000,
            TimeUnit::Nanoseconds => 1,
        }
    }

    const fn fraction_digits(self) -> usize {
        match self {
            TimeUnit::Seconds => 9,
            TimeUnit::Milliseconds => 6,
            TimeUnit::Microseconds => 3,
            TimeUnit::Nanoseconds => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedTimestamp {
    pub instant: DateTime<Utc>,
    pub unit: TimeUnit,
    /// False when the caller chose the unit.
    pub is_unit_detected: bool,
}

/// Guesses the unit from the size of the number. Each unit covers roughly the years 1973 to 5138,
/// so present-day timestamps are never ambiguous.
pub fn detect_unit(whole_units: i128) -> TimeUnit {
    match whole_units.unsigned_abs() {
        0..100_000_000_000 => TimeUnit::Seconds,
        100_000_000_000..100_000_000_000_000 => TimeUnit::Milliseconds,
        100_000_000_000_000..100_000_000_000_000_000 => TimeUnit::Microseconds,
        _ => TimeUnit::Nanoseconds,
    }
}

/// Reads `1700000000`, `-86400`, `1700000000.5` or `1_700_000_000_000`. With `unit` as `None`
/// the unit is detected from the number's size.
pub fn parse_timestamp(input: &str, unit: Option<TimeUnit>) -> CoreResult<ParsedTimestamp> {
    let cleaned: String = input
        .trim()
        .chars()
        .filter(|c| !matches!(c, '_' | ',' | ' '))
        .collect();
    if cleaned.is_empty() {
        return Err(CoreError::InvalidInput(
            "Type a Unix timestamp, such as 1700000000.".to_owned(),
        ));
    }
    let (is_negative, unsigned) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let (whole_text, fraction_text) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let is_numeric = !whole_text.is_empty()
        && whole_text.bytes().all(|b| b.is_ascii_digit())
        && fraction_text.bytes().all(|b| b.is_ascii_digit());
    if !is_numeric {
        return Err(CoreError::InvalidInput(format!(
            "\"{}\" is not a number. A timestamp is whole units since 1970-01-01 UTC.",
            input.trim()
        )));
    }
    if whole_text.len() > MAX_DIGITS {
        return Err(CoreError::InvalidInput(
            "That number is too long to be a timestamp.".to_owned(),
        ));
    }
    let whole: i128 = parse_digits(whole_text)?;
    let detected_unit = unit.unwrap_or_else(|| detect_unit(whole));
    let magnitude = whole * detected_unit.nanos_per_unit()
        + parse_fraction_nanos(fraction_text, detected_unit)?;
    let total_nanos = if is_negative { -magnitude } else { magnitude };
    Ok(ParsedTimestamp {
        instant: instant_from_nanos(total_nanos)?,
        unit: detected_unit,
        is_unit_detected: unit.is_none(),
    })
}

/// The instant as whole units since the epoch, rounded down like `Date.now()` and `time.time()`.
pub fn format_timestamp(instant: DateTime<Utc>, unit: TimeUnit) -> String {
    let total_nanos = i128::from(instant.timestamp()) * NANOS_PER_SECOND
        + i128::from(instant.timestamp_subsec_nanos());
    total_nanos.div_euclid(unit.nanos_per_unit()).to_string()
}

fn parse_digits(digits: &str) -> CoreResult<i128> {
    digits
        .parse()
        .map_err(|error| CoreError::InvalidInput(format!("Could not read the number: {error}")))
}

/// Digits past the unit's nanosecond precision are dropped.
fn parse_fraction_nanos(fraction_text: &str, unit: TimeUnit) -> CoreResult<i128> {
    let precision = unit.fraction_digits();
    if fraction_text.is_empty() || precision == 0 {
        return Ok(0);
    }
    let kept = &fraction_text[..fraction_text.len().min(precision)];
    let padded = format!("{kept:0<precision$}");
    parse_digits(&padded)
}

fn instant_from_nanos(total_nanos: i128) -> CoreResult<DateTime<Utc>> {
    let out_of_range = || {
        CoreError::InvalidInput(
            "That timestamp is outside the supported range (about 262,000 years either side of 1970)."
                .to_owned(),
        )
    };
    let seconds =
        i64::try_from(total_nanos.div_euclid(NANOS_PER_SECOND)).map_err(|_| out_of_range())?;
    let nanos =
        u32::try_from(total_nanos.rem_euclid(NANOS_PER_SECOND)).map_err(|_| out_of_range())?;
    DateTime::from_timestamp(seconds, nanos).ok_or_else(out_of_range)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc3339(input: &str) -> String {
        parse_timestamp(input, None).unwrap().instant.to_rfc3339()
    }

    #[test]
    fn detects_the_unit_from_the_number_of_digits() {
        assert_eq!(detect_unit(1_700_000_000), TimeUnit::Seconds);
        assert_eq!(detect_unit(1_700_000_000_000), TimeUnit::Milliseconds);
        assert_eq!(detect_unit(1_700_000_000_000_000), TimeUnit::Microseconds);
        assert_eq!(
            detect_unit(1_700_000_000_000_000_000),
            TimeUnit::Nanoseconds
        );
        assert_eq!(detect_unit(-1_700_000_000_000), TimeUnit::Milliseconds);
    }

    #[test]
    fn parses_every_unit_to_the_same_instant() {
        let expected = "2023-11-14T22:13:20+00:00";
        assert_eq!(rfc3339("1700000000"), expected);
        assert_eq!(rfc3339("1700000000000"), expected);
        assert_eq!(rfc3339("1700000000000000"), expected);
        assert_eq!(rfc3339("1700000000000000000"), expected);
    }

    #[test]
    fn accepts_separators_signs_and_fractions() {
        assert_eq!(rfc3339(" 1_700_000_000 "), "2023-11-14T22:13:20+00:00");
        assert_eq!(rfc3339("1,700,000,000"), "2023-11-14T22:13:20+00:00");
        assert_eq!(rfc3339("-86400"), "1969-12-31T00:00:00+00:00");
        assert_eq!(rfc3339("+0"), "1970-01-01T00:00:00+00:00");
        assert_eq!(rfc3339("1700000000.25"), "2023-11-14T22:13:20.250+00:00");
        assert_eq!(rfc3339("-1.5"), "1969-12-31T23:59:58.500+00:00");
    }

    #[test]
    fn a_chosen_unit_overrides_detection() {
        let parsed = parse_timestamp("1700000000", Some(TimeUnit::Milliseconds)).unwrap();
        assert_eq!(parsed.instant.to_rfc3339(), "1970-01-20T16:13:20+00:00");
        assert!(!parsed.is_unit_detected);
    }

    #[test]
    fn rejects_text_empty_input_and_huge_numbers() {
        assert!(parse_timestamp("", None).is_err());
        assert!(parse_timestamp("12ab", None).is_err());
        assert!(parse_timestamp("1.2.3", None).is_err());
        assert!(parse_timestamp(".5", None).is_err());
        assert!(parse_timestamp("9999999999999999999999999", None).is_err());
        let error = parse_timestamp("99999999999999", Some(TimeUnit::Seconds)).unwrap_err();
        assert!(error.to_string().contains("outside the supported range"));
    }

    #[test]
    fn formats_in_each_unit_rounding_down() {
        let instant = parse_timestamp("-1.5", None).unwrap().instant;
        assert_eq!(format_timestamp(instant, TimeUnit::Seconds), "-2");
        assert_eq!(format_timestamp(instant, TimeUnit::Milliseconds), "-1500");
        assert_eq!(
            format_timestamp(instant, TimeUnit::Nanoseconds),
            "-1500000000"
        );
    }
}
