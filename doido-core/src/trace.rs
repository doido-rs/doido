/// Emit a structured event for an HTTP request.
///
/// `doido-controller`'s `logging::log_requests` middleware emits richer,
/// header-aware request/response events directly; this stays as a lightweight
/// helper for code that just wants a one-line structured request event.
pub fn request(method: &str, path: &str, status: u16, latency_ms: u64) {
    tracing::info!(
        method = method,
        path = path,
        status = status,
        latency_ms = latency_ms,
        "request"
    );
}

/// Emit a structured event for a background job execution
pub fn job(job_name: &str, queue: &str, attempt: u32, result: &str) {
    tracing::info!(
        job_name = job_name,
        queue = queue,
        attempt = attempt,
        result = result,
        "job"
    );
}

/// Emit a structured event for a database query
pub fn query(sql: &str, duration_ms: u64) {
    tracing::info!(sql = sql, duration_ms = duration_ms, "query");
}

/// Emit a structured event for email delivery
pub fn mail(to: &str, subject: &str, deliverer: &str) {
    tracing::info!(to = to, subject = subject, deliverer = deliverer, "mail");
}
