pub mod protocol;
pub mod registry;
pub mod server;

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use registry::{ResourceDef, ResourceRegistry, ToolDef, ToolRegistry};
pub use server::{mcp_router, McpState};
