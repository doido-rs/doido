//! Job lifecycle callbacks: Rails `before_perform`/`after_perform`/
//! `around_perform` and failure hooks. A job implements [`JobCallbacks`];
//! [`run_perform`] fires the hooks around the work, and `on_failure` sees any
//! error.

use doido_core::{anyhow, Result};

/// Hooks fired around a job's `perform`. Override the ones you need.
pub trait JobCallbacks {
    fn before_perform(&mut self) -> Result<()> {
        Ok(())
    }
    fn after_perform(&mut self) -> Result<()> {
        Ok(())
    }
    fn on_failure(&mut self, _error: &anyhow::Error) {}
}

/// Run `perform` with the callback lifecycle: `before_perform` (may abort) →
/// perform → `after_perform` on success, or `on_failure` on error.
pub async fn run_perform<J: JobCallbacks>(
    job: &mut J,
    perform: impl AsyncFnOnce(&mut J) -> Result<()>,
) -> Result<()> {
    job.before_perform()?;
    match perform(job).await {
        Ok(()) => {
            job.after_perform()?;
            Ok(())
        }
        Err(error) => {
            job.on_failure(&error);
            Err(error)
        }
    }
}
