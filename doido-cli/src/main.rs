mod commands;

use clap::{Parser, Subcommand};
use commands::{db::DbCommand, generate::run_generate, jobs::JobsCommand};

#[derive(Parser)]
#[command(name = "doido", version = "0.1.0", about = "Doido framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Server,
    /// Print routes
    Routes,
    /// Start interactive console
    Console,
    /// Database commands
    Db {
        #[command(subcommand)]
        action: DbCommand,
    },
    /// Background job commands
    Jobs {
        #[command(subcommand)]
        action: JobsCommand,
    },
    /// Start background worker
    Worker,
    /// Manage credentials
    Credentials {
        #[command(subcommand)]
        action: commands::credentials::CredentialsCommand,
    },
    /// Run a code generator
    Generate {
        /// Generator name (controller, model, migration, scaffold, job, mailer, channel)
        generator: String,
        /// Generator arguments
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Server => println!("Starting server on http://0.0.0.0:3000"),
        Commands::Routes => println!("Routes:"),
        Commands::Console => commands::console::run(),
        Commands::Worker => commands::worker::run(),
        Commands::Db { action } => commands::db::run(action),
        Commands::Jobs { action } => commands::jobs::run(action),
        Commands::Credentials { action } => commands::credentials::run(action),
        Commands::Generate { generator, args } => {
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_generate(&generator, &args_refs);
        }
    }
}
