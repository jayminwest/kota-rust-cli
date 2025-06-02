use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tracing::{info, warn, error, debug};

mod mcp_protocol;
mod bridge_client;
mod tools;

use mcp_protocol::*;
use bridge_client::BridgeClient;
use tools::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is reserved for MCP communication)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🌉 Starting KOTA MCP Server for Claude Code");

    // Load configuration
    dotenv::dotenv().ok();
    
    let bridge_host = std::env::var("BRIDGE_HOST").unwrap_or_else(|_| "localhost".to_string());
    let bridge_port = std::env::var("BRIDGE_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse::<u16>()
        .unwrap_or(8081);
    let bridge_secret = std::env::var("BRIDGE_SECRET")
        .unwrap_or_else(|_| "kota-bridge-secret-2025".to_string());

    // Initialize bridge client
    let bridge_client = BridgeClient::new(&bridge_host, bridge_port, &bridge_secret).await?;
    
    // Test bridge connection
    if bridge_client.test_connection().await {
        info!("✅ Connected to KOTA bridge server at {}:{}", bridge_host, bridge_port);
    } else {
        warn!("⚠️  Bridge server not available - some features will be limited");
    }

    // Create MCP server
    let mut server = MCPServer::new(bridge_client);
    
    info!("🚀 KOTA MCP Server ready for Claude Code");
    info!("   Use this server in Claude Code's MCP configuration");
    
    // Handle MCP communication over stdio
    server.run_stdio().await?;

    Ok(())
}

pub struct MCPServer {
    bridge_client: BridgeClient,
    tools: Vec<ToolDefinition>,
}

