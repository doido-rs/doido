//! `Doido` builder tests (kept out of `src/cli.rs` for llvm-cov line counts).

use doido::Doido;
use doido_core::{BootPolicy, InitPolicy};
use doido_generators::{GeneratedFile, Generator};
use doido_model::sea_orm::DatabaseConnection;
use doido_model::sea_orm_migration::{MigrationTrait, MigratorTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
fn builder_collects_router_generators_migrator_and_seeder() {
    struct TestMigrator;
    #[doido_core::async_trait]
    impl MigratorTrait for TestMigrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            Vec::new()
        }
    }

    async fn test_seeder(_conn: &DatabaseConnection) -> doido_core::Result<()> {
        Ok(())
    }

    let _doido = Doido::new()
        .router(doido_controller::axum::Router::new())
        .register_generator(Box::new(DummyGenerator))
        .generators(vec![Box::new(DummyGenerator)])
        .boot_policy(BootPolicy {
            i18n: InitPolicy::FailFast,
            storage: InitPolicy::Warn,
        })
        .storage_config_loader(Box::new(doido_storage::config::load))
        .migrator::<TestMigrator>()
        .seeder(test_seeder);
}

#[test]
fn builder_accepts_before_run_hook() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    let _doido = Doido::new().before_run(move || {
        hits2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}
