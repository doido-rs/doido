use crate::{
    protocol::{JsonRpcRequest, JsonRpcResponse},
    registry::{ResourceRegistry, ToolRegistry},
};
use axum::{extract::State, routing::post, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct McpState {
    pub tools: Arc<Mutex<ToolRegistry>>,
    pub resources: Arc<Mutex<ResourceRegistry>>,
}

pub fn mcp_router(state: McpState) -> Router {
    Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(state)
}

async fn handle_mcp(
    State(state): State<McpState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let resp = dispatch(state, req).await;
    Json(resp)
}

async fn dispatch(state: McpState, req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "tools/list" => {
            let tools = state.tools.lock().await;
            let list: Vec<Value> = tools
                .list()
                .iter()
                .map(|t| json!({"name": t.name, "description": t.description}))
                .collect();
            JsonRpcResponse::ok(req.id, json!({"tools": list}))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let name = params["name"].as_str().unwrap_or("").to_string();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match state.tools.lock().await.call(&name, args).await {
                Ok(result) => JsonRpcResponse::ok(req.id, result),
                Err(e) => JsonRpcResponse::error(req.id, -32603, e.to_string()),
            }
        }
        "resources/list" => {
            let resources = state.resources.lock().await;
            let list: Vec<Value> = resources
                .list()
                .iter()
                .map(|r| json!({"uri": r.uri, "description": r.description}))
                .collect();
            JsonRpcResponse::ok(req.id, json!({"resources": list}))
        }
        "resources/read" => {
            let params = req.params.unwrap_or(json!({}));
            let uri = params["uri"].as_str().unwrap_or("").to_string();
            match state.resources.lock().await.read(&uri).await {
                Ok(result) => JsonRpcResponse::ok(req.id, result),
                Err(e) => JsonRpcResponse::error(req.id, -32603, e.to_string()),
            }
        }
        _ => JsonRpcResponse::error(req.id, -32601, "Method not found"),
    }
}
