//! `doido generate storage:adapter <Name>` — scaffolds a custom storage adapter
//! (an external file-service integration) implementing `doido_storage::Service`.
//!
//! Emits `app/storage/<snake>_service.rs` with a `<Pascal>Service` skeleton plus a
//! `register()` that wires it into the storage adapter registry, so the app can
//! select it from `config/<env>.yml` with `type: <snake>`.

use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

pub struct StorageAdapterGenerator;

fn render(name: &str) -> String {
    let snake = to_snake(name);
    let pascal = to_pascal(name);
    format!(
        r#"//! Custom storage adapter `{pascal}Service` — integrate an external file service.
//!
//! Wire it up: declare this module in your app, call `{snake}_service::register()`
//! once at boot, then select it in `config/<env>.yml`:
//!
//! ```yaml
//! storage:
//!   service: files
//!   services:
//!     files: {{ type: {snake}, token: "...", root: "/app" }}
//! ```

use doido::core::Result;
use doido::storage::{{register_adapter, Service, ServiceConfig}};
use std::sync::Arc;

/// Adapter for your external file service.
pub struct {pascal}Service {{
    name: String,
    // TODO: hold your HTTP/SDK client + credentials here.
}}

impl {pascal}Service {{
    /// Build the adapter from a named service's config. Read custom keys with
    /// `cfg.option_str("...")` and the shared fields (`cfg.bucket`, `cfg.endpoint`).
    pub fn connect(name: &str, cfg: &ServiceConfig) -> Result<Self> {{
        let _token = cfg.option_str("token");
        Ok(Self {{ name: name.to_string() }})
    }}
}}

/// Register this adapter under `type: {snake}`. Call once at boot.
pub fn register() {{
    register_adapter("{snake}", |name: &str, cfg: &ServiceConfig| {{
        Ok(Arc::new({pascal}Service::connect(name, cfg)?) as Arc<dyn Service>)
    }});
}}

#[doido::core::async_trait]
impl Service for {pascal}Service {{
    fn name(&self) -> &str {{
        &self.name
    }}

    async fn upload(&self, key: &str, _data: Vec<u8>, _content_type: Option<&str>) -> Result<()> {{
        todo!("upload object {{key}} to your service")
    }}

    async fn download(&self, key: &str) -> Result<Vec<u8>> {{
        todo!("download object {{key}} from your service")
    }}

    async fn delete(&self, key: &str) -> Result<()> {{
        todo!("delete object {{key}} from your service")
    }}

    async fn exists(&self, key: &str) -> Result<bool> {{
        todo!("check whether object {{key}} exists")
    }}

    async fn size(&self, key: &str) -> Result<u64> {{
        todo!("return the byte size of object {{key}}")
    }}

    // Optionally override `url`/`presigned_put` to return a native (presigned)
    // URL. The defaults return `None`, so blobs are served through the app proxy.
}}
"#
    )
}

impl Generator for StorageAdapterGenerator {
    fn name(&self) -> &str {
        "storage:adapter"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("storage:adapter generator requires a name argument")
        })?;
        let snake = to_snake(name);
        Ok(vec![GeneratedFile {
            path: format!("app/storage/{snake}_service.rs"),
            content: render(name),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_service_skeleton_and_register() {
        let files = StorageAdapterGenerator.generate(&["Dropbox"]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "app/storage/dropbox_service.rs");
        let content = &files[0].content;
        assert!(content.contains("pub struct DropboxService"));
        assert!(content.contains("impl Service for DropboxService"));
        assert!(content.contains("register_adapter(\"dropbox\""));
        assert!(content.contains("pub fn register()"));
    }

    #[test]
    fn requires_a_name() {
        assert!(StorageAdapterGenerator.generate(&[]).is_err());
    }
}
