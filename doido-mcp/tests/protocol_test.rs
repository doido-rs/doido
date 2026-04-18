use doido_mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::json;

#[test]
fn test_request_deserializes_from_json() {
    let s = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":null}"#;
    let req: JsonRpcRequest = serde_json::from_str(s).unwrap();
    assert_eq!(req.method, "tools/list");
    assert_eq!(req.id, json!(1));
}

#[test]
fn test_response_ok_serializes() {
    let resp = JsonRpcResponse::ok(json!(1), json!({"tools": []}));
    let s = serde_json::to_string(&resp).unwrap();
    assert!(s.contains("tools"));
    assert!(!s.contains("error"));
}

#[test]
fn test_response_error_serializes() {
    let resp = JsonRpcResponse::error(json!(1), -32601, "Method not found");
    let s = serde_json::to_string(&resp).unwrap();
    assert!(s.contains("Method not found"));
    assert!(!s.contains("result"));
}
