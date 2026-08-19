//! Code generators shipped by `{doido_ext_name}`.

mod install;

pub use install::InstallGenerator;

use doido_core::Result;

/// A file emitted by an extension generator.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Extension generator contract — mirrors `doido_auth::generators::AuthGenerator`
/// without a dependency on `doido-generators`.
pub trait ExtensionGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>>;
}

/// Implemented by registries that merge extension generators.
pub trait ExtensionGeneratorRegistry {
    fn register_extension(&mut self, generator: Box<dyn ExtensionGenerator>);
}

/// Register all generators from this extension.
pub fn register(reg: &mut impl ExtensionGeneratorRegistry) {
    reg.register_extension(Box::new(InstallGenerator));
}

/// Adapter so apps can install this extension on the `Doido` builder.
pub struct DoidoGenerator;

impl ExtensionGenerator for DoidoGenerator {
    fn name(&self) -> &str {
        ExtensionGenerator::name(&InstallGenerator)
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        ExtensionGenerator::generate(&InstallGenerator, args)
    }
}

impl doido::Generator for DoidoGenerator {
    fn name(&self) -> &str {
        ExtensionGenerator::name(self)
    }

    fn generate(&self, args: &[&str]) -> doido::core::Result<Vec<doido::GeneratedFile>> {
        ExtensionGenerator::generate(self, args).map(|files| {
            files
                .into_iter()
                .map(|f| doido::GeneratedFile {
                    path: f.path,
                    content: f.content,
                })
                .collect()
        })
    }
}

/// Convenience helper for `src/main.rs`.
pub fn install_on(builder: doido::Doido) -> doido::Doido {
    builder.register_generator(Box::new(DoidoGenerator))
}
