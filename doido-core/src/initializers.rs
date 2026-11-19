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

/// Execution contexts detected during application boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionContext {
    /// Server mode (`doido server` or bare app binary): initializes web server defaults.
    Server,
    /// Worker mode (`doido worker`): initializes background worker defaults.
    Worker,
    /// Commands that manage their own connections or do not need an application pool
    /// (e.g. `db migrate`, `db seed`, `generate`, test runners).
    Skip,
}

/// Detects the execution context based on environment variables and CLI arguments.
pub fn detect_execution_context() -> ExecutionContext {
    // 1. Explicit environment variable override (useful for tests and containerized setups)
    if let Ok(cmd) = std::env::var("DOIDO_COMMAND") {
        match cmd.to_lowercase().as_str() {
            "server" => return ExecutionContext::Server,
            "worker" => return ExecutionContext::Worker,
            _ => return ExecutionContext::Skip,
        }
    }

    // 2. Test runner detection: if running under `cargo test` / test harness, skip by default
    if is_test_runner() {
        return ExecutionContext::Skip;
    }

    // 3. Inspect command-line arguments: find first non-flag argument after executable
    let subcmd = std::env::args().skip(1).find(|arg| !arg.starts_with('-'));

    match subcmd.as_deref() {
        Some("worker") => ExecutionContext::Worker,
        Some("server") | None => ExecutionContext::Server,
        // Any other command (db, generate, routes, console, credentials, config, new, jobs...)
        // does not need the application database pool installed at initializer time
        _ => ExecutionContext::Skip,
    }
}

/// Heuristic to detect if running under `cargo test` / test harness.
pub fn is_test_runner() -> bool {
    if crate::Environment::get_env().is_test() {
        return true;
    }
    if std::env::args().any(|a| a == "--test" || a == "--bench") {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("/deps/") || exe_str.contains(r"\deps\") {
            return true;
        }
    }
    false
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
