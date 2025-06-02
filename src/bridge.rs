use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::context::ContextManager;
use crate::memory::MemoryManager;

#[derive(Clone)]
pub struct BridgeClient {
    client: reqwest::Client,
    bridge_url: String,
    auth_token: String,
    context_manager: Option<Arc<RwLock<ContextManager>>>,
    memory_manager: Option<Arc<RwLock<MemoryManager>>>,
}

impl BridgeClient {
    pub fn new(bridge_host: &str, bridge_port: u16, auth_token: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let bridge_url = format!("http://{}:{}", bridge_host, bridge_port);

        Self {
            client,
            bridge_url,
            auth_token,
            context_manager: None,
            memory_manager: None,
        }
    }

    pub fn with_context_manager(mut self, context_manager: Arc<RwLock<ContextManager>>) -> Self {
        self.context_manager = Some(context_manager);
        self
    }

    pub fn with_memory_manager(mut self, memory_manager: Arc<RwLock<MemoryManager>>) -> Self {
        self.memory_manager = Some(memory_manager);
        self
    }

    pub async fn test_bridge_connection(&self) -> bool {
        match self.call_bridge_api("GET", "/health", None).await {
            Ok(_) => {
                info!("✅ Bridge server connection successful");
                true
            }
            Err(e) => {
                warn!("⚠️  Bridge server connection failed: {}", e);
                false
            }
        }
    }

