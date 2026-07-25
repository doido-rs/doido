use chrono::{Duration, TimeZone, Utc};
use doido_core::test_time::TestClock;

#[test]
fn travel_and_travel_to_control_the_clock() {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let clock = TestClock::new(start);
    assert_eq!(clock.now(), start);

    clock.travel(Duration::hours(2));
    assert_eq!(clock.now(), start + Duration::hours(2));

    let elsewhere = Utc.with_ymd_and_hms(2030, 6, 15, 0, 0, 0).unwrap();
    clock.travel_to(elsewhere);
    assert_eq!(clock.now(), elsewhere);
}
