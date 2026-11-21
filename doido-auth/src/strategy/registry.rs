//! Custom strategy registration at boot.

use crate::strategy::AuthStrategy;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

static REGISTRY: OnceLock<RwLock<HashMap<String, Arc<dyn AuthStrategy>>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, Arc<dyn AuthStrategy>>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a custom auth strategy under `name`. Enable it in config via
/// `strategies: [cookie, ldap]`.
pub fn register_strategy(name: impl Into<String>, strategy: Arc<dyn AuthStrategy>) {
    registry()
        .write()
        .expect("strategy registry lock")
        .insert(name.into(), strategy);
}

/// Whether a custom strategy is registered (used by config validation).
pub fn has_strategy(name: &str) -> bool {
    registry()
        .read()
        .expect("strategy registry lock")
        .contains_key(name)
}

/// All registered custom strategy names (sorted).
pub fn registered_strategies() -> Vec<String> {
    let mut names: Vec<String> = registry()
        .read()
        .expect("strategy registry lock")
        .keys()
        .cloned()
        .collect();
    names.sort();
    names
}

/// Look up a registered custom strategy by name.
pub(crate) fn get_strategy(name: &str) -> Option<Arc<dyn AuthStrategy>> {
    registry()
        .read()
        .expect("strategy registry lock")
        .get(name)
        .cloned()
}

/// Collect all custom strategies registered before [`crate::state::init`].
pub(crate) fn all_custom_strategies() -> Vec<Arc<dyn AuthStrategy>> {
    registry()
        .read()
        .expect("strategy registry lock")
        .values()
        .cloned()
        .collect()
}
