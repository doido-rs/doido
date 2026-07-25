//! Boot-time initializers (Rails `config/initializers/*`). Since initializers
//! are code, not data, register named init functions and run them in order at
//! boot; the first that errors stops the sequence.

use doido_core::Result;

type Init = Box<dyn FnOnce() -> Result<()> + Send>;

/// An ordered registry of named initializers.
#[derive(Default)]
pub struct Initializers {
    inits: Vec<(String, Init)>,
}

impl Initializers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an initializer to run at boot.
    pub fn register(
        &mut self,
        name: &str,
        init: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> &mut Self {
        self.inits.push((name.to_string(), Box::new(init)));
        self
    }

    /// Names of registered initializers, in order.
    pub fn names(&self) -> Vec<&str> {
        self.inits.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Run every initializer in registration order; stop at the first error.
    pub fn run_all(self) -> Result<()> {
        for (name, init) in self.inits {
            init().map_err(|e| doido_core::anyhow::anyhow!("initializer `{name}` failed: {e}"))?;
        }
        Ok(())
    }
}
