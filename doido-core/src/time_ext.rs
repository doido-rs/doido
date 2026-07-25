//! Time/date conveniences (Rails `2.days.ago`, `beginning_of_day`, …).

use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};

/// `n` days before now (Rails `n.days.ago`).
pub fn days_ago(n: i64) -> DateTime<Utc> {
    Utc::now() - Duration::days(n)
}

/// `n` days after now (Rails `n.days.from_now`).
pub fn days_from_now(n: i64) -> DateTime<Utc> {
    Utc::now() + Duration::days(n)
}

/// `n` hours before now.
pub fn hours_ago(n: i64) -> DateTime<Utc> {
    Utc::now() - Duration::hours(n)
}

/// Midnight at the start of `dt`'s day (Rails `beginning_of_day`).
pub fn beginning_of_day<Tz: TimeZone>(dt: DateTime<Tz>) -> DateTime<Tz> {
    dt.with_hour(0)
        .and_then(|d| d.with_minute(0))
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}

/// The last second of `dt`'s day (Rails `end_of_day`).
pub fn end_of_day<Tz: TimeZone>(dt: DateTime<Tz>) -> DateTime<Tz> {
    dt.with_hour(23)
        .and_then(|d| d.with_minute(59))
        .and_then(|d| d.with_second(59))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}
