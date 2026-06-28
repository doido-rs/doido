//! `doido db` — database management.
//!
//! Exposes every SeaORM CLI subcommand and option verbatim (`doido db migrate
//! …`, `doido db generate entity …`) plus Doido's own `doido db create`, which
//! SeaORM does not provide. Doido changes two SeaORM defaults to match its app
//! layout:
//!   * migrations live in [`DEFAULT_MIGRATION_DIR`] (`db/migration`), and
//!   * generated entities are written to [`DEFAULT_ENTITY_OUTPUT_DIR`]
//!     (`app/models/_entities`).
//!
//! A user-supplied `-d/--migration-dir` or `-o/--output-dir` always wins.

use clap::Subcommand;
use sea_orm_cli::{
    handle_error, run_generate_command, run_migrate_command, Commands, GenerateSubcommands,
};

/// Subcommands of `doido db`: Doido's `create` plus the flattened SeaORM CLI.
#[derive(Subcommand)]
// The flattened SeaORM `Commands` is large, but this is parsed once at startup
// and can't be boxed through clap's `#[command(flatten)]`.
#[allow(clippy::large_enum_variant)]
pub enum DbCommand {
    /// Create the database for the current environment
    Create,
    /// SeaORM CLI commands (migrate, generate entity)
    #[command(flatten)]
    SeaOrm(Commands),
}

/// Where Doido keeps its SeaORM migration crate.
const DEFAULT_MIGRATION_DIR: &str = "db/migration";
/// Where Doido writes generated SeaORM entities.
const DEFAULT_ENTITY_OUTPUT_DIR: &str = "app/models/_entities";

/// Upstream SeaORM CLI defaults — used to detect "the user didn't override this".
const SEA_ORM_CLI_DEFAULT_MIGRATION_DIR: &str = "./migration";
const SEA_ORM_CLI_DEFAULT_OUTPUT_DIR: &str = "./";

/// Populates `DATABASE_URL` from the app's `config/<env>.yml` (`database.url`)
/// when it isn't already set in the environment.
///
/// SeaORM CLI reads the database URL from the `DATABASE_URL` env var (both
/// `migrate` and `generate entity` bind to it). Seeding it from config means
/// `doido db …` works without the user exporting `DATABASE_URL` by hand, while
/// an explicit `-u/--database-url` or a pre-set env var still wins. Call this
/// before clap parses so the required `generate entity` URL is satisfied.
pub fn ensure_database_url_from_config() {
    if std::env::var_os("DATABASE_URL").is_some() {
        return;
    }
    // Only seed from a real config file; absent config leaves DATABASE_URL unset
    // so the user gets the usual "missing database URL" error rather than a
    // surprising default.
    if let Ok(config) = doido_model::config::YamlConfig::load() {
        std::env::set_var("DATABASE_URL", config.database.url);
    }
}

/// Runs a `doido db <command>`.
pub async fn run(command: DbCommand, verbose: bool) {
    match command {
        DbCommand::Create => create().await,
        DbCommand::SeaOrm(command) => run_sea_orm(command, verbose).await,
    }
}

/// Creates the database named by the resolved [`database_url`].
async fn create() {
    let url = database_url();
    match doido_model::create_database(&url).await {
        Ok(()) => doido_core::tracing::info!("created database: {url}"),
        Err(e) if e.to_string().contains("already exists") => {
            doido_core::tracing::info!("database already exists: {url}");
        }
        Err(e) => handle_error(e),
    }
}

/// Resolves the database URL from `DATABASE_URL` or `config/<env>.yml`, exiting
/// with an error if neither is available.
fn database_url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return url;
    }
    if let Ok(config) = doido_model::config::YamlConfig::load() {
        return config.database.url;
    }
    doido_core::tracing::error!("DATABASE_URL is not set and config/<env>.yml could not be read");
    std::process::exit(1);
}

/// Dispatches a flattened SeaORM CLI command, applying Doido's directory defaults.
async fn run_sea_orm(command: Commands, verbose: bool) {
    match command {
        Commands::Generate { mut command } => {
            apply_entity_output_default(&mut command);
            run_generate_command(command, verbose)
                .await
                .unwrap_or_else(handle_error);
        }
        Commands::Migrate {
            migration_dir,
            database_schema,
            database_url,
            command,
        } => {
            let migration_dir = override_migration_dir(migration_dir);
            run_migrate_command(
                command,
                &migration_dir,
                database_schema,
                database_url,
                verbose,
            )
            .unwrap_or_else(handle_error);
        }
    }
}

/// Substitutes Doido's migration directory when the user left the SeaORM default.
fn override_migration_dir(migration_dir: String) -> String {
    if migration_dir == SEA_ORM_CLI_DEFAULT_MIGRATION_DIR {
        DEFAULT_MIGRATION_DIR.to_string()
    } else {
        migration_dir
    }
}

/// Substitutes Doido's entity output directory when the user left the SeaORM default.
fn apply_entity_output_default(command: &mut GenerateSubcommands) {
    let GenerateSubcommands::Entity { output_dir, .. } = command;
    if output_dir == SEA_ORM_CLI_DEFAULT_OUTPUT_DIR {
        *output_dir = DEFAULT_ENTITY_OUTPUT_DIR.to_string();
    }
}
