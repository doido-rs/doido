//! Custom storage adapters — the first-class extension point for integrating an
//! external file service.
//!
//! Implement [`Service`](crate::Service) for your backend, then register a factory
//! under a `kind` string at boot:
//!
//! ```no_run
//! use doido_storage::{register_adapter, Service, ServiceConfig};
//! # struct DropboxService;
//! # #[async_trait::async_trait]
//! # impl Service for DropboxService {
//! #   fn name(&self) -> &str { "dropbox" }
//! #   async fn upload(&self, _k: &str, _d: Vec<u8>, _c: Option<&str>) -> doido_core::Result<()> { Ok(()) }
//! #   async fn download(&self, _k: &str) -> doido_core::Result<Vec<u8>> { Ok(vec![]) }
//! #   async fn delete(&self, _k: &str) -> doido_core::Result<()> { Ok(()) }
//! #   async fn exists(&self, _k: &str) -> doido_core::Result<bool> { Ok(false) }
//! #   async fn size(&self, _k: &str) -> doido_core::Result<u64> { Ok(0) }
//! # }
//! use std::sync::Arc;
//!
//! register_adapter("dropbox", |name: &str, cfg: &ServiceConfig| {
//!     let _token = cfg.option_str("token");
//!     Ok(Arc::new(DropboxService) as Arc<dyn Service>)
//! });
//! ```
//!
//! Then select it from `config/<env>.yml` like any built-in backend:
//!
//! ```yaml
//! storage:
//!   service: files
//!   services:
//!     files: { type: dropbox, token: "...", root: "/app" }
//! ```

use crate::config::ServiceConfig;
use crate::error::StorageError;
use crate::service::Service;
use doido_core::Result;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

/// Builds a [`Service`] from a named service's config. The factory is synchronous
/// (construct the client here; do network I/O lazily in the async `Service`
/// methods), matching `S3Service::connect` / `AzureBlobService::connect`.
pub trait ServiceFactory: Send + Sync {
    /// Build the service named `name` from `config`.
    fn build(&self, name: &str, config: &ServiceConfig) -> Result<Arc<dyn Service>>;
}

/// Any matching closure is a [`ServiceFactory`].
impl<F> ServiceFactory for F
where
    F: Fn(&str, &ServiceConfig) -> Result<Arc<dyn Service>> + Send + Sync,
{
    fn build(&self, name: &str, config: &ServiceConfig) -> Result<Arc<dyn Service>> {
        self(name, config)
    }
}

static REGISTRY: LazyLock<RwLock<HashMap<String, Arc<dyn ServiceFactory>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Register a custom adapter factory under `kind`. Selecting `type: <kind>` in
/// `config/<env>.yml` will then build a service through `factory`. Registering the
/// same `kind` twice replaces the previous factory.
pub fn register_adapter(kind: impl Into<String>, factory: impl ServiceFactory + 'static) {
    REGISTRY
        .write()
        .unwrap()
        .insert(kind.into(), Arc::new(factory));
}

/// The kinds currently registered (sorted), for diagnostics.
pub fn registered_adapters() -> Vec<String> {
    let mut kinds: Vec<String> = REGISTRY.read().unwrap().keys().cloned().collect();
    kinds.sort();
    kinds
}

/// Build a service for a `type: <kind>` that isn't a built-in backend, by looking
/// up the registered factory. Errors clearly when nothing is registered.
pub(crate) fn build_adapter(
    kind: &str,
    name: &str,
    config: &ServiceConfig,
) -> Result<Arc<dyn Service>> {
    let factory = REGISTRY.read().unwrap().get(kind).cloned();
    match factory {
        Some(factory) => factory.build(name, config),
        None => Err(StorageError::Config(format!(
            "no storage adapter registered for type {kind:?}; register one at boot with \
             doido_storage::register_adapter({kind:?}, ...). registered: {:?}",
            registered_adapters()
        ))
        .into()),
    }
}
