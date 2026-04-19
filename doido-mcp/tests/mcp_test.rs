use axum::body::Body;
use doido_mcp::{
    mcp_router,
    registry::{ResourceDef, ResourceRegistry, ToolDef, ToolRegistry},
    McpState,
};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

fn make_state() -> McpState {
    let mut tools = ToolRegistry::new();
    tools.register(
        ToolDef {
            name: "echo".to_string(),
            description: "echoes input".to_string(),
        },
        Arc::new(|params| Box::pin(async move { Ok(params) })),
    );
    let mut resources = ResourceRegistry::new();
    resources.register(
        ResourceDef {
            uri: "mcp://app/ping".to_string(),
            description: "ping".to_string(),
        },
        Arc::new(|| Box::pin(async { Ok(json!("pong")) })),
    );
    McpState {
        tools: Arc::new(Mutex::new(tools)),
        resources: Arc::new(Mutex::new(resources)),
    }
}

async fn call_mcp(state: McpState, body: serde_json::Value) -> serde_json::Value {
    use tower::ServiceExt;
    let app = mcp_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_tools_list_endpoint() {
    let result = call_mcp(
        make_state(),
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
    )
    .await;
    assert_eq!(result["result"]["tools"][0]["name"], "echo");
}

#[tokio::test]
async fn test_tools_call_endpoint() {
    let result = call_mcp(
        make_state(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"hello":"world"}}})
    ).await;
    assert_eq!(result["result"]["hello"], "world");
}

#[tokio::test]
async fn test_resources_list_endpoint() {
    let result = call_mcp(
        make_state(),
        json!({"jsonrpc":"2.0","id":3,"method":"resources/list"}),
    )
    .await;
    assert_eq!(result["result"]["resources"][0]["uri"], "mcp://app/ping");
}

#[tokio::test]
async fn test_resources_read_endpoint() {
    let result = call_mcp(
        make_state(),
        json!({"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"mcp://app/ping"}}),
    )
    .await;
    assert_eq!(result["result"], "pong");
}

#[tokio::test]
async fn test_unknown_method_returns_error() {
    let result = call_mcp(
        make_state(),
        json!({"jsonrpc":"2.0","id":5,"method":"unknown/method"}),
    )
    .await;
    assert!(result["error"]["code"].as_i64().is_some());
}
