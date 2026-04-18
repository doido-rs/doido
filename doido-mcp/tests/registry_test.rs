use doido_mcp::registry::{ResourceDef, ResourceRegistry, ToolDef, ToolRegistry};
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