    pub async fn send_knowledge_to_mac_pro(&self, category: &str, content: &str, metadata: Option<Value>) -> Result<()> {
        let payload = json!({
            "endpoint": "/api/send-knowledge",
            "data": {
                "category": category,
                "content": content,
                "metadata": metadata,
                "source": "kota-rust-cli",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        match self.call_bridge_api("POST", "/api/send-to-mac-pro", Some(payload)).await {
            Ok(_) => {
                info!("📤 Sent knowledge to Mac Pro: category={}", category);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to send knowledge to Mac Pro: {}", e);
                Err(e)
            }
        }
    }

    pub async fn send_context_update_to_mac_pro(&self, context_type: &str, data: Value) -> Result<()> {
        let payload = json!({
            "endpoint": "/api/send-context-update",
            "data": {
                "context_type": context_type,
                "data": data,
                "source": "kota-rust-cli",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        match self.call_bridge_api("POST", "/api/send-to-mac-pro", Some(payload)).await {
            Ok(_) => {
                info!("🔄 Sent context update to Mac Pro: type={}", context_type);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to send context update to Mac Pro: {}", e);
                Err(e)
            }
        }
    }

    pub async fn send_insight_to_mac_pro(&self, category: &str, content: &str, confidence: f64) -> Result<()> {
        let payload = json!({
            "endpoint": "/api/receive-insight",
            "data": {
                "category": category,
                "content": content,
                "confidence": confidence,
                "source": "kota-rust-cli-proactive",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        match self.call_bridge_api("POST", "/api/send-to-mac-pro", Some(payload)).await {
            Ok(_) => {
                info!("💡 Sent insight to Mac Pro: {}", category);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to send insight to Mac Pro: {}", e);
                Err(e)
            }
        }
    }

    pub async fn query_mac_pro_data(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        let payload = json!({
            "endpoint": "/api/query-mcp-data",
            "data": {
                "server_name": server_name,
                "tool_name": tool_name,
                "arguments": arguments
            }
        });

        match self.call_bridge_api("POST", "/api/send-to-mac-pro", Some(payload)).await {
            Ok(response) => {
                info!("📊 Queried Mac Pro data: {}::{}", server_name, tool_name);
                Ok(response)
            }
            Err(e) => {
                error!("❌ Failed to query Mac Pro data: {}", e);
                Err(e)
            }
        }
    }

    pub async fn get_mac_pro_system_status(&self) -> Result<Value> {
        let payload = json!({
            "endpoint": "/api/system-status",
            "data": {}
        });

        self.call_bridge_api("POST", "/api/send-to-mac-pro", Some(payload)).await
    }

    pub async fn sync_context_with_mac_pro(&self) -> Result<()> {
        if let Some(context_manager) = &self.context_manager {
            let context = context_manager.read().await;
            let context_data = context.get_context_summary();
            
            // Send current context to Mac Pro
            self.send_context_update_to_mac_pro("kota_cli_context", context_data).await?;
            
            info!("🔄 Synchronized context with Mac Pro");
        }
        
        Ok(())
    }

    pub async fn sync_memory_with_mac_pro(&self) -> Result<()> {
        if let Some(memory_manager) = &self.memory_manager {
            let memory = memory_manager.read().await;
            
            // Get recent insights from memory
            if let Ok(recent_sessions) = memory.get_recent_conversations(5) {
                for session in recent_sessions {
                    self.send_knowledge_to_mac_pro(
                        "conversation_memory",
                        &session.summary,
                        Some(json!({
                            "session_id": session.id,
                            "timestamp": session.timestamp,
                            "key_insights": session.key_insights
                        }))
                    ).await.ok(); // Don't fail the whole sync if one fails
                }
            }
            
            info!("🧠 Synchronized memory with Mac Pro");
        }
        
        Ok(())
    }

    pub async fn generate_proactive_insights(&self) -> Result<Vec<String>> {
        let mut insights = Vec::new();

        // Analyze current context for insights
        if let Some(context_manager) = &self.context_manager {
            let context = context_manager.read().await;
            let file_count = context.get_file_count();
            
            if file_count > 20 {
                insights.push(format!(
                    "You have {} files in context. Consider clearing context to improve performance.",
                    file_count
                ));
            }
            
            if file_count > 0 {
                insights.push(format!(
                    "Context contains {} files. Good time to analyze patterns or generate documentation.",
                    file_count
                ));
            }
        }

        // Analyze memory patterns
        if let Some(memory_manager) = &self.memory_manager {
            let memory = memory_manager.read().await;
            
            if let Ok(recent_sessions) = memory.get_recent_conversations(10) {
                let session_count = recent_sessions.len();
                if session_count > 5 {
                    insights.push(format!(
                        "You've had {} recent conversations. Consider reviewing key insights for patterns.",
                        session_count
                    ));
                }
            }
        }

        // Time-based insights
        let current_hour = chrono::Utc::now().hour();
        match current_hour {
            9..=11 => {
                insights.push("Peak morning productivity window. Great time for complex coding tasks.".to_string());
            }
            14..=16 => {
                insights.push("Afternoon focus time. Consider tackling analytical or creative work.".to_string());
            }
            17..=19 => {
                insights.push("End of workday approaching. Good time to review progress and plan tomorrow.".to_string());
            }
            _ => {}
        }

        // Send insights to Mac Pro
        for insight in &insights {
            self.send_insight_to_mac_pro("kota_cli_proactive", insight, 0.75).await.ok();
        }

        Ok(insights)
    }

    pub async fn get_bridge_status(&self) -> Result<Value> {
        self.call_bridge_api("GET", "/api/get-rust-status", None).await
    }

    pub async fn get_communication_logs(&self, limit: Option<u32>) -> Result<Value> {
        let endpoint = if let Some(limit) = limit {
            format!("/api/communication-logs?limit={}", limit)
        } else {
            "/api/communication-logs".to_string()
        };

        self.call_bridge_api("GET", &endpoint, None).await
    }

    async fn call_bridge_api(&self, method: &str, endpoint: &str, payload: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.bridge_url, endpoint);
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        request = request.header("Authorization", format!("Bearer {}", self.auth_token));
        request = request.header("User-Agent", "KOTA-CLI/0.1.0");

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

// Integration functions for the main KOTA CLI
pub struct BridgeIntegration {
    client: Option<BridgeClient>,
    enabled: bool,
}

impl BridgeIntegration {
    pub fn new() -> Self {
        Self {
            client: None,
            enabled: false,
        }
    }

    pub async fn initialize(&mut self, bridge_host: Option<&str>, bridge_port: Option<u16>) -> Result<()> {
        let host = bridge_host.unwrap_or("localhost");
        let port = bridge_port.unwrap_or(8081);
        let auth_token = std::env::var("BRIDGE_SECRET")
            .unwrap_or_else(|_| "kota-bridge-secret-2025".to_string());

        let client = BridgeClient::new(host, port, auth_token);
        
        // Test connection
        if client.test_bridge_connection().await {
            self.client = Some(client);
            self.enabled = true;
            info!("🌉 Bridge integration enabled");
            Ok(())
        } else {
            warn!("🌉 Bridge integration disabled - server not available");
            self.enabled = false;
            Ok(())
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn on_context_change(&self, context_data: Value) -> Result<()> {
        if let Some(client) = &self.client {
            client.send_context_update_to_mac_pro("context_change", context_data).await?;
        }
        Ok(())
    }

    pub async fn on_file_added(&self, file_path: &str, content_preview: &str) -> Result<()> {
        if let Some(client) = &self.client {
            client.send_knowledge_to_mac_pro(
                "file_analysis",
                &format!("Added file: {} (preview: {}...)", file_path, &content_preview[..100.min(content_preview.len())]),
                Some(json!({
                    "file_path": file_path,
                    "content_length": content_preview.len(),
                    "action": "file_added"
                }))
            ).await?;
        }
        Ok(())
    }

    pub async fn on_command_executed(&self, command: &str, output: &str, success: bool) -> Result<()> {
        if let Some(client) = &self.client {
            client.send_knowledge_to_mac_pro(
                "command_execution",
                &format!("Executed: {} (success: {})", command, success),
                Some(json!({
                    "command": command,
                    "output_length": output.len(),
                    "success": success,
                    "action": "command_executed"
                }))
            ).await?;
        }
        Ok(())
    }

    pub async fn on_llm_interaction(&self, prompt: &str, response: &str, model: &str) -> Result<()> {
        if let Some(client) = &self.client {
            client.send_knowledge_to_mac_pro(
                "llm_interaction",
                &format!("LLM interaction with {} (prompt: {}...)", model, &prompt[..50.min(prompt.len())]),
                Some(json!({
                    "model": model,
                    "prompt_length": prompt.len(),
                    "response_length": response.len(),
                    "action": "llm_interaction"
                }))
            ).await?;
        }
        Ok(())
    }

    pub async fn periodic_sync(&self) -> Result<()> {
        if let Some(client) = &self.client {
            // Sync context and memory
            client.sync_context_with_mac_pro().await.ok();
            client.sync_memory_with_mac_pro().await.ok();
            
            // Generate and send proactive insights
            client.generate_proactive_insights().await.ok();
            
            debug!("🔄 Completed periodic sync with Mac Pro");
        }
        Ok(())
    }

    pub async fn get_mac_pro_data(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        if let Some(client) = &self.client {
            client.query_mac_pro_data(server_name, tool_name, arguments).await
        } else {
            Err(anyhow::anyhow!("Bridge not enabled"))
        }
    }
}