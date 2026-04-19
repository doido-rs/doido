pub mod generator;
pub mod generators;
pub mod registry;

pub use generator::{GeneratedFile, Generator};
pub use generators::{
    channel::ChannelGenerator, controller::ControllerGenerator, job::JobGenerator,
    mailer::MailerGenerator, migration::MigrationGenerator, model::ModelGenerator,
    scaffold::ScaffoldGenerator,
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
    reg
}
