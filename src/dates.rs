//! Date resolution. A [`DateSpec`] resolves to a `[start, end)` epoch-seconds
//! (UTC) range; a comparator turns that range into a match. Consumers translate
//! the resolved range into their own SQL or eval-side comparisons; this crate
//! deliberately ships no query translation.

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Utc};

use crate::ast::{Comparator, DateSpec};

/// Resolve a spec to a `[start, end)` epoch-seconds range relative to `today`.
pub fn resolve_range(spec: &DateSpec, today: NaiveDate) -> (i64, i64) {
    let (start, end) = match spec {
        DateSpec::Today => (today, next_day(today)),
        DateSpec::Yesterday => (prev_day(today), today),
        DateSpec::Tomorrow => (next_day(today), next_day(next_day(today))),
        DateSpec::LastWeek => {
            let monday = sub_days(
                sub_days(today, today.weekday().num_days_from_monday() as u64),
                7,
            );
            (monday, add_days(monday, 7))
        }
        DateSpec::NextWeek => {
            let monday = add_days(
                sub_days(today, today.weekday().num_days_from_monday() as u64),
                7,
            );
            (monday, add_days(monday, 7))
        }
        DateSpec::ThisWeek => {
            let monday = sub_days(today, today.weekday().num_days_from_monday() as u64);
            (monday, add_days(monday, 7))
        }
        DateSpec::ThisMonth => {
            let first = ymd(today.year(), today.month(), 1);
            (first, add_month(first))
        }
        DateSpec::LastMonth => {
            let first = ymd(today.year(), today.month(), 1);
            (sub_month(first), first)
        }
        DateSpec::NextMonth => {
            let first = ymd(today.year(), today.month(), 1);
            let next = add_month(first);
            (next, add_month(next))
        }
        DateSpec::ThisYear => (ymd(today.year(), 1, 1), ymd(today.year() + 1, 1, 1)),
        DateSpec::DaysAgo(n) => {
            let d = sub_days(today, *n as u64);
            (d, next_day(d))
        }
        DateSpec::InDays(n) => {
            let d = add_days(today, *n as u64);
            (d, next_day(d))
        }
        DateSpec::Ymd(y, None, _) => (ymd(*y, 1, 1), ymd(*y + 1, 1, 1)),
        DateSpec::Ymd(y, Some(m), None) => {
            let first = ymd(*y, *m, 1);
            (first, add_month(first))
        }
        DateSpec::Ymd(y, Some(m), Some(d)) => {
            let day = ymd(*y, *m, *d);
            (day, next_day(day))
        }
    };
    (start_epoch(start), start_epoch(end))
}

/// Does `value` (epoch seconds) satisfy `comp` against the `[start, end)` range?
/// Precision-aware: `=today` is "within today", `>today` is "strictly after
/// today", `>=today` is "today or later", and so on.
pub fn matches(comp: Comparator, value: i64, start: i64, end: i64) -> bool {
    match comp {
        Comparator::Eq => value >= start && value < end,
        Comparator::Ne => value < start || value >= end,
        Comparator::Lt => value < start,
        Comparator::Le => value < end,
        Comparator::Gt => value >= end,
        Comparator::Ge => value >= start,
    }
}

/// `d + n`, saturating at the latest representable date. Never panics:
/// resolution runs on consumer input, so an absurd offset
/// (`added:4294967295daysago`) clamps instead of aborting the caller.
fn add_days(d: NaiveDate, n: u64) -> NaiveDate {
    d.checked_add_days(Days::new(n)).unwrap_or(NaiveDate::MAX)
}

/// `d - n`, saturating at the earliest representable date.
fn sub_days(d: NaiveDate, n: u64) -> NaiveDate {
    d.checked_sub_days(Days::new(n)).unwrap_or(NaiveDate::MIN)
}

fn next_day(d: NaiveDate) -> NaiveDate {
    add_days(d, 1)
}

fn prev_day(d: NaiveDate) -> NaiveDate {
    sub_days(d, 1)
}

/// Build a date, clamping an out-of-range day to the last valid day of a month.
fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
    let m = m.clamp(1, 12);
    for day in (1..=d.clamp(1, 31)).rev() {
        if let Some(date) = NaiveDate::from_ymd_opt(y, m, day) {
            return date;
        }
    }
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
}

