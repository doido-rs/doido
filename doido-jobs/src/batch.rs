//! Job batches (Sidekiq/Rails-style): track a set of jobs and know when they
//! have all finished, so a completion step can run.

use std::collections::HashSet;

/// A batch of jobs tracked by id.
#[derive(Debug, Default)]
pub struct Batch {
    pending: HashSet<String>,
    total: usize,
}

impl Batch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a job as a member of the batch.
    pub fn add(&mut self, job_id: &str) -> &mut Self {
        if self.pending.insert(job_id.to_string()) {
            self.total += 1;
        }
        self
    }

    /// Mark a member finished.
    pub fn complete(&mut self, job_id: &str) {
        self.pending.remove(job_id);
    }

    /// Whether every member has finished.
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.pending.is_empty()
    }

    /// `(completed, total)` progress.
    pub fn progress(&self) -> (usize, usize) {
        (self.total - self.pending.len(), self.total)
    }
}
