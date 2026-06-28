use clap::Subcommand;

#[derive(Subcommand)]
pub enum JobsCommand {
    /// List failed jobs
    Failed,
    /// Retry failed jobs
    Retry,
    /// Discard failed jobs
    Discard,
}

pub fn run(cmd: JobsCommand) {
    match cmd {
        JobsCommand::Failed => doido_core::tracing::info!("failed jobs: (none)"),
        JobsCommand::Retry => doido_core::tracing::info!("retrying failed jobs..."),
        JobsCommand::Discard => doido_core::tracing::info!("discarding failed jobs..."),
    }
}
