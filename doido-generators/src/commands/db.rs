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
//! After every schema-changing migrate (`up`, `down`, `fresh`, `refresh`, `reset`),
//! Doido re-exports entities from the database into `_entities/` and ensures
//! extension stubs exist under `app/models/<name>.rs`.
//!
//! A user-supplied `-d/--migration-dir` or `-o/--output-dir` always wins.

use clap::Subcommand;
use doido_model::sea_orm_cli::{
    handle_error, run_generate_command, run_migrate_command, BannerVersion, BigIntegerType,
    Commands, DateTimeCrate, GenerateSubcommands, MigrateSubcommands,
};
use std::path::Path;

/// Subcommands of `doido db`: Doido's `create` plus the flattened SeaORM CLI.
#[derive(Subcommand)]
// The flattened SeaORM `Commands` is large, but this is parsed once at startup
// and can't be boxed through clap's `#[command(flatten)]`.
#[allow(clippy::large_enum_variant)]
pub enum DbCommand {
    /// Create the database for the current environment
    Create,
    /// Drop every table and reload `db/schema.sql`
    Reset,
    /// Load `db/schema.sql` only if the database has no tables yet (idempotent)
    Prepare,
    /// Run the `db/seed` crate (Rust models-based seeder)
    Seed,
    /// Schema dump/load (`db/schema.sql`)
    Schema {
        #[command(subcommand)]
        action: SchemaCommand,
    },
    /// SeaORM CLI commands (migrate, generate entity)
    #[command(flatten)]
    SeaOrm(Commands),
}

/// Subcommands of `doido db schema`.
#[derive(Subcommand)]
pub enum SchemaCommand {
    /// Dump the current schema to `db/schema.sql`
    Dump,
    /// Load `db/schema.sql` into the database
    Load,
}

/// Where Doido keeps its SeaORM migration crate.
const DEFAULT_MIGRATION_DIR: &str = "db/migration";
/// Where Doido keeps its Rust seed runner crate.
const DEFAULT_SEED_DIR: &str = "db/seed";
/// Where Doido writes generated SeaORM entities.
const DEFAULT_ENTITY_OUTPUT_DIR: &str = "app/models/_entities";
/// Canonical schema file (Rails `db/schema.rb` analogue).
const SCHEMA_FILE: &str = "db/schema.sql";
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
        DbCommand::Reset => reset().await,
        DbCommand::Prepare => prepare().await,
        DbCommand::Seed => seed().await,
        DbCommand::Schema { action } => schema(action).await,
        DbCommand::SeaOrm(command) => run_sea_orm(command, verbose).await,
    }
}

/// Opens a connection to the resolved [`database_url`], exiting on failure.
async fn connect() -> doido_model::DatabaseConnection {
    let url = database_url();
    match doido_model::connect_with_url(&url).await {
        Ok(conn) => conn,
        Err(e) => {
            doido_core::tracing::error!("failed to connect to {url}: {e}");
            std::process::exit(1);
        }
    }
}

/// Reads a file, logging (and returning `None`) on failure.
fn read_sql_file(path: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Some(contents),
        Err(e) => {
            doido_core::tracing::error!("could not read {path}: {e}");
            None
        }
    }
}

/// `doido db reset` — drop everything, then reload `db/schema.sql`.
async fn reset() {
    let Some(schema) = read_sql_file(SCHEMA_FILE) else {
        return;
    };
    let conn = connect().await;
    match doido_model::tasks::reset(&conn, &schema).await {
        Ok(()) => doido_core::tracing::info!("reset database from {SCHEMA_FILE}"),
        Err(e) => doido_core::tracing::error!("db reset failed: {e}"),
    }
}

/// `doido db prepare` — load `db/schema.sql` only if the database is empty.
async fn prepare() {
    let Some(schema) = read_sql_file(SCHEMA_FILE) else {
        return;
    };
    let conn = connect().await;
    match doido_model::tasks::prepare(&conn, &schema).await {
        Ok(()) => doido_core::tracing::info!("prepared database from {SCHEMA_FILE}"),
        Err(e) => doido_core::tracing::error!("db prepare failed: {e}"),
    }
}

/// Program + args to run the seed crate (`db/seed`).
pub fn seed_command() -> (String, Vec<String>) {
    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            format!("{DEFAULT_SEED_DIR}/Cargo.toml"),
        ],
    )
}

/// `doido db seed` — compile and run the `db/seed` crate, which inserts data
/// using the app's SeaORM models in `app/models/`.
async fn seed() {
    let (program, args) = seed_command();
    match std::process::Command::new(&program).args(&args).status() {
        Ok(status) if status.success() => {
            doido_core::tracing::info!("seeded database via {DEFAULT_SEED_DIR}");
        }
        Ok(status) => {
            doido_core::tracing::error!(
                "db seed failed: cargo exited with {}",
                status.code().unwrap_or(-1)
            );
        }
        Err(e) => doido_core::tracing::error!("db seed failed: {e}"),
    }
}

