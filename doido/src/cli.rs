use crate::banner;
use clap::{Parser, Subcommand};
use doido_controller::axum;
use doido_core::commands::credentials::CredentialsCommand;
use doido_generators::commands as generator_commands;
use doido_generators::new_options::{CacheBackend, DatabaseBackend, JobsBackend, NewOptions};
use doido_jobs::commands::jobs::JobsCommand;
use doido_model::commands::db::DbCommand;

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
    /// Scaffold a new Doido extension crate (e.g. Payments → doido-payments)
    Extension {
        /// Extension name
        name: String,
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
///
/// To also install app-owned code generators into `doido generate`, use the
/// [`Doido`] builder instead.
pub async fn run(routes: Option<axum::Router>) {
    run_inner(routes, Vec::new()).await;
}

/// Builder for the Doido CLI. Lets an application install custom code
/// generators — any type implementing [`doido_generators::Generator`] — into
/// `doido generate`, alongside the framework built-ins, before handing control
/// to the CLI. The app binary is compiled with the app's own dependencies, so
/// generators defined in the app (or in a crate it depends on) become reachable
/// via `cargo doido generate <name>`.
///
/// ```ignore
/// #[tokio::main]
/// async fn main() {
///     doido::Doido::new()
///         .router(routes::router())
///         .register_generator(Box::new(MyGenerator))
///         .run()
///         .await;
/// }
/// ```
pub struct Doido {
    router: Option<axum::Router>,
    generators: Vec<Box<dyn doido_generators::Generator>>,
}

impl Doido {
    /// Start a CLI builder with no router and no custom generators.
    pub fn new() -> Self {
        Self {
            router: None,
            generators: Vec::new(),
        }
    }

    /// Attach the application's router so `doido server` can boot the HTTP server.
    pub fn router(mut self, router: axum::Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Replace the installed generators with `generators`.
    pub fn generators(mut self, generators: Vec<Box<dyn doido_generators::Generator>>) -> Self {
        self.generators = generators;
        self
    }

    /// Install one custom generator into `doido generate`.
    pub fn register_generator(mut self, generator: Box<dyn doido_generators::Generator>) -> Self {
        self.generators.push(generator);
        self
    }

    /// Run the CLI: like [`run`], plus any generators installed on this builder.
    pub async fn run(self) {
        run_inner(self.router, self.generators).await;
    }
}

impl Default for Doido {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_inner(
    routes: Option<axum::Router>,
    generators: Vec<Box<dyn doido_generators::Generator>>,
) {
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
        Commands::Db { verbose, command } => doido_model::commands::db::run(command, verbose).await,
        Commands::Jobs { action } => doido_jobs::commands::jobs::run(action).await,
        Commands::Credentials { action } => doido_core::commands::credentials::run(action),
        Commands::Generate { args } => generator_commands::generate::run_with(&args, generators),
        Commands::Extension { name } => generator_commands::extension::run_extension(&name),
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

#[cfg(test)]
mod tests {
    use super::*;
    use doido_generators::{GeneratedFile, Generator};

    struct DummyGenerator;
    impl Generator for DummyGenerator {
        fn name(&self) -> &str {
            "dummy"
        }
        fn generate(&self, _args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn builder_collects_router_and_registered_generators() {
        let doido = Doido::new()
            .router(axum::Router::new())
            .register_generator(Box::new(DummyGenerator))
            .register_generator(Box::new(DummyGenerator));
        assert!(doido.router.is_some());
        assert_eq!(doido.generators.len(), 2);
    }

    #[test]
    fn generators_replaces_the_installed_list_and_default_is_empty() {
        let doido = Doido::default().generators(vec![Box::new(DummyGenerator)]);
        assert!(doido.router.is_none());
        assert_eq!(doido.generators.len(), 1);
    }
}
