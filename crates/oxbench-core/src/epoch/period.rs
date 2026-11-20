use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeDelta, Utc};

use super::zone::Zone;
use crate::{CoreError, CoreResult};

/// A span of whole seconds. `end` is the last second inside it, such as 23:59:59.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Period {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodBounds {
    pub day: Period,
    pub month: Period,
    pub year: Period,
}

/// The day, month and year that contain `date`, as wall-clock periods in `zone`.
pub fn compute_period_bounds(date: NaiveDate, zone: Zone) -> CoreResult<PeriodBounds> {
    let first_of_month = date.with_day(1).ok_or_else(out_of_range)?;
    let first_of_year = date.with_ordinal(1).ok_or_else(out_of_range)?;
    let next_day = date.succ_opt().ok_or_else(out_of_range)?;
    let next_month = first_of_month
        .checked_add_months(chrono::Months::new(1))
        .ok_or_else(out_of_range)?;
    let next_year = first_of_year
        .checked_add_months(chrono::Months::new(12))
        .ok_or_else(out_of_range)?;
    Ok(PeriodBounds {
        day: build_period(date, next_day, zone)?,
        month: build_period(first_of_month, next_month, zone)?,
        year: build_period(first_of_year, next_year, zone)?,
    })
}

/// Reads `2024-03-15` or `2024/03/15`.
pub fn parse_calendar_date(input: &str) -> CoreResult<NaiveDate> {
    let text = input.trim();
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(text, "%Y/%m/%d"))
        .map_err(|_| CoreError::InvalidInput("Type a date as YYYY-MM-DD.".to_owned()))
}

fn build_period(first_day: NaiveDate, next_first_day: NaiveDate, zone: Zone) -> CoreResult<Period> {
    let start = zone.resolve_boundary(midnight(first_day))?;
    let next_start = zone.resolve_boundary(midnight(next_first_day))?;
    Ok(Period {
        start,
        end: next_start - TimeDelta::seconds(1),
    })
}

fn midnight(date: NaiveDate) -> NaiveDateTime {
    date.and_time(chrono::NaiveTime::MIN)
}

fn out_of_range() -> CoreError {
    CoreError::InvalidInput("That date is outside the supported range.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(date: &str, zone: Zone) -> PeriodBounds {
        compute_period_bounds(parse_calendar_date(date).unwrap(), zone).unwrap()
    }

    fn timestamps(period: Period) -> (i64, i64) {
        (period.start.timestamp(), period.end.timestamp())
    }

    #[test]
    fn computes_utc_day_month_and_year() {
        let bounds = bounds("2024-02-15", Zone::Utc);
        assert_eq!(timestamps(bounds.day), (1_707_955_200, 1_708_041_599));
        assert_eq!(timestamps(bounds.month), (1_706_745_600, 1_709_251_199));
        assert_eq!(timestamps(bounds.year), (1_704_067_200, 1_735_689_599));
    }

    #[test]
    fn applies_the_zone_offset() {
        let bounds = bounds("2024-01-01", Zone::Named(chrono_tz::Asia::Dhaka));
        assert_eq!(bounds.day.start.to_rfc3339(), "2023-12-31T18:00:00+00:00");
        assert_eq!(bounds.day.end.to_rfc3339(), "2024-01-01T17:59:59+00:00");
    }

    #[test]
    fn a_daylight_saving_day_is_23_hours_long() {
        let bounds = bounds("2024-03-31", Zone::Named(chrono_tz::Europe::London));
        let length = bounds.day.end - bounds.day.start + TimeDelta::seconds(1);
        assert_eq!(length, TimeDelta::hours(23));
    }

    #[test]
    fn december_rolls_over_to_the_next_year() {
        let bounds = bounds("2023-12-31", Zone::Utc);
        assert_eq!(bounds.month.end.to_rfc3339(), "2023-12-31T23:59:59+00:00");
        assert_eq!(bounds.year.end, bounds.day.end);
    }

    #[test]
    fn parses_calendar_dates() {
        assert!(parse_calendar_date("2024/02/29").is_ok());
        assert!(parse_calendar_date("2023-02-29").is_err());
        assert!(parse_calendar_date("today").is_err());
    }
}
