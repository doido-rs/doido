//! Framework default initializer for background jobs subsystem and application boot hooks.

use crate::queue::JobQueue;
use crate::registry::JobRegistry;
use doido_core::tracing;
use doido_core::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type BootHook =
    Box<dyn FnOnce(Arc<dyn JobQueue>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send>;

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static BOOT_HOOKS: Mutex<Vec<BootHook>> = Mutex::new(Vec::new());

/// Register an asynchronous boot routine that receives the configured [`JobQueue`].
///
/// Hooks run during application boot in server and worker modes when the jobs
/// subsystem initializes.
pub fn on_boot<F, Fut>(hook: F)
where
    F: FnOnce(Arc<dyn JobQueue>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let mut hooks = BOOT_HOOKS.lock().unwrap();
    hooks.push(Box::new(move |queue| Box::pin(hook(queue))));
}

/// Resets the initializer state and clears registered boot hooks for testing.
#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
    let mut hooks = BOOT_HOOKS.lock().unwrap();
    hooks.clear();
}

/// Framework default initializer registered for `jobs` (priority 30).
///
/// Discovers jobs in inventory, inspects execution context (skipping migrations/admin commands),
/// builds the configured queue backend, and executes registered [`on_boot`] hooks.
pub fn default_initializer() -> Result<()> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    // 1. Discover and log handlers registered via inventory
    let registry = JobRegistry::from_inventory();
    let names = registry.names();
    if names.is_empty() {
        tracing::debug!("no job handlers registered in inventory");
    } else {
        tracing::info!("registered jobs: {:?}", names);
    }

    // 2. CLI awareness: skip in migration and administrative contexts
    let ctx = doido_core::cli_context();
    if ctx.is_skip() {
        return Ok(());
    }

    // 3. Build queue and execute registered on_boot hooks
    doido_core::initializers::run_async(async {
        let cfg = crate::config::load();
        let queue = crate::config::build_configured_queue(&cfg).await?;

        let hooks: Vec<_> = {
            let mut guard = BOOT_HOOKS.lock().unwrap();
            std::mem::take(&mut *guard)
        };

        for hook in hooks {
            hook(queue.clone()).await?;
        }

        Ok(())
    })
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "jobs",
        priority: 30,
        init: default_initializer,
    }
}
