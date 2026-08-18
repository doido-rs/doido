pub mod auth_registry;
pub mod commands;
pub mod dev_workspace;
pub mod generator;
pub mod generators;
pub mod new_options;
pub mod project_auth;
pub mod project_generator;
pub mod registry;
pub mod templates;

pub use dev_workspace::DependencyMode;

/// The Doido release version generated apps depend on when this crate is
/// isolated or published. Matches `CARGO_PKG_VERSION` of the `doido` metacrate.
pub const DOIDO_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use generator::{GeneratedFile, Generator};
pub use generators::{
    channel::ChannelGenerator, controller::ControllerGenerator, generator_gen::GeneratorGenerator,
    helper::HelperGenerator, job::JobGenerator, mailer::MailerGenerator,
    migration::MigrationGenerator, model::ModelGenerator, new::ProjectGenerator,
    resource::ResourceGenerator, scaffold::ScaffoldGenerator,
    storage_adapter::StorageAdapterGenerator, storage_install::StorageInstallGenerator,
    templates_gen::TemplatesGenerator,
};
pub use registry::GeneratorRegistry;

pub fn default_registry() -> GeneratorRegistry {
    let mut reg = GeneratorRegistry::new();
    reg.register(Box::new(ControllerGenerator));
    reg.register(Box::new(HelperGenerator));
    reg.register(Box::new(ModelGenerator));
    reg.register(Box::new(MigrationGenerator));
    reg.register(Box::new(JobGenerator));
    reg.register(Box::new(MailerGenerator));
    reg.register(Box::new(ChannelGenerator));
    reg.register(Box::new(ScaffoldGenerator));
    reg.register(Box::new(ResourceGenerator));
    reg.register(Box::new(StorageInstallGenerator));
    reg.register(Box::new(StorageAdapterGenerator));
    reg.register(Box::new(ProjectGenerator));
    reg.register(Box::new(TemplatesGenerator));
    reg.register(Box::new(GeneratorGenerator));
    reg.register(Box::new(crate::generators::locale::LocaleGenerator));
    reg
}
