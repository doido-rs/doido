//! Cursor (token) pagination for job list APIs — stable sort, no offset.

use crate::queue::JobPayload;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Sort key matching `list_jobs` ordering: priority DESC, run_at ASC, id ASC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSortKey {
    pub priority: i32,
    pub run_at_ms: i64,
    pub id: String,
}

impl JobSortKey {
    pub fn from_job(job: &JobPayload) -> Self {
        Self {
            priority: job.priority,
            run_at_ms: job.run_at.timestamp_millis(),
            id: job.id.clone(),
        }
    }
}

/// Compare two jobs (or a job and key) for dashboard list ordering.
pub fn job_sort_cmp(a: &JobPayload, b: &JobPayload) -> std::cmp::Ordering {
    job_key_cmp(
        a.priority,
        a.run_at.timestamp_millis(),
        &a.id,
        b.priority,
        b.run_at.timestamp_millis(),
        &b.id,
    )
}

fn job_key_cmp(ap: i32, ar: i64, aid: &str, bp: i32, br: i64, bid: &str) -> std::cmp::Ordering {
    bp.cmp(&ap)
        .then_with(|| ar.cmp(&br))
        .then_with(|| aid.cmp(bid))
}

pub fn sort_jobs(jobs: &mut [JobPayload]) {
    jobs.sort_by(job_sort_cmp);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobPageDirection {
    After,
    Before,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobPageAnchor {
    After(JobSortKey),
    Before(JobSortKey),
}

#[derive(Clone, Debug, Default)]
pub struct JobListPage {
    pub jobs: Vec<JobPayload>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct JobTokenPayload {
    d: char,
    p: i32,
    r: i64,
    i: String,
}

pub fn encode_job_page_token(key: &JobSortKey, direction: JobPageDirection) -> String {
    let d = match direction {
        JobPageDirection::After => 'a',
        JobPageDirection::Before => 'b',
    };
    let raw = serde_json::to_vec(&JobTokenPayload {
        d,
        p: key.priority,
        r: key.run_at_ms,
        i: key.id.clone(),
    })
    .unwrap_or_default();
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn decode_job_page_token(token: &str) -> Option<JobPageAnchor> {
    let bytes = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    let payload: JobTokenPayload = serde_json::from_slice(&bytes).ok()?;
    let key = JobSortKey {
        priority: payload.p,
        run_at_ms: payload.r,
        id: payload.i,
    };
    match payload.d {
        'a' => Some(JobPageAnchor::After(key)),
        'b' => Some(JobPageAnchor::Before(key)),
        _ => None,
    }
}

/// Paginate an in-memory list already filtered to a bucket (memory/redis backends).
pub fn paginate_sorted(
    mut jobs: Vec<JobPayload>,
    anchor: Option<JobPageAnchor>,
    limit: u32,
) -> JobListPage {
    sort_jobs(&mut jobs);
    let limit = limit.max(1) as usize;

    match anchor {
        Some(JobPageAnchor::Before(key)) => {
            let end = jobs.partition_point(|j| {
                job_key_cmp(
                    j.priority,
                    j.run_at.timestamp_millis(),
                    &j.id,
                    key.priority,
                    key.run_at_ms,
                    &key.id,
                ) == std::cmp::Ordering::Less
            });
            if end == 0 {
                JobListPage::default()
            } else {
                let prefix = &jobs[..end];
                let suffix_start = prefix.len().saturating_sub(limit);
                let page: Vec<JobPayload> = prefix[suffix_start..].to_vec();
                let has_next = end < jobs.len();
                let has_prev = suffix_start > 0;
                build_job_list_page(page, has_prev, has_next)
            }
        }
        anchor => {
            let start = match anchor {
                Some(JobPageAnchor::After(key)) => jobs.partition_point(|j| {
                    job_key_cmp(
                        j.priority,
                        j.run_at.timestamp_millis(),
                        &j.id,
                        key.priority,
                        key.run_at_ms,
                        &key.id,
                    ) != std::cmp::Ordering::Greater
                }),
                None => 0,
                _ => unreachable!(),
            };
            let mut page: Vec<JobPayload> =
                jobs.iter().skip(start).take(limit + 1).cloned().collect();
            let has_next = page.len() > limit;
            if has_next {
                page.pop();
            }
            let has_prev = start > 0;
            build_job_list_page(page, has_prev, has_next)
        }
    }
}

fn build_job_list_page(page: Vec<JobPayload>, has_prev: bool, has_next: bool) -> JobListPage {
    let next_page_token = if has_next {
        page.last()
            .map(|j| encode_job_page_token(&JobSortKey::from_job(j), JobPageDirection::After))
    } else {
        None
    };
    let prev_page_token = if has_prev {
        page.first()
            .map(|j| encode_job_page_token(&JobSortKey::from_job(j), JobPageDirection::Before))
    } else {
        None
    };
    JobListPage {
        jobs: page,
        next_page_token,
        prev_page_token,
    }
}

/// Token pagination for the **Queues** tab (config order, keyed by queue name).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueueListPage<T> {
    pub rows: Vec<T>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct QueueTokenPayload {
    d: char,
    q: String,
}

pub fn encode_queue_page_token(after_queue: &str) -> String {
    let raw = serde_json::to_vec(&QueueTokenPayload {
        d: 'a',
        q: after_queue.to_string(),
    })
    .unwrap_or_default();
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn encode_queue_page_token_before(before_queue: &str) -> String {
    let raw = serde_json::to_vec(&QueueTokenPayload {
        d: 'b',
        q: before_queue.to_string(),
    })
    .unwrap_or_default();
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn decode_queue_page_token(token: &str) -> Option<QueuePageAnchor> {
    let bytes = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    let payload: QueueTokenPayload = serde_json::from_slice(&bytes).ok()?;
    match payload.d {
        'a' => Some(QueuePageAnchor::After(payload.q)),
        'b' => Some(QueuePageAnchor::Before(payload.q)),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuePageAnchor {
    After(String),
    Before(String),
}

pub fn paginate_queue_names<T, F>(
    rows: Vec<T>,
    name: F,
    anchor: Option<QueuePageAnchor>,
    limit: usize,
) -> QueueListPage<T>
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    let limit = limit.max(1);
    let len = rows.len();
    let (start, end) = match anchor {
        None => (0, len),
        Some(QueuePageAnchor::After(ref q)) => {
            let start = rows
                .iter()
                .position(|r| name(r) == q)
                .map(|i| i + 1)
                .unwrap_or(0);
            (start, len)
        }
        Some(QueuePageAnchor::Before(ref q)) => {
            let end = rows.iter().position(|r| name(r) == q).unwrap_or(0);
            let start = end.saturating_sub(limit + 1);
            (start, end)
        }
    };

    let window = &rows[start..end.min(len)];
    let mut page: Vec<T> = window.iter().take(limit + 1).cloned().collect();
    let has_next = match anchor {
        Some(QueuePageAnchor::Before(_)) => end < len,
        _ => page.len() > limit,
    };
    if page.len() > limit {
        page.pop();
    }
    let has_prev = match anchor {
        Some(QueuePageAnchor::Before(_)) => start > 0,
        _ => start > 0 || anchor.is_some(),
    };

    let next_page_token = if has_next {
        page.last().map(|r| encode_queue_page_token(name(r)))
    } else {
        None
    };
    let prev_page_token = if has_prev {
        page.first()
            .map(|r| encode_queue_page_token_before(name(r)))
    } else {
        None
    };

    QueueListPage {
        rows: page,
        next_page_token,
        prev_page_token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::JobPayload;
    use serde_json::json;

    fn job(id: &str, priority: i32, run_at_ms: i64) -> JobPayload {
        let mut j = JobPayload::new("default", json!({}), 3);
        j.id = id.into();
        j.priority = priority;
        j.run_at = chrono::DateTime::from_timestamp_millis(run_at_ms).unwrap();
        j
    }

    #[test]
    fn forward_pages_use_cursor_not_offset() {
        let jobs: Vec<_> = (0..5)
            .map(|i| job(&format!("id-{i}"), 0, i as i64))
            .collect();
        let p1 = paginate_sorted(jobs.clone(), None, 2);
        assert_eq!(p1.jobs.len(), 2);
        assert!(p1.next_page_token.is_some());
        assert!(p1.prev_page_token.is_none());

        let anchor = decode_job_page_token(p1.next_page_token.as_ref().unwrap()).unwrap();
        let p2 = paginate_sorted(jobs, Some(anchor), 2);
        assert_eq!(p2.jobs.len(), 2);
        assert!(p2.prev_page_token.is_some());
    }

    #[test]
    fn roundtrip_job_token() {
        let key = JobSortKey {
            priority: 1,
            run_at_ms: 99,
            id: "abc".into(),
        };
        let t = encode_job_page_token(&key, JobPageDirection::After);
        match decode_job_page_token(&t).unwrap() {
            JobPageAnchor::After(k) => assert_eq!(k, key),
            _ => panic!("expected after"),
        }
        let t = encode_job_page_token(&key, JobPageDirection::Before);
        match decode_job_page_token(&t).unwrap() {
            JobPageAnchor::Before(k) => assert_eq!(k, key),
            _ => panic!("expected before"),
        }
    }

    #[test]
    fn backward_page_uses_before_token() {
        let jobs: Vec<_> = (0..5)
            .map(|i| job(&format!("id-{i}"), 0, i as i64))
            .collect();
        let p2 = paginate_sorted(jobs.clone(), None, 2);
        let forward = decode_job_page_token(p2.next_page_token.as_ref().unwrap()).unwrap();
        let p3 = paginate_sorted(jobs.clone(), Some(forward), 2);
        let before = decode_job_page_token(p3.prev_page_token.as_ref().unwrap()).unwrap();
        let back = paginate_sorted(jobs, Some(before), 2);
        assert_eq!(back.jobs.len(), 2);
        assert_eq!(back.jobs[0].id, "id-0");
    }

    #[test]
    fn paginate_queue_names_forward_and_back() {
        let rows: Vec<(String, u32)> = (0..5).map(|i| (format!("q{i}"), i)).collect();
        let p1 = paginate_queue_names(rows.clone(), |(n, _)| n.as_str(), None, 2);
        assert_eq!(p1.rows.len(), 2);
        assert!(p1.next_page_token.is_some());
        let after = decode_queue_page_token(p1.next_page_token.as_ref().unwrap()).unwrap();
        let p2 = paginate_queue_names(rows.clone(), |(n, _)| n.as_str(), Some(after), 2);
        assert_eq!(p2.rows[0].0, "q2");
        let before = decode_queue_page_token(p2.prev_page_token.as_ref().unwrap()).unwrap();
        let back = paginate_queue_names(rows, |(n, _)| n.as_str(), Some(before), 2);
        assert_eq!(back.rows[0].0, "q0");
    }
}
