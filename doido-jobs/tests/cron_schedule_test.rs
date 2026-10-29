use chrono::{Datelike, TimeZone, Timelike, Utc};
use cron::Schedule;
use doido_jobs::cron::CompiledCronTask;

#[test]
fn compiled_task_next_run_daily_at_three_utc() {
    let schedule: Schedule = "0 0 3 * * *".parse().expect("valid cron");
    let task = CompiledCronTask {
        job_name: "sync".into(),
        schedule,
        payload: serde_json::json!({}),
        queue: None,
    };
    let from = Utc.with_ymd_and_hms(2026, 4, 15, 2, 30, 0).unwrap();
    let next = task.next_run_after(from).expect("next run");
    assert_eq!(next.hour(), 3);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.day(), 15);
}
