use crate::banner;
use clap::{Parser, Subcommand};
use doido_controller::axum;
use doido_generators::new_options::{CacheBackend, DatabaseBackend, JobsBackend, NewOptions};
use doido_generators::{commands as generator_commands};
use doido_jobs::commands::jobs::JobsCommand;
use doido_model::commands::db::DbCommand;
use doido_core::commands::credentials::CredentialsCommand;

#[derive(Parser)]
#[command(name = "doido", version = env!("CARGO_PKG_VERSION"), about = "Doido framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Start the web server
    Server {
        /// Port to bind (overrides `server.port` in config/<env>.yml)
        #[arg(long)]
        port: Option<u16>,
        /// Environment for this run: development | test | production (sets DOIDO_ENV)
        #[arg(long)]
        env: Option<String>,
    },
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
        command: DbCommand,
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
        action: CredentialsCommand,
    },
    /// Run a code generator (omit the name, or pass --help, to list generators)
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
        /// Skip interactive prompts; use flag values or defaults
        #[arg(long)]
        non_interactive: bool,
        /// Database backend (prompted when omitted in interactive mode)
        #[arg(long, value_enum)]
        database: Option<DatabaseBackend>,
        /// Include a doido-cable example channel and its wiring
        #[arg(long)]
        cable: bool,
        /// Add doido-auth and run auth:install
        #[arg(long)]
        auth: bool,
        /// API-only auth (JSON endpoints; omit for HTML sign-in/sign-up views)
        #[arg(long)]
        api: bool,
        /// Cache backend (prompted when omitted in interactive mode)
        #[arg(long, value_enum)]
        cache: Option<CacheBackend>,
        /// Jobs backend (prompted when omitted in interactive mode)
        #[arg(long, value_enum)]
        jobs: Option<JobsBackend>,
    },
}

/// Runs the Doido CLI.
///
/// `routes` carries the application's router. The `server` command starts the
/// HTTP server only when `routes` is `Some`; with `None` (e.g. the standalone
/// `doido` binary without an app router) the server is not started.
pub async fn run(routes: Option<axum::Router>) {
    let mode = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .unwrap_or_else(|| "server".to_string());
    banner::print(&mode);

    let app_config = doido_controller::config::YamlConfig::load().unwrap_or_default();
    doido_core::logger::init_with_config(&app_config.logger);

    if let Err(e) = doido_core::load_inflections(doido_core::inflector::DEFAULT_CONFIG_PATH) {
        doido_core::tracing::warn!("{e}");
    }

    if std::env::args().nth(1).as_deref() == Some("db") {
        doido_model::commands::db::ensure_database_url_from_config();
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Server { port, env } => {
            crate::server::run(routes, env, port).await;
        }
        Commands::Routes => {
            if routes.is_some() {
                doido_controller::print_routes();
            } else {
                doido_core::tracing::warn!("no routes configured");
            }
        }
        Commands::Console => doido_controller::commands::console::run(),
        Commands::Worker { once } => doido_jobs::commands::worker::run(once).await,
        Commands::Db { verbose, command } => {
            doido_model::commands::db::run(command, verbose).await
        }
        Commands::Jobs { action } => doido_jobs::commands::jobs::run(action).await,
        Commands::Credentials { action } => doido_core::commands::credentials::run(action),
        Commands::Generate { args } => generator_commands::generate::run(&args),
        Commands::New {
            name,
            non_interactive,
            database,
            cable,
            auth,
            api,
            cache,
            jobs,
        } => {
            generator_commands::new::run_new(
                &name,
                NewOptions {
                    non_interactive,
                    database,
                    cable,
                    auth,
                    api,
                    cache,
                    jobs,
                },
            );
        }
    }
}
