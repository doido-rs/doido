pub mod generator;
pub mod registry;
pub mod generators;

pub use generator::{GeneratedFile, Generator};
pub use registry::GeneratorRegistry;
pub use generators::{
    controller::ControllerGenerator,
    model::ModelGenerator,
    migration::MigrationGenerator,
    job::JobGenerator,
    mailer::MailerGenerator,
    channel::ChannelGenerator,
    scaffold::ScaffoldGenerator,
};

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