impl MCPServer {
    pub fn new(bridge_client: BridgeClient) -> Self {
        let tools = vec![
            // Bridge communication tools
            ToolDefinition {
                name: "send_to_mac_pro".to_string(),
                description: "Send data or commands to the Mac Pro system through the bridge".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Category of data (knowledge, context, insight, command)"
                        },
                        "content": {
                            "type": "string", 
                            "description": "The content to send"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "Optional metadata"
                        }
                    },
                    "required": ["category", "content"]
                }),
            },
            ToolDefinition {
                name: "query_mac_pro_data".to_string(),
                description: "Query data from Mac Pro MCP servers (calendar, finance, etc.)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "server_name": {
                            "type": "string",
                            "description": "Name of the MCP server to query (e.g., google-calendar, plaid-finance)"
                        },
                        "tool_name": {
                            "type": "string",
                            "description": "Name of the tool to call"
                        },
                        "arguments": {
                            "type": "object",
                            "description": "Arguments to pass to the tool"
                        }
                    },
                    "required": ["server_name", "tool_name"]
                }),
            },
            ToolDefinition {
                name: "get_mac_pro_status".to_string(),
                description: "Get system status and health information from Mac Pro".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "get_bridge_logs".to_string(),
                description: "Get communication logs from the bridge server".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of log entries to return",
                            "default": 50
                        }
                    }
                }),
            },
            // Local system tools
            ToolDefinition {
                name: "analyze_kota_context".to_string(),
                description: "Analyze current KOTA CLI context and generate insights".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "analysis_type": {
                            "type": "string",
                            "enum": ["files", "commands", "patterns", "summary"],
                            "description": "Type of analysis to perform"
                        }
                    },
                    "required": ["analysis_type"]
                }),
            },
            ToolDefinition {
                name: "send_proactive_insight".to_string(),
                description: "Send a proactive insight to Mac Pro for context awareness".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "insight": {
                            "type": "string",
                            "description": "The insight or recommendation to send"
                        },
                        "confidence": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "description": "Confidence level of the insight (0-1)"
                        },
                        "category": {
                            "type": "string",
                            "description": "Category of insight (productivity, schedule, analysis, etc.)"
                        }
                    },
                    "required": ["insight", "confidence"]
                }),
            },
        ];

        Self {
            bridge_client,
            tools,
        }
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = AsyncBufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    debug!("Received: {}", trimmed);

                    match self.handle_request(trimmed).await {
                        Ok(response) => {
                            let response_str = serde_json::to_string(&response)?;
                            stdout.write_all(response_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                            debug!("Sent: {}", response_str);
                        }
                        Err(e) => {
                            error!("Error handling request: {}", e);
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32603,
                                    "message": format!("Internal error: {}", e)
                                },
                                "id": null
                            });
                            let error_str = serde_json::to_string(&error_response)?;
                            stdout.write_all(error_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        info!("MCP Server shutting down");
        Ok(())
    }

    async fn handle_request(&mut self, request_str: &str) -> Result<Value> {
        let request: MCPRequest = serde_json::from_str(request_str)?;

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "notifications/initialized" => Ok(json!({})), // No response needed for notifications
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            _ => Ok(json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", request.method)
                },
                "id": request.id
            }))
        }
    }

    async fn handle_initialize(&self, request: MCPRequest) -> Result<Value> {
        info!("Handling initialize request");
        
        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "logging": {}
                },
                "serverInfo": {
                    "name": "kota-mcp-server",
                    "version": "0.1.0"
                }
            },
            "id": request.id
        }))
    }

    async fn handle_tools_list(&self, request: MCPRequest) -> Result<Value> {
        debug!("Listing available tools");

        let tools: Vec<Value> = self.tools.iter().map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema
            })
        }).collect();

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": tools
            },
            "id": request.id
        }))
    }

    async fn handle_tools_call(&mut self, request: MCPRequest) -> Result<Value> {
        let params = request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
        let tool_name = params["name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
        let arguments = params["arguments"].clone();

        info!("Calling tool: {} with args: {:?}", tool_name, arguments);

        let result = match tool_name {
            "send_to_mac_pro" => self.handle_send_to_mac_pro(arguments).await?,
            "query_mac_pro_data" => self.handle_query_mac_pro_data(arguments).await?,
            "get_mac_pro_status" => self.handle_get_mac_pro_status().await?,
            "get_bridge_logs" => self.handle_get_bridge_logs(arguments).await?,
            "analyze_kota_context" => self.handle_analyze_kota_context(arguments).await?,
            "send_proactive_insight" => self.handle_send_proactive_insight(arguments).await?,
            _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": result
                    }
                ]
            },
            "id": request.id
        }))
    }

    async fn handle_send_to_mac_pro(&self, arguments: Value) -> Result<String> {
        let category = arguments["category"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing category"))?;
        let content = arguments["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
        let metadata = arguments.get("metadata").cloned();

        match category {
            "knowledge" => {
                self.bridge_client.send_knowledge(category, content, metadata).await?;
                Ok(format!("✅ Sent knowledge to Mac Pro: {}", content))
            }
            "context" => {
                self.bridge_client.send_context_update(category, content.into(), metadata).await?;
                Ok(format!("✅ Sent context update to Mac Pro: {}", content))
            }
            "insight" => {
                let confidence = arguments["confidence"].as_f64().unwrap_or(0.8);
                self.bridge_client.send_insight(category, content, confidence).await?;
                Ok(format!("✅ Sent insight to Mac Pro: {}", content))
            }
            _ => {
                // Generic send
                self.bridge_client.send_knowledge(category, content, metadata).await?;
                Ok(format!("✅ Sent {} to Mac Pro: {}", category, content))
            }
        }
    }

    async fn handle_query_mac_pro_data(&self, arguments: Value) -> Result<String> {
        let server_name = arguments["server_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing server_name"))?;
        let tool_name = arguments["tool_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tool_name"))?;
        let tool_arguments = arguments.get("arguments").cloned().unwrap_or(json!({}));

        let result = self.bridge_client.query_mcp_data(server_name, tool_name, tool_arguments).await?;
        Ok(format!("📊 Data from {}::{}: {}", server_name, tool_name, 
                  serde_json::to_string_pretty(&result)?))
    }

    async fn handle_get_mac_pro_status(&self) -> Result<String> {
        let status = self.bridge_client.get_system_status().await?;
        Ok(format!("🖥️  Mac Pro Status: {}", serde_json::to_string_pretty(&status)?))
    }

    async fn handle_get_bridge_logs(&self, arguments: Value) -> Result<String> {
        let limit = arguments.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
        let logs = self.bridge_client.get_communication_logs(limit).await?;
        Ok(format!("📋 Bridge Communication Logs: {}", serde_json::to_string_pretty(&logs)?))
    }

    async fn handle_analyze_kota_context(&self, arguments: Value) -> Result<String> {
        let analysis_type = arguments["analysis_type"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing analysis_type"))?;

        // This would integrate with the main KOTA CLI context in a real implementation
        // For now, we'll simulate the analysis
        match analysis_type {
            "files" => Ok("📁 Context Analysis: Currently analyzing file structure and dependencies".to_string()),
            "commands" => Ok("⚡ Command Analysis: Recent command patterns show focus on development workflow".to_string()),
            "patterns" => Ok("🔍 Pattern Analysis: Detected productivity patterns and optimization opportunities".to_string()),
            "summary" => Ok("📊 Context Summary: Active development session with multiple file contexts and command history".to_string()),
            _ => Err(anyhow::anyhow!("Unknown analysis type: {}", analysis_type))
        }
    }

    async fn handle_send_proactive_insight(&self, arguments: Value) -> Result<String> {
        let insight = arguments["insight"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing insight"))?;
        let confidence = arguments["confidence"].as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing confidence"))?;
        let category = arguments["category"].as_str().unwrap_or("general");

        self.bridge_client.send_insight(category, insight, confidence).await?;
        Ok(format!("💡 Sent proactive insight to Mac Pro: {} (confidence: {:.2})", insight, confidence))
    }
}