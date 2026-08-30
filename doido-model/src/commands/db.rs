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

use crate::sea_orm_cli::{
    handle_error, run_generate_command, run_migrate_command, BannerVersion, BigIntegerType,
    Commands, DateTimeCrate, GenerateSubcommands, MigrateSubcommands,
};
use crate::sea_orm_migration::MigratorTrait;
use crate::DatabaseConnection;
use clap::Subcommand;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;

/// A boxed, borrow-scoped future returning a Doido [`Result`](doido_core::Result).
/// Used to type-erase the app's registered seeder/migrator so the CLI can run
/// them in-process without knowing their concrete types. Not `Send`: the CLI
/// future is `block_on`'d on the calling thread (never spawned), so an app's
/// seeder need not return a `Send` future.
pub type BoxFut<'a> = Pin<Box<dyn Future<Output = doido_core::Result<()>> + 'a>>;

/// A registered database seeder, type-erased for storage on the `Doido` builder.
///
/// Blanket-implemented for any `async fn(&DatabaseConnection) -> Result<()>`, so
/// apps register a plain async fn via `Doido::seeder(..)`. A trait (rather than a
/// boxed closure) is used because [`seed`](Seeder::seed) may legitimately return
/// a future borrowing `&self` — a `Fn` closure can't let a borrow of a captured
/// value escape its body.
pub trait Seeder {
    /// Run the seeder against `conn`.
    fn seed<'a>(&'a self, conn: &'a DatabaseConnection) -> BoxFut<'a>;
}

impl<F> Seeder for F
where
    F: AsyncFn(&DatabaseConnection) -> doido_core::Result<()>,
{
    fn seed<'a>(&'a self, conn: &'a DatabaseConnection) -> BoxFut<'a> {
        Box::pin((*self)(conn))
    }
}

/// The app's registered seeder, type-erased. Registered via `Doido::seeder(..)`
/// and run in-process by [`run`] (`doido db seed`), so its `INSERT`s log through
/// the app's tracing subscriber.
pub type SeederFn = Box<dyn Seeder>;

/// The app's registered migrator dispatcher, produced by `Doido::migrator::<M>()`.
/// Maps a [`MigrateSubcommands`] to the matching [`MigratorTrait`] method and runs
/// it in-process against the app connection (so DDL logs like any other statement).
pub type MigratorFn =
    Box<dyn for<'a> Fn(Option<MigrateSubcommands>, &'a DatabaseConnection) -> BoxFut<'a>>;

/// Runs a migrate subcommand in-process against `conn` using `M`'s
/// [`MigratorTrait`] methods — the in-binary replacement for shelling out to
/// `cargo run --manifest-path db/migration/…`. `None` applies all pending
/// migrations (SeaORM CLI's default). `Init`/`Generate` are filesystem-only
/// scaffolding and are handled by the CLI, not here.
pub async fn run_migrator<M: MigratorTrait>(
    command: Option<MigrateSubcommands>,
    conn: &DatabaseConnection,
) -> doido_core::Result<()> {
    let result = match command {
        None => M::up(conn, None).await,
        Some(MigrateSubcommands::Up { num }) => M::up(conn, num).await,
        Some(MigrateSubcommands::Down { num }) => M::down(conn, Some(num)).await,
        Some(MigrateSubcommands::Fresh) => M::fresh(conn).await,
        Some(MigrateSubcommands::Refresh) => M::refresh(conn).await,
        Some(MigrateSubcommands::Reset) => M::reset(conn).await,
        Some(MigrateSubcommands::Status) => M::status(conn).await,
        Some(MigrateSubcommands::Init) | Some(MigrateSubcommands::Generate { .. }) => {
            return Err(doido_core::anyhow::anyhow!(
                "`migrate init`/`generate` scaffold files and are handled by the CLI, not run_migrator"
            ));
        }
    };
    result.map_err(|e| doido_core::anyhow::anyhow!("migrate failed: {e}"))
}

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
    /// Export an interactive ER diagram to HTML
    Diagram {
        /// Output path (default: db/schema.html)
        #[arg(short, long, default_value = "db/schema.html")]
        output: PathBuf,
        /// Tables to skip (repeatable; `seaql_migrations` is always ignored)
        #[arg(long = "ignore-table")]
        ignore_tables: Vec<String>,
    },
}

