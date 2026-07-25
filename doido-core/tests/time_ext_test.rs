use chrono::{Timelike, Utc};
use doido_core::time_ext::{beginning_of_day, days_ago, days_from_now, end_of_day};

#[test]
fn relative_times() {
    let now = Utc::now();
    assert!(days_ago(1) < now);
    assert!(days_from_now(1) > now);
}

#[test]
fn day_boundaries() {
    let now = Utc::now();
    let bod = beginning_of_day(now);
    assert_eq!((bod.hour(), bod.minute(), bod.second()), (0, 0, 0));
    let eod = end_of_day(now);
    assert_eq!((eod.hour(), eod.minute(), eod.second()), (23, 59, 59));
}
