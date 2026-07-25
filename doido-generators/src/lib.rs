mod banner;
mod cli;
pub mod commands;
pub mod generator;
pub mod generators;
pub mod project_generator;
pub mod registry;
pub mod templates;

// CLI entry point (merged in from the former `doido-cli` crate).
pub use cli::run;

/// Absolute path to the doido workspace root, captured when `doido-generators`
/// was built. Generated apps point their `doido-*` path dependencies here so a
/// freshly generated app builds against the local framework crates.
pub const TEMPLATE_WORKSPACE_PATH: &str = env!("DOIDO_GENERATOR_TEMPLATE_WORKSPACE_PATH");

/// The doido release version generated apps depend on when this crate was built
/// outside the workspace (i.e. from a published crates.io release). Mirrors the
/// workspace version, so a published `doido-generators x.y.z` scaffolds apps
/// that depend on `doido x.y.z`.
pub const DOIDO_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether generated apps should use local `path` dependencies (`true`, set when
/// `doido-generators` is built inside the workspace) or crates.io `version`
/// dependencies (`false`, set once the crate is packaged/published). Decided at
/// build time by `build.rs` from whether the sibling framework crates are on disk.
pub const TEMPLATE_USE_PATH_DEPS: bool = {
    let flag = env!("DOIDO_GENERATOR_USE_PATH_DEPS").as_bytes();
    flag.len() == 1 && flag[0] == b'1'
};

pub use generator::{GeneratedFile, Generator};
pub use generators::{
    channel::ChannelGenerator, controller::ControllerGenerator, generator_gen::GeneratorGenerator,
    job::JobGenerator, mailer::MailerGenerator, migration::MigrationGenerator,
    model::ModelGenerator, new::ProjectGenerator, resource::ResourceGenerator,
    scaffold::ScaffoldGenerator, templates_gen::TemplatesGenerator,
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
    reg.register(Box::new(ResourceGenerator));
    reg.register(Box::new(ProjectGenerator));
    reg.register(Box::new(TemplatesGenerator));
    reg.register(Box::new(GeneratorGenerator));
    reg
}