/// Where Doido keeps its SeaORM migration crate.
const DEFAULT_MIGRATION_DIR: &str = "db/migration";
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
    if let Ok(config) = crate::config::YamlConfig::load() {
        std::env::set_var("DATABASE_URL", config.database.url);
    }
}

/// Runs a `doido db <command>`.
///
/// `migrator`/`seeder` are the app's registrations from the [`Doido`] builder
/// (`.migrator::<M>()` / `.seeder(..)`). They run in-process against the app
/// connection so migrate/seed no longer fork `cargo run`, and their SQL logs
/// through the app's tracing subscriber. Both are `None` for the bare `doido`
/// binary, where `migrate`/`seed` report that nothing is registered.
pub async fn run(
    command: DbCommand,
    verbose: bool,
    migrator: Option<MigratorFn>,
    seeder: Option<SeederFn>,
) {
    match command {
        DbCommand::Create => create().await,
        DbCommand::Reset => reset().await,
        DbCommand::Prepare => prepare().await,
        DbCommand::Seed => seed(seeder).await,
        DbCommand::Schema { action } => schema(action).await,
        DbCommand::SeaOrm(command) => run_sea_orm(command, verbose, migrator).await,
    }
}

/// Opens a connection to the resolved [`database_url`], exiting on failure.
async fn connect() -> crate::DatabaseConnection {
    let url = database_url();
    match crate::connect_with_url(&url).await {
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
    match crate::tasks::reset(&conn, &schema).await {
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
    match crate::tasks::prepare(&conn, &schema).await {
        Ok(()) => doido_core::tracing::info!("prepared database from {SCHEMA_FILE}"),
        Err(e) => doido_core::tracing::error!("db prepare failed: {e}"),
    }
}

/// `doido db seed` — run the app's registered seeder in-process against the app
/// connection. The seeder (`db/seeds.rs`'s `run`, registered via
/// `Doido::seeder(..)`) inserts data using the app's SeaORM models; because it
/// runs in-binary its `INSERT`s log through the app's tracing subscriber.
async fn seed(seeder: Option<SeederFn>) {
    let Some(seeder) = seeder else {
        doido_core::tracing::error!(
            "no seeder registered; add `.seeder(seed::run)` to `Doido::new()` in src/main.rs"
        );
        return;
    };
    let conn = connect().await;
    match seeder.seed(&conn).await {
        Ok(()) => doido_core::tracing::info!("seeded database"),
        Err(e) => doido_core::tracing::error!("db seed failed: {e}"),
    }
}

/// `doido db schema dump|load|diagram`.
async fn schema(action: SchemaCommand) {
    match action {
        SchemaCommand::Dump => {
            let conn = connect().await;
            match crate::schema::dump_to_file(&conn, SCHEMA_FILE).await {
                Ok(()) => doido_core::tracing::info!("wrote schema to {SCHEMA_FILE}"),
                Err(e) => doido_core::tracing::error!("schema dump failed: {e}"),
            }
        }
        SchemaCommand::Load => {
            let Some(sql) = read_sql_file(SCHEMA_FILE) else {
                return;
            };
            let conn = connect().await;
            match crate::schema::load(&conn, &sql).await {
                Ok(()) => doido_core::tracing::info!("loaded schema from {SCHEMA_FILE}"),
                Err(e) => doido_core::tracing::error!("schema load failed: {e}"),
            }
        }
        SchemaCommand::Diagram {
            output,
            ignore_tables,
        } => schema_diagram(output, ignore_tables).await,
    }
}

/// `doido db schema diagram` — introspect the live database and write HTML.
async fn schema_diagram(output: PathBuf, ignore_tables: Vec<String>) {
    let url = database_url();
    let ignore = crate::schema_design::resolve_ignore_tables(&ignore_tables);
    match crate::schema_design::introspect_from_url(&url, None, &ignore).await {
        Ok(design) => match crate::schema_design::write_html(&design, &output) {
            Ok(()) => doido_core::tracing::info!("wrote ER diagram to {}", output.display()),
            Err(e) => doido_core::tracing::error!("schema diagram export failed: {e}"),
        },
        Err(e) => doido_core::tracing::error!("schema introspection failed: {e}"),
    }
}

/// Creates the database named by the resolved [`database_url`].
async fn create() {
    let url = database_url();
    match crate::create_database(&url).await {
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
    if let Ok(config) = crate::config::YamlConfig::load() {
        return config.database.url;
    }
    doido_core::tracing::error!("DATABASE_URL is not set and config/<env>.yml could not be read");
    std::process::exit(1);
}

/// Dispatches a flattened SeaORM CLI command, applying Doido's directory defaults.
async fn run_sea_orm(command: Commands, verbose: bool, migrator: Option<MigratorFn>) {
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
            // `init`/`generate` only scaffold migration source files — no DB, no
            // fork — so they stay on the SeaORM CLI. Every DB-executing subcommand
            // runs in-process against the app connection via the registered
            // migrator, replacing the old `cargo run` on `db/migration`.
            if is_filesystem_migrate(command.as_ref()) {
                let migration_dir = override_migration_dir(migration_dir);
                run_migrate_command(command, &migration_dir, database_schema, database_url, verbose)
                    .unwrap_or_else(handle_error);
                return;
            }
            let Some(migrator) = migrator else {
                doido_core::tracing::error!(
                    "no migrator registered; add `.migrator::<migration::Migrator>()` to `Doido::new()` in src/main.rs"
                );
                return;
            };
            let export = should_export_entities(command.as_ref());
            let conn = match database_url {
                Some(url) => match crate::connect_with_url(&url).await {
                    Ok(conn) => conn,
                    Err(e) => {
                        doido_core::tracing::error!("failed to connect to {url}: {e}");
                        return;
                    }
                },
                None => connect().await,
            };
            if let Err(e) = migrator(command, &conn).await {
                doido_core::tracing::error!("db migrate failed: {e}");
                return;
            }
            if export {
                export_entities_from_database(verbose).await;
            }
        }
    }
}

/// Whether a migrate subcommand only touches the filesystem (`init`/`generate`)
/// and so runs via the SeaORM CLI rather than in-process against the database.
fn is_filesystem_migrate(command: Option<&MigrateSubcommands>) -> bool {
    matches!(
        command,
        Some(MigrateSubcommands::Init) | Some(MigrateSubcommands::Generate { .. })
    )
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
    let entities_dir = Path::new("app/models/_entities");
    let models_dir = Path::new("app/models");
    if !entities_dir.is_dir() {
        return;
    }
    match crate::entities::postprocess_entity_export(entities_dir, models_dir) {
        Ok(()) => doido_core::tracing::debug!("synced model extensions"),
        Err(e) => doido_core::tracing::error!("model extension sync failed: {e}"),
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
        impl_active_model_behavior: false,
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

    #[test]
    fn init_and_generate_are_the_only_filesystem_migrations() {
        assert!(is_filesystem_migrate(Some(&MigrateSubcommands::Init)));
        assert!(!is_filesystem_migrate(None));
        assert!(!is_filesystem_migrate(Some(&MigrateSubcommands::Up {
            num: None
        })));
        assert!(!is_filesystem_migrate(Some(&MigrateSubcommands::Status)));
    }

    /// A migrator with no migrations: `up`/`status` still create and read the
    /// tracking table, exercising the in-process dispatch + connection path.
    struct EmptyMigrator;
    impl MigratorTrait for EmptyMigrator {
        fn migrations() -> Vec<Box<dyn crate::sea_orm_migration::MigrationTrait>> {
            Vec::new()
        }
    }

    #[tokio::test]
    async fn run_migrator_runs_in_process_against_the_connection() {
        let conn = crate::connect_with_url("sqlite::memory:").await.unwrap();
        // `None` applies all pending migrations (there are none) and succeeds.
        run_migrator::<EmptyMigrator>(None, &conn).await.unwrap();
        // `status` reads the tracking table in-process.
        run_migrator::<EmptyMigrator>(Some(MigrateSubcommands::Status), &conn)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_migrator_rejects_filesystem_subcommands() {
        let conn = crate::connect_with_url("sqlite::memory:").await.unwrap();
        assert!(run_migrator::<EmptyMigrator>(Some(MigrateSubcommands::Init), &conn)
            .await
            .is_err());
    }
}
