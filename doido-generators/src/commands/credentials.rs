use clap::Subcommand;

#[derive(Subcommand)]
pub enum CredentialsCommand {
    /// Edit encrypted credentials
    Edit,
}

pub fn run(cmd: CredentialsCommand) {
    match cmd {
        CredentialsCommand::Edit => doido_core::tracing::info!("opening credentials editor..."),
    }
}
