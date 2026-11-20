use crate::{CoreError, CoreResult};

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurationParts {
    pub is_negative: bool,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: u64,
}

impl DurationParts {
    /// `1 day, 2 hours, 3 minutes and 4 seconds`, leaving out zero parts.
    pub fn describe(&self) -> String {
        let parts: Vec<String> = [
            (self.days, "day"),
            (self.hours, "hour"),
            (self.minutes, "minute"),
            (self.seconds, "second"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, unit)| pluralize(count, unit))
        .collect();
        let sentence = match parts.as_slice() {
            [] => return "0 seconds".to_owned(),
            [only] => only.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        };
        if self.is_negative {
            format!("minus {sentence}")
        } else {
            sentence
        }
    }

    /// `1d 02:03:04`, compact and fixed-width.
    pub fn format_clock(&self) -> String {
        let sign = if self.is_negative { "-" } else { "" };
        format!(
            "{sign}{}d {:02}:{:02}:{:02}",
            self.days, self.hours, self.minutes, self.seconds
        )
    }
}

pub fn split_duration(total_seconds: i64) -> DurationParts {
    let magnitude = total_seconds.unsigned_abs();
    DurationParts {
        is_negative: total_seconds < 0,
        days: magnitude / SECONDS_PER_DAY,
        hours: magnitude % SECONDS_PER_DAY / SECONDS_PER_HOUR,
        minutes: magnitude % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        seconds: magnitude % SECONDS_PER_MINUTE,
    }
}

/// Reads a whole number of seconds, allowing `_` and `,` separators.
pub fn parse_seconds(input: &str) -> CoreResult<i64> {
    let cleaned: String = input
        .trim()
        .chars()
        .filter(|c| !matches!(c, '_' | ',' | ' '))
        .collect();
    if cleaned.is_empty() {
        return Err(CoreError::InvalidInput(
            "Type a number of seconds, such as 90061.".to_owned(),
        ));
    }
    cleaned.parse().map_err(|_| {
        CoreError::InvalidInput(format!(
            "\"{}\" is not a whole number of seconds.",
            input.trim()
        ))
    })
}

fn pluralize(count: u64, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_seconds_into_parts() {
        let parts = split_duration(90_061);
        assert_eq!(
            (parts.days, parts.hours, parts.minutes, parts.seconds),
            (1, 1, 1, 1)
        );
        assert_eq!(parts.describe(), "1 day, 1 hour, 1 minute and 1 second");
        assert_eq!(parts.format_clock(), "1d 01:01:01");
    }

    #[test]
    fn leaves_out_zero_parts() {
        assert_eq!(split_duration(0).describe(), "0 seconds");
        assert_eq!(split_duration(7_200).describe(), "2 hours");
        assert_eq!(split_duration(172_830).describe(), "2 days and 30 seconds");
    }

    #[test]
    fn handles_negative_and_extreme_values() {
        assert_eq!(split_duration(-3_600).describe(), "minus 1 hour");
        assert_eq!(split_duration(-61).format_clock(), "-0d 00:01:01");
        assert_eq!(split_duration(i64::MIN).days, 106_751_991_167_300);
    }

    #[test]
    fn parses_seconds() {
        assert_eq!(parse_seconds(" 1,000 ").unwrap(), 1_000);
        assert_eq!(parse_seconds("-5").unwrap(), -5);
        assert!(parse_seconds("").is_err());
        assert!(parse_seconds("1.5").is_err());
    }
}
