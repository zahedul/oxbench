use chrono::{DateTime, Utc};

/// Largest unit first; months and years are average lengths, which is fine for a rough phrase.
const UNITS: [(i64, &str); 6] = [
    (365 * 24 * 3600, "year"),
    (30 * 24 * 3600, "month"),
    (24 * 3600, "day"),
    (3600, "hour"),
    (60, "minute"),
    (1, "second"),
];

/// `3 hours ago`, `in 2 days` or `now`, using the largest unit that fits.
pub fn describe_relative(instant: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let difference = (instant - now).num_seconds();
    let magnitude = difference.unsigned_abs();
    let Some((count, unit)) = UNITS.iter().find_map(|(unit_seconds, unit)| {
        let count = magnitude / unit_seconds.unsigned_abs();
        (count > 0).then_some((count, *unit))
    }) else {
        return "now".to_owned();
    };
    let amount = if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    };
    if difference < 0 {
        format!("{amount} ago")
    } else {
        format!("in {amount}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn describe(offset_seconds: i64) -> String {
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        describe_relative(now + TimeDelta::seconds(offset_seconds), now)
    }

    #[test]
    fn picks_the_largest_unit() {
        assert_eq!(describe(0), "now");
        assert_eq!(describe(-1), "1 second ago");
        assert_eq!(describe(-150), "2 minutes ago");
        assert_eq!(describe(3 * 3600), "in 3 hours");
        assert_eq!(describe(-45 * 24 * 3600), "1 month ago");
        assert_eq!(describe(800 * 24 * 3600), "in 2 years");
    }
}
