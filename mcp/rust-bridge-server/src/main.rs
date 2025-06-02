mod server;
mod communication_logger;
mod mac_pro_client;

use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn, error, debug};
use tracing_subscriber;
use tokio::fs;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, EventKind};
use std::sync::mpsc::channel;
use chrono::Timelike;
use serde_json::Value;
use std::net::UdpSocket;

use mac_pro_client::MacProClient;
use communication_logger::CommunicationLogger;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with human-readable format
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)  // Remove module names
        .with_thread_ids(false)  // Remove thread IDs
        .with_line_number(false)  // Remove line numbers
        .with_file(false)  // Remove file names
        .without_time()  // Remove timestamps for cleaner output
        .with_ansi(true)  // Enable colors
        .init();

    info!("Starting KOTA Rust Bridge Server v0.1.0");

    // Log system IP address
    if let Ok(local_ip) = get_local_ip().await {
        info!("Server IP Address: {}", local_ip);
    } else {
        warn!("Could not determine local IP address");
    }

    // Load configuration from environment variables
    let config = load_configuration();
    
    info!("Configuration loaded:");
    info!("  Mac Pro Host: {}", config.mac_pro_host);
    info!("  Rust Port: {}", config.rust_port);
    info!("  Queue Polling: {}", config.enable_queue_polling);
    info!("  Using separate auth tokens for MCP and Mac Pro");

    // Initialize communication logger
    let comm_logger = match CommunicationLogger::new().await {
        Ok(logger) => {
            info!("Communication logger initialized");
            std::sync::Arc::new(logger)
        }
        Err(e) => {
            error!("Failed to initialize communication logger: {}", e);
            return Err(e);
        }
    };

    // Initialize Mac Pro client with Mac Pro's secret
    let mac_pro_client = std::sync::Arc::new(MacProClient::new(
        &config.mac_pro_host,
        8080, // Mac Pro bridge server port
        config.mac_pro_secret.clone(),
        comm_logger.clone()
    ));

    // Test connection to Mac Pro
    info!("Testing connection to Mac Pro bridge server...");
    match mac_pro_client.test_connection().await {
        true => {
            info!("✅ Successfully connected to Mac Pro bridge server at {}", 
                  mac_pro_client.get_base_url());
        }
        false => {
            warn!("⚠️  Failed to connect to Mac Pro bridge server at {}", 
                  mac_pro_client.get_base_url());
            warn!("   The server will start anyway and retry connections automatically");
            
            // Try network discovery as fallback (with timeout)
            info!("Attempting automatic network discovery...");
            let discovery_task = tokio::time::timeout(
                Duration::from_secs(5), // 5 second timeout
                MacProClient::discover()
            ).await;
            
            match discovery_task {
                Ok(Ok(discovered_ip)) => {
                    info!("🔍 Discovered Mac Pro at {}", discovered_ip);
                    // Note: In production, you might want to recreate the client with the discovered IP
                }
                Ok(Err(e)) => {
                    warn!("Network discovery failed: {}", e);
                }
                Err(_) => {
                    warn!("Network discovery timed out after 5 seconds");
                }
            }
        }
    }

    // Start the HTTP server in a background task
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        info!("Starting HTTP server on port {}", server_config.rust_port);
        if let Err(e) = server::create_server(
            server_config.rust_port,
            server_config.mac_pro_host,
            server_config.shared_secret
        ).await {
            error!("❌ HTTP server error: {}", e);
        }
    });

    // Start queue polling if enabled
    let queue_handle = if config.enable_queue_polling {
        let queue_config = config.clone();
        Some(tokio::spawn(async move {
            info!("Starting queue polling for Mac Pro messages");
            if let Err(e) = poll_mac_pro_queue(&queue_config.queue_directory).await {
                error!("❌ Queue polling error: {}", e);
            }
        }))
    } else {
        info!("Queue polling disabled");
        None
    };

    // Start periodic health checks and proactive monitoring
    let health_check_client = mac_pro_client.clone();
    let health_handle = tokio::spawn(async move {
        periodic_health_checks(health_check_client).await;
    });

    // Start proactive insight generation
    let insight_client = mac_pro_client.clone();
    let insight_handle = tokio::spawn(async move {
        proactive_insight_generation(insight_client).await;
    });

    // Start message polling from Mac Pro
    let message_client = mac_pro_client.clone();
    let message_handle = tokio::spawn(async move {
        poll_mac_pro_messages(message_client).await;
    });

    info!("🚀 KOTA Rust Bridge Server is running!");
    info!("   HTTP Server: http://0.0.0.0:{}", config.rust_port);
    info!("   Health Check: http://0.0.0.0:{}/health", config.rust_port);
    info!("   Discovery: http://0.0.0.0:{}/discovery", config.rust_port);

    // Wait for all tasks to complete (they should run indefinitely)
    tokio::select! {
        result = server_handle => {
            if let Err(e) = result {
                error!("Server task panicked: {}", e);
            }
        }
        result = health_handle => {
            if let Err(e) = result {
                error!("Health check task panicked: {}", e);
            }
        }
        result = insight_handle => {
            if let Err(e) = result {
                error!("Insight generation task panicked: {}", e);
            }
        }
        result = async {
            if let Some(handle) = queue_handle {
                handle.await
            } else {
                // If queue polling is disabled, return Ok and let other tasks run
                std::future::pending().await
            }
        } => {
            if let Err(e) = result {
                error!("Queue polling task panicked: {}", e);
            }
        }
        result = message_handle => {
            if let Err(e) = result {
                error!("Message polling task panicked: {}", e);
            }
        }
    }

    warn!("🛑 KOTA Rust Bridge Server shutting down");
    Ok(())
}

