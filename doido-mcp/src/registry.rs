use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use serde_json::Value;
use doido_core::Result;

pub type ToolHandler = Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync>;
pub type ResourceHandler = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync>;

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
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, def: ToolDef, handler: ToolHandler) {
        self.tools.insert(def.name.clone(), (def, handler));
    }

    pub fn list(&self) -> Vec<&ToolDef> {
        self.tools.values().map(|(def, _)| def).collect()
    }

    pub async fn call(&self, name: &str, params: Value) -> Result<Value> {
        let (_, handler) = self.tools.get(name)
            .ok_or_else(|| doido_core::anyhow::anyhow!("tool '{}' not found", name))?;
        handler(params).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}

pub struct ResourceRegistry {
    resources: HashMap<String, (ResourceDef, ResourceHandler)>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self { resources: HashMap::new() }
    }

    pub fn register(&mut self, def: ResourceDef, handler: ResourceHandler) {
        self.resources.insert(def.uri.clone(), (def, handler));
    }

    pub fn list(&self) -> Vec<&ResourceDef> {
        self.resources.values().map(|(def, _)| def).collect()
    }

    pub async fn read(&self, uri: &str) -> Result<Value> {
        let (_, handler) = self.resources.get(uri)
            .ok_or_else(|| doido_core::anyhow::anyhow!("resource '{}' not found", uri))?;
        handler().await
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tool_registry_register_and_call() {
        let mut reg = ToolRegistry::new();
        reg.register(
            ToolDef { name: "greet".to_string(), description: "says hello".to_string() },
            Arc::new(|params| Box::pin(async move {
                Ok(json!({"greeting": format!("Hello, {}!", params["name"].as_str().unwrap_or("world"))}))
            })),
        );
        let result = reg.call("greet", json!({"name": "Alice"})).await.unwrap();
        assert_eq!(result["greeting"], "Hello, Alice!");
    }

    #[tokio::test]
    async fn test_tool_registry_unknown_tool_returns_error() {
        let reg = ToolRegistry::new();
        assert!(reg.call("nonexistent", json!({})).await.is_err());
    }

    #[tokio::test]
    async fn test_tool_registry_list() {
        let mut reg = ToolRegistry::new();
        reg.register(
            ToolDef { name: "ping".to_string(), description: "pong".to_string() },
            Arc::new(|_| Box::pin(async { Ok(json!("pong")) })),
        );
        let tools = reg.list();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "ping");
    }

    #[tokio::test]
    async fn test_resource_registry_register_and_read() {
        let mut reg = ResourceRegistry::new();
        reg.register(
            ResourceDef { uri: "mcp://app/status".to_string(), description: "app status".to_string() },
            Arc::new(|| Box::pin(async { Ok(json!({"status": "ok"})) })),
        );
        let result = reg.read("mcp://app/status").await.unwrap();
        assert_eq!(result["status"], "ok");
    }
}
