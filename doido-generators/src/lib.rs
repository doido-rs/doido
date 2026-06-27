mod cli;
pub mod commands;
pub mod generator;
pub mod generators;
pub mod registry;

// CLI entry point (merged in from the former `doido-cli` crate).
pub use cli::run;

/// Absolute path to the doido workspace root, captured when `doido-generators`
/// was built. Generated apps point their `doido-*` path dependencies here so a
/// freshly generated app builds against the local framework crates.
pub const TEMPLATE_WORKSPACE_PATH: &str = env!("DOIDO_GENERATOR_TEMPLATE_WORKSPACE_PATH");

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
