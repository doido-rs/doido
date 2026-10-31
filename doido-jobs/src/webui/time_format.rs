//! Human-readable timestamps for the jobs Web UI.

use chrono::{DateTime, Utc};

/// Tooltip format: `dd/mm/YYYY - HH:MM:ss`.
pub fn format_tooltip(dt: DateTime<Utc>) -> String {
    dt.format("%d/%m/%Y - %H:%M:%S").to_string()
}

fn ago(unit: &str, n: i64) -> String {
    if n == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{n} {unit}s ago")
    }
}

fn in_future(unit: &str, n: i64) -> String {
    if n == 1 {
        format!("in 1 {unit}")
    } else {
        format!("in {n} {unit}s")
    }
}

/// Relative label such as `5 minutes ago` or `in 2 hours` (compared to `now`).
pub fn format_relative(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (now - dt).num_seconds();
    if secs == 0 {
        return "just now".to_string();
    }
    if secs > 0 {
        relative_past(secs)
    } else {
        relative_future(-secs)
    }
}

fn relative_past(secs: i64) -> String {
    if secs < 60 {
        return ago("second", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return ago("minute", mins);
    }
    let hours = mins / 60;
    if hours < 24 {
        return ago("hour", hours);
    }
    let days = hours / 24;
    ago("day", days)
}

fn relative_future(secs: i64) -> String {
    if secs < 60 {
        return in_future("second", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return in_future("minute", mins);
    }
    let hours = mins / 60;
    if hours < 24 {
        return in_future("hour", hours);
    }
    let days = hours / 24;
    in_future("day", days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn tooltip_uses_day_month_year_and_clock() {
        let dt = Utc.with_ymd_and_hms(2026, 3, 5, 14, 7, 9).unwrap();
        assert_eq!(format_tooltip(dt), "05/03/2026 - 14:07:09");
    }

    #[test]
    fn relative_seconds_minutes_hours() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        assert_eq!(
            format_relative(now - chrono::Duration::seconds(5), now),
            "5 seconds ago"
        );
        assert_eq!(
            format_relative(now - chrono::Duration::minutes(5), now),
            "5 minutes ago"
        );
        assert_eq!(
            format_relative(now - chrono::Duration::hours(5), now),
            "5 hours ago"
        );
        assert_eq!(
            format_relative(now + chrono::Duration::minutes(3), now),
            "in 3 minutes"
        );
    }
}