async fn get_local_ip() -> Result<String> {
    // Use a UDP socket to determine the local IP address
    // This method connects to a remote address but doesn't actually send data
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?; // Connect to Google DNS
    
    if let Ok(local_addr) = socket.local_addr() {
        Ok(local_addr.ip().to_string())
    } else {
        // Fallback: try to get the hostname and resolve it
        match hostname::get() {
            Ok(hostname) => {
                if let Some(hostname_str) = hostname.to_str() {
                    Ok(format!("{} (hostname)", hostname_str))
                } else {
                    Ok("localhost".to_string())
                }
            }
            Err(_) => Ok("localhost".to_string())
        }
    }
}

#[derive(Clone)]
struct Configuration {
    mac_pro_host: String,
    rust_port: u16,
    shared_secret: String,  // Secret for incoming requests from MCP
    mac_pro_secret: String, // Secret for outgoing requests to Mac Pro
    enable_queue_polling: bool,
    queue_directory: String,
}

fn load_configuration() -> Configuration {
    // Try to load .env file
    dotenv::dotenv().ok();

    Configuration {
        mac_pro_host: std::env::var("MAC_PRO_HOST")
            .unwrap_or_else(|_| "100.118.223.57".to_string()),
        rust_port: std::env::var("RUST_CLI_PORT")
            .unwrap_or_else(|_| "8081".to_string())
            .parse::<u16>()
            .unwrap_or(8081),
        shared_secret: std::env::var("BRIDGE_SECRET")
            .unwrap_or_else(|_| "kota-bridge-secret-2025".to_string()),
        mac_pro_secret: std::env::var("MAC_PRO_SECRET")
            .unwrap_or_else(|_| "default-secret-change-me".to_string()),
        enable_queue_polling: std::env::var("ENABLE_QUEUE_POLLING")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false),
        queue_directory: std::env::var("MAC_PRO_QUEUE_DIR")
            .unwrap_or_else(|_| "/Users/jaymin/kota_md/core/mcp/data/rust-cli-queue".to_string()),
    }
}