/// `doido db schema dump|load` over [`SCHEMA_FILE`].
async fn schema(action: SchemaCommand) {
    let conn = connect().await;
    match action {
        SchemaCommand::Dump => match doido_model::schema::dump(&conn).await {
            Ok(sql) => {
                if let Some(parent) = std::path::Path::new(SCHEMA_FILE).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(SCHEMA_FILE, sql) {
                    Ok(()) => doido_core::tracing::info!("wrote schema to {SCHEMA_FILE}"),
                    Err(e) => doido_core::tracing::error!("could not write {SCHEMA_FILE}: {e}"),
                }
            }
            Err(e) => doido_core::tracing::error!("schema dump failed: {e}"),
        },
        SchemaCommand::Load => {
            let Some(sql) = read_sql_file(SCHEMA_FILE) else {
                return;
            };
            match doido_model::schema::load(&conn, &sql).await {
                Ok(()) => doido_core::tracing::info!("loaded schema from {SCHEMA_FILE}"),
                Err(e) => doido_core::tracing::error!("schema load failed: {e}"),
            }
        }
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
            let is_entity = matches!(&command, GenerateSubcommands::Entity { .. });
            run_generate_command(command, verbose)
                .await
                .unwrap_or_else(handle_error);
            if is_entity {
                sync_model_extensions();
            }
        }
        Commands::Migrate {
            migration_dir,
            database_schema,
            database_url,
            command,
        } => {
            let migration_dir = override_migration_dir(migration_dir);
            let export = should_export_entities(command.as_ref());
            run_migrate_command(
                command,
                &migration_dir,
                database_schema,
                database_url,
                verbose,
            )
            .unwrap_or_else(handle_error);
            if export {
                export_entities_from_database(verbose).await;
            }
        }
    }
}

/// Whether a migrate subcommand changes the schema enough to warrant re-export.
fn should_export_entities(command: Option<&MigrateSubcommands>) -> bool {
    matches!(
        command,
        None | Some(MigrateSubcommands::Up { .. })
            | Some(MigrateSubcommands::Down { .. })
            | Some(MigrateSubcommands::Fresh)
            | Some(MigrateSubcommands::Refresh)
            | Some(MigrateSubcommands::Reset)
    )
}

/// Re-export entities from the live database into [`DEFAULT_ENTITY_OUTPUT_DIR`].
async fn export_entities_from_database(verbose: bool) {
    ensure_database_url_from_config();
    let mut command = default_entity_generate_command(database_url());
    apply_entity_output_default(&mut command);
    if let Err(e) = run_generate_command(command, verbose).await {
        handle_error(e);
    }
    sync_model_extensions();
}

fn sync_model_extensions() {
    let entities_dir = Path::new(DEFAULT_ENTITY_OUTPUT_DIR);
    let models_dir = Path::new("app/models");
    match doido_model::entities::postprocess_entity_export(entities_dir, models_dir) {
        Ok(()) => doido_core::tracing::info!("post-processed exported entities"),
        Err(e) => doido_core::tracing::error!("entity post-process failed: {e}"),
    }
}

fn default_entity_generate_command(database_url: String) -> GenerateSubcommands {
    GenerateSubcommands::Entity {
        entity_format: None,
        compact_format: false,
        expanded_format: false,
        frontend_format: false,
        include_hidden_tables: false,
        tables: Vec::new(),
        ignore_tables: vec!["seaql_migrations".to_string()],
        max_connections: 1,
        acquire_timeout: 30,
        output_dir: SEA_ORM_CLI_DEFAULT_OUTPUT_DIR.to_string(),
        database_schema: None,
        database_url,
        with_prelude: "all".to_string(),
        with_serde: "both".to_string(),
        serde_skip_deserializing_primary_key: false,
        serde_skip_hidden_column: false,
        with_copy_enums: false,
        date_time_crate: DateTimeCrate::Chrono,
        big_integer_type: BigIntegerType::I64,
        lib: false,
        model_extra_derives: Vec::new(),
        model_extra_attributes: Vec::new(),
        enum_extra_derives: Vec::new(),
        enum_extra_attributes: Vec::new(),
        column_extra_derives: Vec::new(),
        seaography: false,
        impl_active_model_behavior: true,
        preserve_user_modifications: false,
        banner_version: BannerVersion::Minor,
        er_diagram: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_changing_migrate_commands_export_entities() {
        assert!(should_export_entities(None));
        assert!(should_export_entities(Some(&MigrateSubcommands::Up {
            num: None
        })));
        assert!(should_export_entities(Some(&MigrateSubcommands::Down {
            num: 1
        })));
        assert!(should_export_entities(Some(&MigrateSubcommands::Fresh)));
        assert!(!should_export_entities(Some(&MigrateSubcommands::Status)));
        assert!(!should_export_entities(Some(&MigrateSubcommands::Init)));
    }

    #[test]
    fn apply_entity_output_default_rewrites_sea_orm_default() {
        let mut command = default_entity_generate_command("sqlite://x".into());
        apply_entity_output_default(&mut command);
        let GenerateSubcommands::Entity { output_dir, .. } = command;
        assert_eq!(output_dir, DEFAULT_ENTITY_OUTPUT_DIR);
    }
}
