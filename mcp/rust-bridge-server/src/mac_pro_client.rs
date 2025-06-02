use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use anyhow::Result;
use tracing::{info, warn, error};
use crate::communication_logger::{
    CommunicationLogger, Direction, CommunicationType, Protocol
};
use std::sync::Arc;

pub struct MacProClient {
    client: Client,
    base_url: String,
    auth_token: String,
    comm_logger: Arc<CommunicationLogger>,
}

impl MacProClient {
    pub fn new(host: &str, port: u16, auth_token: String, comm_logger: Arc<CommunicationLogger>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        let base_url = format!("http://{}:{}", host, port);
        
        Self {
            client,
            base_url,
            auth_token,
            comm_logger,
        }
    }

    pub async fn discover() -> Result<String> {
        info!("Discovering Mac Pro bridge server on local network...");
        
        // Try common local network ranges
        let ranges = vec![
            "192.168.1", "192.168.0", "10.0.0", "172.16.0"
        ];

        for range in ranges {
            for i in 1..=254 {
                let ip = format!("{}.{}", range, i);
                if let Ok(response) = Self::health_check_ip(&ip).await {
                    if response.contains("kota-bridge") {
                        info!("Found Mac Pro bridge server at {}", ip);
                        return Ok(ip);
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Mac Pro bridge server not found on local network"))
    }

    async fn health_check_ip(ip: &str) -> Result<String> {
        let client = Client::builder()
            .timeout(Duration::from_millis(500))
            .build()?;
            
        let url = format!("http://{}:8080/health", ip);
        let response = client.get(&url).send().await?;
        Ok(response.text().await?)
    }

    pub async fn health_check(&self) -> Result<Value> {
        // Health check endpoint may not require authentication
        self.call_api_no_auth("GET", "/health", None).await
    }

    pub async fn send_knowledge(&self, category: &str, content: &str, metadata: Option<Value>) -> Result<Value> {
        let payload = serde_json::json!({
            "category": category,
            "content": content,
            "metadata": metadata,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/send-knowledge", Some(payload)).await
    }

    pub async fn send_context_update(&self, context_type: &str, data: Value) -> Result<Value> {
        let payload = serde_json::json!({
            "context_type": context_type,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/send-context-update", Some(payload)).await
    }

    pub async fn query_mcp_data(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        let payload = serde_json::json!({
            "server_name": server_name,
            "tool_name": tool_name,
            "arguments": arguments
        });

        self.call_api("POST", "/api/query-mcp-data", Some(payload)).await
    }

    pub async fn get_system_status(&self) -> Result<Value> {
        self.call_api("GET", "/api/system-status", None).await
    }

    pub async fn send_insight(&self, category: &str, content: &str, confidence: f64, source: &str) -> Result<Value> {
        let payload = serde_json::json!({
            "category": category,
            "content": content,
            "confidence": confidence,
            "source": source,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.call_api("POST", "/api/receive-insight", Some(payload)).await
    }

    pub async fn get_communication_logs(&self, limit: Option<u32>) -> Result<Value> {
        let endpoint = if let Some(limit) = limit {
            format!("/api/communication-logs?limit={}", limit)
        } else {
            "/api/communication-logs".to_string()
        };

        self.call_api("GET", &endpoint, None).await
    }

    pub async fn get_communication_stats(&self) -> Result<Value> {
        self.call_api("GET", "/api/communication-stats", None).await
    }

    pub async fn call_api_no_auth(&self, method: &str, endpoint: &str, payload: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let start_time = Instant::now();
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        // No authentication for this call
        request = request.header("User-Agent", "KOTA-Rust-CLI/0.1.0");

        if let Some(payload) = &payload {
            request = request.json(payload);
        }

        // Log the outbound request
        let record = self.comm_logger.create_record(
            Direction::Outbound,
            CommunicationType::HttpRequest,
            None,
            Some(endpoint.to_string()),
            Some(method.to_string()),
            payload.clone().unwrap_or(serde_json::json!({})),
            None,
            false, // Will be updated after response
            None,
            "rust-bridge-server".to_string(),
            "mac-pro-bridge".to_string(),
            Protocol::Http,
            None, // Will be updated after response
            None, // Will be updated after response
        );

        let response = request.send().await;
        let duration = start_time.elapsed();

        match response {
            Ok(resp) => {
                let status = resp.status();
                let success = status.is_success();
                
                match resp.text().await {
                    Ok(text) => {
                        match serde_json::from_str::<Value>(&text) {
                            Ok(json) => {
                                // Log successful communication
                                let mut final_record = record;
                                final_record.success = success;
                                final_record.response_data = Some(json.clone());
                                final_record.metadata.duration_ms = Some(duration.as_millis() as u64);
                                final_record.metadata.http_status = Some(status.as_u16());

                                if let Err(e) = self.comm_logger.log_communication(final_record).await {
                                    warn!("Failed to log communication: {}", e);
                                }

                                if success {
                                    Ok(json)
                                } else {
                                    Err(anyhow::anyhow!("HTTP error {}: {}", status, text))
                                }
                            }
                            Err(_) => {
                                // Not JSON, treat as plain text response
                                let response_value = serde_json::json!({"text": text});
                                
                                // Log successful communication
                                let mut final_record = record;
                                final_record.success = success;
                                final_record.response_data = Some(response_value.clone());
                                final_record.metadata.duration_ms = Some(duration.as_millis() as u64);
                                final_record.metadata.http_status = Some(status.as_u16());

                                if let Err(e) = self.comm_logger.log_communication(final_record).await {
                                    warn!("Failed to log communication: {}", e);
                                }

                                if success {
                                    Ok(response_value)
                                } else {
                                    Err(anyhow::anyhow!("HTTP error {}: {}", status, text))
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to read response body: {}", e);
                        
                        // Log failed communication
                        let mut final_record = record;
                        final_record.success = false;
                        final_record.error_message = Some(error_msg.clone());
                        final_record.metadata.duration_ms = Some(duration.as_millis() as u64);
                        final_record.metadata.http_status = Some(status.as_u16());

                        if let Err(e) = self.comm_logger.log_communication(final_record).await {
                            warn!("Failed to log communication: {}", e);
                        }

                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("HTTP request failed: {}", e);
                error!("Mac Pro API call failed: {}", error_msg);

                // Log failed communication
                let mut final_record = record;
                final_record.success = false;
                final_record.error_message = Some(error_msg.clone());
                final_record.metadata.duration_ms = Some(duration.as_millis() as u64);

                if let Err(e) = self.comm_logger.log_communication(final_record).await {
                    warn!("Failed to log communication: {}", e);
                }

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    pub async fn call_api(&self, method: &str, endpoint: &str, payload: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let start_time = Instant::now();
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        request = request.header("Authorization", format!("Bearer {}", self.auth_token));
        request = request.header("User-Agent", "KOTA-Rust-CLI/0.1.0");

        if let Some(payload) = &payload {
            request = request.json(payload);
        }

        // Log the outbound request
        let record = self.comm_logger.create_record(
            Direction::Outbound,
            CommunicationType::HttpRequest,
            None,
            Some(endpoint.to_string()),
            Some(method.to_string()),
            payload.clone().unwrap_or(serde_json::json!({})),
            None,
            false, // Will be updated after response
            None,
            "rust-bridge-server".to_string(),
            "mac-pro-bridge".to_string(),
            Protocol::Http,
            None, // Will be updated after response
            None, // Will be updated after response
        );

        let response = request.send().await;
        let duration = start_time.elapsed();

        match response {
            Ok(resp) => {
                let status = resp.status();
                let success = status.is_success();
                
                match resp.text().await {
                    Ok(text) => {
                        let response_data: Value = serde_json::from_str(&text)
                            .unwrap_or_else(|_| serde_json::json!({"raw_response": text}));

                        // Log successful communication
                        let mut final_record = record;
                        final_record.response_data = Some(response_data.clone());
                        final_record.success = success;
                        final_record.metadata.duration_ms = Some(duration.as_millis() as u64);
                        final_record.metadata.http_status = Some(status.as_u16());

                        if let Err(e) = self.comm_logger.log_communication(final_record).await {
                            warn!("Failed to log communication: {}", e);
                        }

                        if success {
                            Ok(response_data)
                        } else {
                            Err(anyhow::anyhow!("API error {}: {}", status, text))
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to read response body: {}", e);
                        
                        // Log failed communication
                        let mut final_record = record;
                        final_record.success = false;
                        final_record.error_message = Some(error_msg.clone());
                        final_record.metadata.duration_ms = Some(duration.as_millis() as u64);
                        final_record.metadata.http_status = Some(status.as_u16());

                        if let Err(e) = self.comm_logger.log_communication(final_record).await {
                            warn!("Failed to log communication: {}", e);
                        }

                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("HTTP request failed: {}", e);
                error!("Mac Pro API call failed: {}", error_msg);

                // Log failed communication
                let mut final_record = record;
                final_record.success = false;
                final_record.error_message = Some(error_msg.clone());
                final_record.metadata.duration_ms = Some(duration.as_millis() as u64);

                if let Err(e) = self.comm_logger.log_communication(final_record).await {
                    warn!("Failed to log communication: {}", e);
                }

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    pub async fn test_connection(&self) -> bool {
        match self.health_check().await {
            Ok(response) => {
                info!("Mac Pro connection test successful: {:?}", response);
                true
            }
            Err(e) => {
                warn!("Mac Pro connection test failed: {}", e);
                false
            }
        }
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}