async fn poll_mac_pro_queue(queue_dir: &str) -> Result<()> {
    // Check if the queue directory exists
    if !std::path::Path::new(queue_dir).exists() {
        warn!("Queue directory does not exist: {}", queue_dir);
        warn!("Skipping queue polling. Enable network file sharing or use HTTP polling instead.");
        return Ok(());
    }

    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })?;
    watcher.watch(std::path::Path::new(queue_dir), RecursiveMode::NonRecursive)?;

    info!("👀 Watching queue directory: {}", queue_dir);

    loop {
        match rx.recv() {
            Ok(event_result) => {
                match event_result {
                    Ok(event) => {
                        debug!("File system event: {:?}", event);
                        if let EventKind::Create(_) = event.kind {
                            for path in event.paths {
                                if let Some(extension) = path.extension() {
                                    if extension == "json" {
                                        info!("📥 Processing queued message: {:?}", path);
                                        if let Err(e) = process_queued_message(&path).await {
                                            error!("Failed to process queued message: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("File system event error: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("File watcher channel error: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_queued_message(path: &std::path::Path) -> Result<()> {
    let content = fs::read_to_string(path).await?;
    let message: Value = serde_json::from_str(&content)?;
    
    info!("Processing message: {}", message["type"].as_str().unwrap_or("unknown"));

    match message["type"].as_str() {
        Some("knowledge_update") => {
            let category = message["category"].as_str().unwrap_or("unknown");
            let content = message["content"].as_str().unwrap_or("");
            info!("📚 Processing knowledge update: category={}, length={}", category, content.len());
            
            // Process the knowledge update
            // This could involve storing in local knowledge base, analyzing patterns, etc.
        }
        Some("context_update") => {
            let context_type = message["context_type"].as_str().unwrap_or("unknown");
            info!("🔄 Processing context update: type={}", context_type);
            
            // Process the context update
            // This could involve updating local context state, triggering analysis, etc.
        }
        Some("insight_request") => {
            info!("💡 Processing insight request");
            
            // Generate insights based on current data
            // This is where proactive analysis would happen
        }
        _ => {
            warn!("❓ Unknown message type in queue: {:?}", message["type"]);
        }
    }

    // Delete the processed file
    if let Err(e) = fs::remove_file(path).await {
        warn!("Failed to delete processed queue file: {}", e);
    } else {
        debug!("✅ Deleted processed queue file: {:?}", path);
    }

    Ok(())
}

async fn periodic_health_checks(mac_pro_client: std::sync::Arc<MacProClient>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        debug!("🩺 Performing periodic health check");
        
        let is_healthy = mac_pro_client.test_connection().await;
        if !is_healthy {
            warn!("⚠️  Mac Pro connection is unhealthy, attempting reconnection");
            
            // Could implement exponential backoff here
            tokio::time::sleep(Duration::from_secs(5)).await;
        } else {
            debug!("✅ Mac Pro connection is healthy");
        }
    }
}

async fn proactive_insight_generation(mac_pro_client: std::sync::Arc<MacProClient>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
    
    loop {
        interval.tick().await;
        
        debug!("🧠 Generating proactive insights");
        
        // Example proactive insights based on time of day, patterns, etc.
        let current_hour = chrono::Utc::now().hour();
        
        match current_hour {
            9..=11 => {
                // Morning productivity window
                if let Err(e) = mac_pro_client.send_insight(
                    "productivity",
                    "Peak productivity window detected. Consider tackling high-focus tasks now.",
                    0.85,
                    "rust-proactive-monitor"
                ).await {
                    debug!("Failed to send morning productivity insight: {}", e);
                }
            }
            14..=16 => {
                // Afternoon productivity window
                if let Err(e) = mac_pro_client.send_insight(
                    "productivity",
                    "Afternoon productivity window. Good time for creative or analytical work.",
                    0.82,
                    "rust-proactive-monitor"
                ).await {
                    debug!("Failed to send afternoon productivity insight: {}", e);
                }
            }
            17..=19 => {
                // End of day wrap-up
                if let Err(e) = mac_pro_client.send_insight(
                    "schedule",
                    "Consider reviewing today's accomplishments and planning tomorrow's priorities.",
                    0.75,
                    "rust-proactive-monitor"
                ).await {
                    debug!("Failed to send end-of-day insight: {}", e);
                }
            }
            _ => {
                // Other times - send general insights less frequently
                if current_hour % 3 == 0 { // Every 3 hours
                    if let Err(e) = mac_pro_client.send_insight(
                        "general",
                        "Regular check-in: How are your energy levels and focus?",
                        0.60,
                        "rust-proactive-monitor"
                    ).await {
                        debug!("Failed to send general insight: {}", e);
                    }
                }
            }
        }
    }
}

async fn poll_mac_pro_messages(mac_pro_client: std::sync::Arc<MacProClient>) {
    let mut interval = tokio::time::interval(Duration::from_secs(10)); // Poll every 10 seconds
    
    loop {
        interval.tick().await;
        
        debug!("📨 Polling Mac Pro for messages");
        
        // Try to get messages from the Mac Pro
        // First, let's try to discover what endpoints are available
        match mac_pro_client.call_api("GET", "/api/outbound/queue", None).await {
            Ok(response) => {
                if let Some(messages) = response.get("messages").and_then(|m| m.as_array()) {
                    if !messages.is_empty() {
                        info!("📥 Found {} messages from Mac Pro", messages.len());
                        for message in messages {
                            process_mac_pro_message(message).await;
                        }
                    }
                }
            }
            Err(e) => {
                debug!("No messages available or endpoint not accessible: {}", e);
                // Try alternative endpoint
                if let Ok(response) = mac_pro_client.call_api("GET", "/api/messages/pending", None).await {
                    if let Some(messages) = response.get("messages").and_then(|m| m.as_array()) {
                        if !messages.is_empty() {
                            info!("📥 Found {} pending messages from Mac Pro", messages.len());
                            for message in messages {
                                process_mac_pro_message(message).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn process_mac_pro_message(message: &Value) {
    let msg_type = message.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    
    let content = message.get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    
    info!("📬 Processing Mac Pro message: type={}, content_length={}", msg_type, content.len());
    
    match msg_type {
        "collaboration_message" => {
            info!("💬 Collaboration message from Mac Pro: {}", content);
            // Here you would handle the collaboration message
            // For now, just log it
        }
        "insight" => {
            info!("💡 Insight from Mac Pro: {}", content);
            // Process insight
        }
        "context_update" => {
            info!("🔄 Context update from Mac Pro");
            // Process context update
        }
        _ => {
            warn!("❓ Unknown message type from Mac Pro: {}", msg_type);
        }
    }
}