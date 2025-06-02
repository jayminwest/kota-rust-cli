use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn, error};

pub struct BridgeClient {
    client: Client,
    base_url: String,
    auth_token: String,
}

impl BridgeClient {
    pub async fn new(host: &str, port: u16, auth_token: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let base_url = format!("http://{}:{}", host, port);

        let bridge_client = Self {
            client,
            base_url,
            auth_token: auth_token.to_string(),
        };

        Ok(bridge_client)
    }

    pub async fn test_connection(&self) -> bool {
        match self.call_api("GET", "/health", None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn send_knowledge(&self, category: &str, content: &str, metadata: Option<Value>) -> Result<Value> {
        let payload = json!({
            "category": category,
            "content": content,
            "metadata": metadata,
            "source": "claude-code-mcp",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/send-knowledge", Some(payload)).await
    }

    pub async fn send_context_update(&self, context_type: &str, data: Value, metadata: Option<Value>) -> Result<Value> {
        let payload = json!({
            "context_type": context_type,
            "data": data,
            "metadata": metadata,
            "source": "claude-code-mcp",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/send-context-update", Some(payload)).await
    }

    pub async fn send_insight(&self, category: &str, content: &str, confidence: f64) -> Result<Value> {
        let payload = json!({
            "category": category,
            "content": content,
            "confidence": confidence,
            "source": "claude-code-mcp",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/receive-insight", Some(payload)).await
    }

    pub async fn query_mcp_data(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        let payload = json!({
            "endpoint": "/api/send-to-mac-pro",
            "data": {
                "endpoint": "/api/query-mcp-data",
                "data": {
                    "server_name": server_name,
                    "tool_name": tool_name,
                    "arguments": arguments
                }
            }
        });

        self.call_api("POST", "/api/send-to-mac-pro", Some(payload)).await
    }

    pub async fn get_system_status(&self) -> Result<Value> {
        let payload = json!({
            "endpoint": "/api/send-to-mac-pro",
            "data": {
                "endpoint": "/api/system-status",
                "data": {}
            }
        });

        self.call_api("POST", "/api/send-to-mac-pro", Some(payload)).await
    }

    pub async fn get_communication_logs(&self, limit: Option<u32>) -> Result<Value> {
        let endpoint = if let Some(limit) = limit {
            format!("/api/communication-logs?limit={}", limit)
        } else {
            "/api/communication-logs".to_string()
        };

        self.call_api("GET", &endpoint, None).await
    }

    // New outbound endpoints for receiving data from Mac Pro
    pub async fn get_outbound_knowledge(&self) -> Result<Value> {
        self.call_api("GET", "/api/outbound/knowledge", None).await
    }

    pub async fn get_outbound_context(&self) -> Result<Value> {
        self.call_api("GET", "/api/outbound/context", None).await
    }

    pub async fn get_outbound_insights(&self) -> Result<Value> {
        self.call_api("GET", "/api/outbound/insights", None).await
    }

    async fn call_api(&self, method: &str, endpoint: &str, payload: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        request = request.header("Authorization", format!("Bearer {}", self.auth_token));
        request = request.header("User-Agent", "KOTA-MCP-Server/0.1.0");

        if let Some(payload) = payload {
            request = request.json(&payload);
        }

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            let json: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| json!({"raw_response": text}));
            Ok(json)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Bridge API error {}: {}", status, error_text))
        }
    }
}