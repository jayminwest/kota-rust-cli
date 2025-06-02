use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<MCPError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// MCP initialization structures
#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<RootsCapability>,
    pub sampling: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RootsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub logging: Option<Value>,
    pub prompts: Option<Value>,
    pub resources: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// Tool call structures
#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Option<Value>,
}

impl MCPResponse {
    pub fn success(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(code: i32, message: String, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(MCPError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

impl ToolContent {
    pub fn text(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
        }
    }
}

impl ToolResult {
    pub fn success(text: String) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: Some(false),
        }
    }

    pub fn error(text: String) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: Some(true),
        }
    }
}