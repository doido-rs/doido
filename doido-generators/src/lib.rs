pub mod generator;
pub mod generators;
pub mod registry;

/// `doido` semver written into generated app `Cargo.toml` (resolved when `doido-generators` is built).
pub const TEMPLATE_PINNED_DOIDO_VERSION: &str = env!("DOIDO_GENERATOR_TEMPLATE_DOIDO_VERSION");

/// `doido-controller` semver written into generated app `Cargo.toml`.
pub const TEMPLATE_PINNED_DOIDO_CONTROLLER_VERSION: &str =
    env!("DOIDO_GENERATOR_TEMPLATE_DOIDO_CONTROLLER_VERSION");

pub use generator::{GeneratedFile, Generator};
pub use generators::{
    channel::ChannelGenerator, controller::ControllerGenerator, job::JobGenerator,
    mailer::MailerGenerator, migration::MigrationGenerator, model::ModelGenerator,
    new::ProjectGenerator, scaffold::ScaffoldGenerator,
};
pub use registry::GeneratorRegistry;

pub fn default_registry() -> GeneratorRegistry {
    let mut reg = GeneratorRegistry::new();
    reg.register(Box::new(ControllerGenerator));
    reg.register(Box::new(ModelGenerator));
    reg.register(Box::new(MigrationGenerator));
    reg.register(Box::new(JobGenerator));
    reg.register(Box::new(MailerGenerator));
    reg.register(Box::new(ChannelGenerator));
    reg.register(Box::new(ScaffoldGenerator));
    reg.register(Box::new(ProjectGenerator));
    reg
}
