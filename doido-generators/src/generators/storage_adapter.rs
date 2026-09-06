//! `doido generate storage:adapter <Name>` — scaffolds a custom storage adapter
//! (an external file-service integration) implementing `doido_storage::Service`.
//!
//! Emits `app/storage/<snake>_service.rs` with a `<Pascal>Service` skeleton plus a
//! `register()` that wires it into the storage adapter registry, and updates
//! `app/storage/mod.rs` with a `register_all()` boot hook.

use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

pub struct StorageAdapterGenerator;

const STORAGE_MOD_PATH: &str = "app/storage/mod.rs";

fn render_service(name: &str) -> String {
    let snake = to_snake(name);
    let pascal = to_pascal(name);
    format!(
        r#"//! Custom storage adapter `{pascal}Service` — integrate an external file service.
//!
//! Registered via [`super::register_all`] at boot (see `app/storage/mod.rs`).
//! Select it in `config/<env>.yml`:
//!
//! ```yaml
//! storage:
//!   driver: files
//!   drivers:
//!     files: {{ type: {snake}, token: "...", root: "/app" }}
//! ```
#![allow(dead_code)]

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

/// Register this adapter under `type: {snake}`. Called from [`super::register_all`].
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

fn render_storage_mod(modules: &[String]) -> String {
    let mut out = String::from(
        "//! Custom storage adapters. Call [`register_all`] from `src/main.rs` before\n\
         //! `Doido::new().run()` so adapters are registered before storage boots.\n\n",
    );
    for snake in modules {
        out.push_str(&format!("pub mod {snake}_service;\n"));
    }
    out.push_str("\n/// Register every custom storage adapter. Call once at boot.\npub fn register_all() {\n");
    for snake in modules {
        out.push_str(&format!("    {snake}_service::register();\n"));
    }
    out.push_str("}\n");
    out
}

fn merge_storage_mod(existing: &str, snake: &str) -> String {
    let module_line = format!("pub mod {snake}_service;");
    let register_line = format!("    {snake}_service::register();");

    if existing.contains(&module_line) && existing.contains(&register_line) {
        return existing.to_string();
    }

    let mut modules: Vec<String> = existing
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("pub mod ")
                .and_then(|rest| rest.strip_suffix("_service;"))
                .map(str::to_string)
        })
        .collect();

    if !modules.iter().any(|m| m == snake) {
        modules.push(snake.to_string());
    }
    modules.sort();
    modules.dedup();
    render_storage_mod(&modules)
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

        let mod_content = match std::fs::read_to_string(STORAGE_MOD_PATH) {
            Ok(existing) => merge_storage_mod(&existing, &snake),
            Err(_) => render_storage_mod(std::slice::from_ref(&snake)),
        };

        Ok(vec![
            GeneratedFile {
                path: format!("app/storage/{snake}_service.rs"),
                content: render_service(name),
            },
            GeneratedFile {
                path: STORAGE_MOD_PATH.to_string(),
                content: mod_content,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_service_skeleton_mod_and_register() {
        let files = StorageAdapterGenerator.generate(&["Dropbox"]).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "app/storage/dropbox_service.rs");
        assert_eq!(files[1].path, "app/storage/mod.rs");
        let content = &files[0].content;
        assert!(content.contains("pub struct DropboxService"));
        assert!(content.contains("impl Service for DropboxService"));
        assert!(content.contains("register_adapter(\"dropbox\""));
        assert!(content.contains("pub fn register()"));
        let mod_rs = &files[1].content;
        assert!(mod_rs.contains("pub mod dropbox_service;"));
        assert!(mod_rs.contains("dropbox_service::register();"));
        assert!(mod_rs.contains("pub fn register_all()"));
    }

    #[test]
    fn requires_a_name() {
        assert!(StorageAdapterGenerator.generate(&[]).is_err());
    }

    #[test]
    fn merge_storage_mod_is_idempotent() {
        let first = render_storage_mod(&["dropbox".to_string()]);
        let second = merge_storage_mod(&first, "dropbox");
        assert_eq!(first, second);
    }
}
