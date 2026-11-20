use chrono::{DateTime, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::{CoreError, CoreResult};

/// Where a wall-clock time is read or shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Utc,
    /// The operating system's zone, including its daylight-saving rules.
    Local,
    Named(Tz),
}

impl Zone {
    pub fn name(self) -> &'static str {
        match self {
            Zone::Utc => "UTC",
            Zone::Local => "Local",
            Zone::Named(tz) => tz.name(),
        }
    }

    pub fn to_zoned(self, instant: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self {
            Zone::Utc => instant.fixed_offset(),
            Zone::Local => instant.with_timezone(&Local).fixed_offset(),
            Zone::Named(tz) => instant.with_timezone(&tz).fixed_offset(),
        }
    }

    /// Reads a wall-clock time in this zone. When the clocks go back, the time happens twice;
    /// the earlier one is used, as most tools do.
    pub fn resolve(self, naive: NaiveDateTime) -> CoreResult<DateTime<Utc>> {
        match self.map_local(naive) {
            LocalResult::Single(instant) | LocalResult::Ambiguous(instant, _) => Ok(instant),
            LocalResult::None => Err(CoreError::InvalidInput(format!(
                "{} does not exist in {}: the clocks skip it for daylight saving.",
                naive.format("%Y-%m-%d %H:%M:%S"),
                self.name()
            ))),
        }
    }

    /// Like `resolve`, but a time inside a daylight-saving gap maps to the moment the clocks jump,
    /// which is where a day that "starts" in the gap really begins.
    pub(super) fn resolve_boundary(self, naive: NaiveDateTime) -> CoreResult<DateTime<Utc>> {
        if let LocalResult::Single(instant) | LocalResult::Ambiguous(instant, _) =
            self.map_local(naive)
        {
            return Ok(instant);
        }
        // Gaps last at most a few hours, so three hours earlier is safely before it.
        let before_gap = self.resolve(naive - chrono::TimeDelta::hours(3))?;
        let offset_before_gap = self.to_zoned(before_gap).offset().local_minus_utc();
        Ok((naive - chrono::TimeDelta::seconds(offset_before_gap.into())).and_utc())
    }

    fn map_local(self, naive: NaiveDateTime) -> LocalResult<DateTime<Utc>> {
        match self {
            Zone::Utc => LocalResult::Single(naive.and_utc()),
            Zone::Local => Local
                .from_local_datetime(&naive)
                .map(|time| time.with_timezone(&Utc)),
            Zone::Named(tz) => tz
                .from_local_datetime(&naive)
                .map(|time| time.with_timezone(&Utc)),
        }
    }
}

/// Every IANA zone, sorted by name.
pub fn list_named_zones() -> &'static [Tz] {
    &chrono_tz::TZ_VARIANTS
}

/// The system zone's IANA name, such as `Europe/London`, when the platform reports one.
pub fn detect_local_zone_name() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn naive(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn named_zone_applies_its_offset() {
        let zone = Zone::Named(chrono_tz::Asia::Dhaka);
        let instant = zone.resolve(naive("2024-01-01 06:00:00")).unwrap();
        assert_eq!(instant.timestamp(), 1_704_067_200);
        assert_eq!(zone.to_zoned(instant).offset().local_minus_utc(), 6 * 3600);
    }

    #[test]
    fn repeated_hour_resolves_to_the_earlier_time() {
        let zone = Zone::Named(chrono_tz::Europe::London);
        let instant = zone.resolve(naive("2024-10-27 01:30:00")).unwrap();
        assert_eq!(zone.to_zoned(instant).offset().local_minus_utc(), 3600);
    }

    #[test]
    fn skipped_hour_is_an_error() {
        let zone = Zone::Named(chrono_tz::Europe::London);
        let error = zone.resolve(naive("2024-03-31 01:30:00")).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not exist in Europe/London")
        );
    }

    #[test]
    fn boundary_in_a_gap_moves_to_the_jump() {
        // Havana skipped midnight on 2024-03-10: clocks went from 00:00 straight to 01:00.
        let zone = Zone::Named(chrono_tz::America::Havana);
        let midnight = NaiveDate::from_ymd_opt(2024, 3, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let start = zone.resolve_boundary(midnight).unwrap();
        assert_eq!(start.to_rfc3339(), "2024-03-10T05:00:00+00:00");
    }

    #[test]
    fn lists_zones_by_name() {
        let zones = list_named_zones();
        assert!(zones.len() > 400);
        assert!(zones.contains(&chrono_tz::Asia::Tokyo));
    }
}
