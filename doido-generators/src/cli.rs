use crate::commands::{self, generate::run_generate, jobs::JobsCommand, new::run_new};
use clap::{Parser, Subcommand};
use doido_controller::axum;

#[derive(Parser)]
#[command(name = "doido", version = "0.1.0", about = "Doido framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
// Parsed once at startup, so the variant-size disparity from clap's embedded
// subcommands is irrelevant; boxing derived subcommand fields is fragile.
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Start the web server
    Server,
    /// Print routes
    Routes,
    /// Start interactive console
    Console,
    /// Database commands (create, SeaORM migrations and entity codegen)
    Db {
        /// Show debug messages
        #[arg(short, long, global = true)]
        verbose: bool,
        #[command(subcommand)]
        command: commands::db::DbCommand,
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
    /// Create a new Doido application
    New {
        /// Application name
        name: String,
        /// Database backend: sqlite, postgres, or mysql (prompted if omitted)
        #[arg(long)]
        database: Option<String>,
    },
}

/// Runs the Doido CLI.
///
/// `routes` carries the application's router. The `server` command starts the
/// HTTP server only when `routes` is `Some`; with `None` (e.g. the standalone
/// `doido-generators` binary) the server is not started.
pub async fn run(routes: Option<axum::Router>) {
    // Install project-specific inflection rules from `config/inflection.yaml`
    // (relative to the project root) before any generator pluralizes a name.
    // A missing file falls back to the default English rules.
    if let Err(e) = doido_core::load_inflections(doido_core::inflector::DEFAULT_CONFIG_PATH) {
        eprintln!("Warning: {e}");
    }

    // Seed DATABASE_URL from `config/<env>.yml` before clap parses, so the SeaORM
    // CLI under `doido db` (whose `generate entity` requires a database URL)
    // picks up the configured database without the user exporting it by hand.
    if std::env::args().nth(1).as_deref() == Some("db") {
        commands::db::ensure_database_url_from_config();
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Server => commands::server::run(routes).await,
        Commands::Routes => {
            // `routes` being `Some` means the app already built its router, which
            // populated the global route table the macro registers into.
            if routes.is_some() {
                doido_controller::print_routes();
            } else {
                println!("No routes configured.");
            }
        }
        Commands::Console => commands::console::run(),
        Commands::Worker => commands::worker::run(),
        Commands::Db { verbose, command } => commands::db::run(command, verbose).await,
        Commands::Jobs { action } => commands::jobs::run(action),
        Commands::Credentials { action } => commands::credentials::run(action),
        Commands::Generate { generator, args } => {
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_generate(&generator, &args_refs);
        }
        Commands::New { name, database } => {
            run_new(&name, database.as_deref());
        }
    }
}
