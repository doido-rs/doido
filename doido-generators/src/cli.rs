use crate::commands::{self, jobs::JobsCommand, new::run_new};
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
    Worker {
        /// Drain the jobs currently ready, then exit (instead of running until Ctrl-C).
        #[arg(long)]
        once: bool,
    },
    /// Manage credentials
    Credentials {
        #[command(subcommand)]
        action: commands::credentials::CredentialsCommand,
    },
    /// Run a code generator (omit the name, or pass --help, to list generators)
    // `disable_help_flag` + `trailing_var_arg` let `--help` flow into `args` so
    // we can render the dynamic generator list instead of clap's static help.
    #[command(disable_help_flag = true)]
    Generate {
        /// Generator name followed by its arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
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
    // Greet on startup with the DOIDO banner (stderr, so stdout output like
    // route tables stays clean). The running mode is the first non-flag arg.
    let mode = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .unwrap_or_else(|| "server".to_string());
    crate::banner::print(&mode);

    // Install the global tracing subscriber first so every command logs through
    // the centralized logger. The fallback verbosity (when `RUST_LOG` is unset)
    // comes from the `logger` section of `config/<env>.yml`; a missing or invalid
    // config file falls back to the framework defaults.
    let app_config = doido_controller::config::YamlConfig::load().unwrap_or_default();
    doido_core::logger::init_with_config(&app_config.logger);

    // Install project-specific inflection rules from `config/inflection.yaml`
    // (relative to the project root) before any generator pluralizes a name.
    // A missing file falls back to the default English rules.
    if let Err(e) = doido_core::load_inflections(doido_core::inflector::DEFAULT_CONFIG_PATH) {
        doido_core::tracing::warn!("{e}");
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
                // The route table is this command's primary output — print it
                // directly to stdout rather than through the logger.
                doido_controller::print_routes();
            } else {
                doido_core::tracing::warn!("no routes configured");
            }
        }
        Commands::Console => commands::console::run(),
        Commands::Worker { once } => commands::worker::run(once).await,
        Commands::Db { verbose, command } => commands::db::run(command, verbose).await,
        Commands::Jobs { action } => commands::jobs::run(action),
        Commands::Credentials { action } => commands::credentials::run(action),
        Commands::Generate { args } => commands::generate::run(&args),
        Commands::New { name, database } => {
            run_new(&name, database.as_deref());
        }
    }
}
