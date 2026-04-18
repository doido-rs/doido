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
        JobsCommand::Failed => println!("Failed jobs: (none)"),
        JobsCommand::Retry => println!("Retrying failed jobs..."),
        JobsCommand::Discard => println!("Discarding failed jobs..."),
    }
}
