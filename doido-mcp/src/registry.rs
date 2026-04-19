use doido_core::Result;
use serde_json::Value;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

pub type ToolHandler =
    Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync>;
pub type ResourceHandler =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct ResourceDef {
    pub uri: String,
    pub description: String,
}

pub struct ToolRegistry {
    tools: HashMap<String, (ToolDef, ToolHandler)>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: ToolDef, handler: ToolHandler) {
        self.tools.insert(def.name.clone(), (def, handler));
    }

    pub fn list(&self) -> Vec<&ToolDef> {
        self.tools.values().map(|(def, _)| def).collect()
    }

    pub async fn call(&self, name: &str, params: Value) -> Result<Value> {
        let (_, handler) = self
            .tools
            .get(name)
            .ok_or_else(|| doido_core::anyhow::anyhow!("tool '{}' not found", name))?;
        handler(params).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ResourceRegistry {
    resources: HashMap<String, (ResourceDef, ResourceHandler)>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: ResourceDef, handler: ResourceHandler) {
        self.resources.insert(def.uri.clone(), (def, handler));
    }

    pub fn list(&self) -> Vec<&ResourceDef> {
        self.resources.values().map(|(def, _)| def).collect()
    }

    pub async fn read(&self, uri: &str) -> Result<Value> {
        let (_, handler) = self
            .resources
            .get(uri)
            .ok_or_else(|| doido_core::anyhow::anyhow!("resource '{}' not found", uri))?;
        handler().await
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
