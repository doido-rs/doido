//! Framework default initializers and boot-time initializer engine (Rails `config/initializers/*`).

use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Default framework initializer for `doido-core`.
///
/// Idempotently sets up basic logging/tracing and loads the `.env` file when
/// running in the [`crate::Environment::Development`] environment.
pub fn default_initializer() -> crate::Result<()> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    // 1. Basic logging / tracing if not already configured
    crate::logger::init();

    // 2. Load .env in development
    if crate::Environment::get_env() == crate::Environment::Development {
        let _ = dotenvy::dotenv();
    }

    Ok(())
}

#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
}

/// A framework initializer registered at link-time via `inventory`.
pub struct FrameworkInitializer {
    pub name: &'static str,
    pub priority: u32,
    pub init: fn() -> crate::Result<()>,
}

inventory::collect!(FrameworkInitializer);

inventory::submit! {
    FrameworkInitializer {
        name: "core",
        priority: 0,
        init: default_initializer,
    }
}

type Init = Box<dyn FnOnce() -> crate::Result<()> + Send>;

/// An ordered registry of named initializers.
#[derive(Default)]
pub struct Initializers {
    inits: Vec<(String, Init)>,
}

impl Initializers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the registry pre-populated with framework defaults (`core`, `db`, etc.)
    /// discovered via link-time registration.
    pub fn with_defaults() -> Self {
        let mut inits = Self::new();
        let mut framework_inits: Vec<_> = inventory::iter::<FrameworkInitializer>
            .into_iter()
            .collect();
        framework_inits.sort_by_key(|f| f.priority);
        let mut has_core = false;
        for item in framework_inits {
            if item.name == "core" {
                has_core = true;
            }
            inits.register(item.name, item.init);
        }
        if !has_core {
            inits.register("core", default_initializer);
        }
        inits
    }

    /// Register an initializer to run at boot.
    pub fn register(
        &mut self,
        name: &str,
        init: impl FnOnce() -> crate::Result<()> + Send + 'static,
    ) -> &mut Self {
        self.inits.push((name.to_string(), Box::new(init)));
        self
    }

    /// Names of registered initializers, in order.
    pub fn names(&self) -> Vec<&str> {
        self.inits.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Run every initializer in registration order; stop at the first error.
    pub fn run_all(self) -> crate::Result<()> {
        for (name, init) in self.inits {
            init().map_err(|e| crate::anyhow::anyhow!("initializer `{name}` failed: {e}"))?;
        }
        Ok(())
    }
}
