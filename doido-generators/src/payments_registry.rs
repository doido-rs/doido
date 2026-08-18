//! Adapter merging `doido-payments` generators into the CLI registry.

use crate::generator::{GeneratedFile, Generator};
use crate::registry::GeneratorRegistry;

struct PaymentsGeneratorAdapter(Box<dyn doido_payments::generators::PaymentsGenerator>);

impl Generator for PaymentsGeneratorAdapter {
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

impl doido_payments::generators::PaymentsGeneratorRegistry for RegistryAdapter<'_> {
    fn register_payments(
        &mut self,
        generator: Box<dyn doido_payments::generators::PaymentsGenerator>,
    ) {
        self.0
            .register(Box::new(PaymentsGeneratorAdapter(generator)));
    }
}

/// Merge payment generators into `reg`.
pub fn register_payments_generators(reg: &mut GeneratorRegistry) {
    let mut adapter = RegistryAdapter(reg);
    doido_payments::generators::register(&mut adapter);
}

/// Payment generator names owned by `doido-payments`.
pub fn payments_generator_names() -> &'static [&'static str] {
    doido_payments::generators::generator_names()
}
