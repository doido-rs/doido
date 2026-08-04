//! Adapter merging `doido-auth` generators into the CLI registry.

use crate::generator::{GeneratedFile, Generator};
use crate::registry::GeneratorRegistry;

struct AuthGeneratorAdapter(Box<dyn doido_auth::generators::AuthGenerator>);

impl Generator for AuthGeneratorAdapter {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn generate(&self, args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> {
        self.0.generate(args).map(|files| {
            files
                .into_iter()
                .map(|f| GeneratedFile {
                    path: f.path,
                    content: f.content,
                })
                .collect()
        })
    }
}

struct RegistryAdapter<'a>(&'a mut GeneratorRegistry);

impl doido_auth::generators::AuthGeneratorRegistry for RegistryAdapter<'_> {
    fn register_auth(&mut self, generator: Box<dyn doido_auth::generators::AuthGenerator>) {
        let name = generator.name().to_string();
        self.0.register(Box::new(AuthGeneratorAdapter(generator)));
        // register() uses generator.name() at insert time; ensure key matches.
        let _ = name;
    }
}

/// Merge auth generators when the project lists `doido-auth`.
pub fn register_auth_generators(reg: &mut GeneratorRegistry) {
    let mut adapter = RegistryAdapter(reg);
    doido_auth::generators::register(&mut adapter);
}

/// Auth generator names owned by `doido-auth`.
pub fn auth_generator_names() -> &'static [&'static str] {
    &["auth:install", "auth:controller", "auth:scaffold"]
}
