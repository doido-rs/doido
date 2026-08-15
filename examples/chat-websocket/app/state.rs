//! Shared runtime handles wired at router boot (cable pub/sub).
use doido::cable::Cable;
use std::sync::{Arc, OnceLock};

static CABLE: OnceLock<Arc<Cable>> = OnceLock::new();

/// Store the global [`Cable`] handle used by channels and controllers.
pub fn init_cable(cable: Arc<Cable>) {
    let _ = CABLE.set(cable);
}

/// Clone the global [`Cable`] handle.
pub fn cable() -> Arc<Cable> {
    CABLE
        .get()
        .cloned()
        .expect("cable not initialized — call init_cable in routes::router")
}