/// First of the month after `first` (which is itself a first-of-month).
fn add_month(first: NaiveDate) -> NaiveDate {
    let (y, m) = if first.month() == 12 {
        (first.year() + 1, 1)
    } else {
        (first.year(), first.month() + 1)
    };
    ymd(y, m, 1)
}

/// First of the month before `first` (which is itself a first-of-month).
fn sub_month(first: NaiveDate) -> NaiveDate {
    let (y, m) = if first.month() == 1 {
        (first.year() - 1, 12)
    } else {
        (first.year(), first.month() - 1)
    };
    ymd(y, m, 1)
}

fn start_epoch(date: NaiveDate) -> i64 {
    date.and_time(NaiveTime::MIN).and_utc().timestamp()
}

/// Today's date in UTC, the convenience answer to "what do I pass as
/// `today`?" for consumers resolving against the current day.
pub fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn year_precision_spans_the_year() {
        let (s, e) = resolve_range(&DateSpec::Ymd(2002, None, None), d(2026, 6, 20));
        assert_eq!(s, start_epoch(d(2002, 1, 1)));
        assert_eq!(e, start_epoch(d(2003, 1, 1)));
    }

    #[test]
    fn comparator_semantics() {
        let today = d(2026, 6, 20);
        let (s, e) = resolve_range(&DateSpec::Today, today);
        let noon = s + 12 * 3600;
        assert!(matches(Comparator::Eq, noon, s, e));
        assert!(!matches(Comparator::Gt, noon, s, e)); // not strictly after today
        assert!(matches(Comparator::Ge, noon, s, e));
        assert!(matches(Comparator::Gt, e + 1, s, e)); // tomorrow is after
        assert!(matches(Comparator::Lt, s - 1, s, e)); // yesterday is before
    }

    #[test]
    fn days_ago() {
        let today = d(2026, 6, 20);
        let (s, e) = resolve_range(&DateSpec::DaysAgo(3), today);
        assert_eq!(s, start_epoch(d(2026, 6, 17)));
        assert_eq!(e, start_epoch(d(2026, 6, 18)));
    }

    #[test]
    fn month_neighbors_and_in_days() {
        let today = d(2026, 6, 20);
        let (s, e) = resolve_range(&DateSpec::LastMonth, today);
        assert_eq!(s, start_epoch(d(2026, 5, 1)));
        assert_eq!(e, start_epoch(d(2026, 6, 1)));
        let (s, e) = resolve_range(&DateSpec::NextMonth, today);
        assert_eq!(s, start_epoch(d(2026, 7, 1)));
        assert_eq!(e, start_epoch(d(2026, 8, 1)));
        // Year boundaries in both directions.
        let (s, e) = resolve_range(&DateSpec::LastMonth, d(2026, 1, 15));
        assert_eq!(s, start_epoch(d(2025, 12, 1)));
        assert_eq!(e, start_epoch(d(2026, 1, 1)));
        let (s, e) = resolve_range(&DateSpec::NextMonth, d(2026, 12, 15));
        assert_eq!(s, start_epoch(d(2027, 1, 1)));
        assert_eq!(e, start_epoch(d(2027, 2, 1)));
        let (s, e) = resolve_range(&DateSpec::InDays(3), today);
        assert_eq!(s, start_epoch(d(2026, 6, 23)));
        assert_eq!(e, start_epoch(d(2026, 6, 24)));
    }

    #[test]
    fn extreme_offsets_saturate_instead_of_panicking() {
        let today = d(2026, 6, 20);
        // u32::MAX days cannot be stepped over in either direction: the range
        // clamps to the representable edge rather than aborting the caller.
        let (s, e) = resolve_range(&DateSpec::DaysAgo(u32::MAX), today);
        assert_eq!(s, start_epoch(NaiveDate::MIN));
        assert_eq!(e, start_epoch(add_days(NaiveDate::MIN, 1)));
        let (s, e) = resolve_range(&DateSpec::InDays(u32::MAX), today);
        assert_eq!(s, start_epoch(NaiveDate::MAX));
        assert_eq!(e, start_epoch(NaiveDate::MAX));
        // The week arms walk the same helpers, so they hold at the edges too.
        resolve_range(&DateSpec::LastWeek, NaiveDate::MIN);
        resolve_range(&DateSpec::ThisWeek, NaiveDate::MIN);
        resolve_range(&DateSpec::NextWeek, NaiveDate::MAX);
        resolve_range(&DateSpec::Tomorrow, NaiveDate::MAX);
        resolve_range(&DateSpec::Yesterday, NaiveDate::MIN);
    }
}
