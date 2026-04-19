use crate::generator::{GeneratedFile, Generator};
use doido_core::Result;
use std::collections::HashMap;

pub struct GeneratorRegistry {
    generators: HashMap<String, Box<dyn Generator>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        Self {
            generators: HashMap::new(),
        }
    }

    pub fn register(&mut self, generator: Box<dyn Generator>) {
        self.generators
            .insert(generator.name().to_string(), generator);
    }

    pub fn run(&self, name: &str, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let gen = self
            .generators
            .get(name)
            .ok_or_else(|| doido_core::anyhow::anyhow!("generator '{}' not found", name))?;
        gen.generate(args)
    }

    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.generators.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
