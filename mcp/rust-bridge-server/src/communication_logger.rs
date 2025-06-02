use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::Path;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub direction: Direction,
    pub record_type: CommunicationType,
    pub tool_name: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub request_data: serde_json::Value,
    pub response_data: Option<serde_json::Value>,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationType {
    McpCall,
    HttpRequest,
    HttpResponse,
    WebsocketMessage,
    QueueMessage,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub source: String,
    pub destination: String,
    pub protocol: Protocol,
    pub size_bytes: u64,
    pub duration_ms: Option<u64>,
    pub http_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Mcp,
    Http,
    Websocket,
    File,
}

pub struct CommunicationLogger {
    logs_dir: String,
}

impl CommunicationLogger {
    pub async fn new() -> Result<Self> {
        let logs_dir = "mcp/rust-bridge-server/logs".to_string();
        
        // Create logs directory if it doesn't exist
        fs::create_dir_all(&logs_dir).await?;
        
        Ok(Self { logs_dir })
    }

    pub async fn log_communication(&self, record: CommunicationRecord) -> Result<()> {
        // Daily JSON log file
        let date_str = record.timestamp.format("%Y-%m-%d").to_string();
        let daily_log_path = Path::new(&self.logs_dir).join(format!("communications-{}.json", date_str));
        
        // JSONL stream file
        let jsonl_path = Path::new(&self.logs_dir).join("all-communications.jsonl");

        // Write to daily JSON log
        self.write_to_daily_log(&daily_log_path, &record).await?;
        
        // Append to JSONL stream
        self.append_to_jsonl(&jsonl_path, &record).await?;

        tracing::info!(
            "Logged {} {} communication: {} -> {}",
            record.direction,
            record.record_type,
            record.metadata.source,
            record.metadata.destination
        );

        Ok(())
    }

    async fn write_to_daily_log(&self, path: &Path, record: &CommunicationRecord) -> Result<()> {
        let mut records = if path.exists() {
            let content = fs::read_to_string(path).await?;
            serde_json::from_str::<Vec<CommunicationRecord>>(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        records.push(record.clone());
        let json_content = serde_json::to_string_pretty(&records)?;
        fs::write(path, json_content).await?;

        Ok(())
    }

    async fn append_to_jsonl(&self, path: &Path, record: &CommunicationRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        let line = serde_json::to_string(record)?;
        file.write_all(format!("{}\n", line).as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn get_recent_logs(&self, limit: Option<usize>) -> Result<Vec<CommunicationRecord>> {
        let jsonl_path = Path::new(&self.logs_dir).join("all-communications.jsonl");
        
        if !jsonl_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&jsonl_path).await?;
        let mut records: Vec<CommunicationRecord> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        // Sort by timestamp descending (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if specified
        if let Some(limit) = limit {
            records.truncate(limit);
        }

        Ok(records)
    }

    pub fn create_record(
        &self,
        direction: Direction,
        record_type: CommunicationType,
        tool_name: Option<String>,
        endpoint: Option<String>,
        method: Option<String>,
        request_data: serde_json::Value,
        response_data: Option<serde_json::Value>,
        success: bool,
        error_message: Option<String>,
        source: String,
        destination: String,
        protocol: Protocol,
        duration_ms: Option<u64>,
        http_status: Option<u16>,
    ) -> CommunicationRecord {
        let request_size = serde_json::to_string(&request_data).unwrap_or_default().len() as u64;
        let response_size = response_data
            .as_ref()
            .map(|data| serde_json::to_string(data).unwrap_or_default().len() as u64)
            .unwrap_or(0);

        CommunicationRecord {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            direction,
            record_type,
            tool_name,
            endpoint,
            method,
            request_data,
            response_data,
            success,
            error_message,
            metadata: Metadata {
                source,
                destination,
                protocol,
                size_bytes: request_size + response_size,
                duration_ms,
                http_status,
            },
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Inbound => write!(f, "inbound"),
            Direction::Outbound => write!(f, "outbound"),
        }
    }
}

impl std::fmt::Display for CommunicationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationType::McpCall => write!(f, "mcp_call"),
            CommunicationType::HttpRequest => write!(f, "http_request"),
            CommunicationType::HttpResponse => write!(f, "http_response"),
            CommunicationType::WebsocketMessage => write!(f, "websocket_message"),
            CommunicationType::QueueMessage => write!(f, "queue_message"),
            CommunicationType::Error => write!(f, "error"),
        }
    }
}