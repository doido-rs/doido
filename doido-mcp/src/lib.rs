pub mod protocol;
pub mod registry;
pub mod server;

pub use doido_mcp_macros::{mcp_resource, mcp_server, resource, tool};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use registry::{ResourceDef, ResourceRegistry, ToolDef, ToolRegistry};
pub use server::{mcp_router, McpState};
