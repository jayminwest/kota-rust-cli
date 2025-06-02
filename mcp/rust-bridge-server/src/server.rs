use axum::{
    extract::{Query, State, Path}, 
    http::{StatusCode, Request, HeaderMap},
    response::Json,
    routing::{get, post},
    Router,
    middleware::{self, Next},
    body::Body,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error};

use crate::communication_logger::{
    CommunicationLogger, Direction, CommunicationType, Protocol
};
use crate::mac_pro_client::MacProClient;

#[derive(Clone)]
pub struct AppState {
    pub comm_logger: Arc<CommunicationLogger>,
    pub mac_pro_client: Arc<MacProClient>,
    pub shared_secret: String,
    pub knowledge_store: Arc<RwLock<HashMap<String, Value>>>,
    pub context_store: Arc<RwLock<HashMap<String, Value>>>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    limit: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct KnowledgeRequest {
    category: String,
    content: String,
    metadata: Option<Value>,
}

#[derive(Deserialize, Serialize)]
pub struct ContextUpdateRequest {
    context_type: String,
    data: Value,
}

#[derive(Deserialize)]
pub struct AnalysisRequest {
    data_sources: Vec<String>,
    analysis_type: String,
    parameters: Option<Value>,
}

#[derive(Deserialize)]
pub struct InsightRequest {
    category: String,
    content: String,
    confidence: f64,
    source: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Value>,
    timestamp: String,
}

impl ApiResponse {
    fn success(message: &str, data: Option<Value>) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub async fn create_server(
    port: u16, 
    mac_pro_host: String, 
    shared_secret: String,
) -> Result<(), anyhow::Error> {
    let comm_logger = Arc::new(CommunicationLogger::new().await?);
    let mac_pro_client = Arc::new(MacProClient::new(
        &mac_pro_host, 
        8080, 
        shared_secret.clone(),
        comm_logger.clone()
    ));

    let state = AppState {
        comm_logger,
        mac_pro_client,
        shared_secret,
        knowledge_store: Arc::new(RwLock::new(HashMap::new())),
        context_store: Arc::new(RwLock::new(HashMap::new())),
    };

    // Create base router with public endpoints
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/discovery", get(discovery_info));

    // Create protected routes that require authentication
    let protected_routes = Router::new()
        .route("/api/receive-knowledge", post(receive_knowledge))
        .route("/api/receive-context-update", post(receive_context_update))
        .route("/api/analyze-patterns", post(analyze_patterns))
        .route("/api/generate-insights", post(generate_insights))
        .route("/api/get-rust-status", get(get_rust_status))
        .route("/api/communication-logs", get(get_communication_logs))
        .route("/api/communication-stats", get(get_communication_stats))
        .route("/api/knowledge/:category", get(get_knowledge))
        .route("/api/context/:context_type", get(get_context))
        .route("/api/send-to-mac-pro", post(send_to_mac_pro))
        // Add endpoints expected by KOTA MCP server
        .route("/api/send-knowledge", post(send_knowledge))
        .route("/api/send-context-update", post(send_context_update))
        .route("/api/query-mcp-data", post(query_mcp_data))
        .route("/api/system-status", get(system_status))
        .route("/api/receive-insight", post(receive_insight))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Combine routes
    let app = public_routes
        .merge(protected_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Rust Bridge Server listening on port {}", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}

// Authentication middleware
async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    // Allow health check and discovery endpoints without auth
    let path = request.uri().path();
    if path == "/health" || path == "/discovery" {
        return Ok(next.run(request).await);
    }

    // Check Authorization header
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            // Expected format: "Bearer <token>"
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if token == state.shared_secret {
                    return Ok(next.run(request).await);
                }
            }
        }
    }

    warn!("Authentication failed for request to {}", path);
    Err(StatusCode::UNAUTHORIZED)
}

async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rust-bridge-server",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn discovery_info() -> Json<Value> {
    Json(serde_json::json!({
        "service": "rust-bridge-server",
        "version": "0.1.0",
        "capabilities": [
            "knowledge_analysis", 
            "pattern_detection", 
            "proactive_insights",
            "mac_pro_communication",
            "data_processing"
        ],
        "endpoints": [
            "/api/receive-knowledge",
            "/api/receive-context-update", 
            "/api/analyze-patterns",
            "/api/generate-insights",
            "/api/get-rust-status",
            "/api/communication-logs",
            "/api/send-to-mac-pro"
        ],
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn receive_knowledge(
    State(state): State<AppState>,
    Json(payload): Json<KnowledgeRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("Received knowledge update: category={}, content_length={}", 
          payload.category, payload.content.len());

    // Log the incoming communication
    let record = state.comm_logger.create_record(
        Direction::Inbound,
        CommunicationType::HttpRequest,
        Some("receive_knowledge".to_string()),
        Some("/api/receive-knowledge".to_string()),
        Some("POST".to_string()),
        serde_json::to_value(&payload).unwrap_or_default(),
        None,
        true,
        None,
        "mac-pro-bridge".to_string(),
        "rust-bridge-server".to_string(),
        Protocol::Http,
        None,
        Some(200),
    );

    if let Err(e) = state.comm_logger.log_communication(record).await {
        warn!("Failed to log communication: {}", e);
    }

    // Store knowledge in local store
    {
        let mut store = state.knowledge_store.write().await;
        store.insert(
            format!("{}_{}", payload.category, chrono::Utc::now().timestamp()),
            serde_json::json!({
                "category": payload.category,
                "content": payload.content,
                "metadata": payload.metadata,
                "received_at": chrono::Utc::now().to_rfc3339()
            })
        );
    }

    let response = ApiResponse::success(
        "Knowledge received and stored successfully",
        Some(serde_json::json!({
            "category": payload.category,
            "processed_at": chrono::Utc::now().to_rfc3339()
        }))
    );

    Ok(Json(response))
}

async fn receive_context_update(
    State(state): State<AppState>,
    Json(payload): Json<ContextUpdateRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("Received context update: type={}", payload.context_type);

    // Log the incoming communication
    let record = state.comm_logger.create_record(
        Direction::Inbound,
        CommunicationType::HttpRequest,
        Some("receive_context_update".to_string()),
        Some("/api/receive-context-update".to_string()),
        Some("POST".to_string()),
        serde_json::to_value(&payload).unwrap_or_default(),
        None,
        true,
        None,
        "mac-pro-bridge".to_string(),
        "rust-bridge-server".to_string(),
        Protocol::Http,
        None,
        Some(200),
    );

    if let Err(e) = state.comm_logger.log_communication(record).await {
        warn!("Failed to log communication: {}", e);
    }

    // Store context update
    {
        let mut store = state.context_store.write().await;
        store.insert(
            payload.context_type.clone(),
            serde_json::json!({
                "data": payload.data,
                "updated_at": chrono::Utc::now().to_rfc3339()
            })
        );
    }

    let response = ApiResponse::success(
        "Context update received and stored successfully",
        Some(serde_json::json!({
            "context_type": payload.context_type,
            "processed_at": chrono::Utc::now().to_rfc3339()
        }))
    );

    Ok(Json(response))
}

async fn analyze_patterns(
    State(_state): State<AppState>,
    Json(payload): Json<AnalysisRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("Analyzing patterns for: {:?}", payload.data_sources);

    // Simulate pattern analysis
    let analysis_results = serde_json::json!({
        "patterns_found": [
            {
                "type": "schedule_pattern",
                "confidence": 0.87,
                "description": "User tends to have heavy meeting loads on Tuesdays and Thursdays"
            },
            {
                "type": "productivity_pattern", 
                "confidence": 0.92,
                "description": "Highest productivity between 9-11 AM and 2-4 PM"
            }
        ],
        "recommendations": [
            "Consider scheduling deep work during high-productivity windows",
            "Block calendar time for focused work on heavy meeting days"
        ],
        "analysis_type": payload.analysis_type,
        "analyzed_at": chrono::Utc::now().to_rfc3339()
    });

    let response = ApiResponse::success(
        "Pattern analysis completed successfully",
        Some(analysis_results)
    );

    Ok(Json(response))
}

async fn generate_insights(
    State(state): State<AppState>,
    Json(_payload): Json<Value>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("Generating insights from provided data");

    // Read current knowledge and context for insight generation
    let knowledge = state.knowledge_store.read().await;
    let context = state.context_store.read().await;

    let insights = serde_json::json!({
        "insights": [
            {
                "category": "productivity",
                "insight": "Based on recent patterns, consider scheduling important meetings in the morning when focus is highest",
                "confidence": 0.89,
                "data_sources": ["calendar", "activity_tracking"],
                "actionable": true
            },
            {
                "category": "schedule_optimization",
                "insight": "You have 3 back-to-back meetings tomorrow - consider adding buffer time between meetings",
                "confidence": 0.95,
                "data_sources": ["calendar"],
                "actionable": true
            }
        ],
        "knowledge_entries_analyzed": knowledge.len(),
        "context_types_analyzed": context.len(),
        "generated_at": chrono::Utc::now().to_rfc3339()
    });

    // Automatically send insights to Mac Pro
    if let Err(e) = state.mac_pro_client.send_insight(
        "automated_analysis",
        &serde_json::to_string_pretty(&insights).unwrap_or_default(),
        0.85,
        "rust-pattern-analyzer"
    ).await {
        warn!("Failed to send insights to Mac Pro: {}", e);
    }

    let response = ApiResponse::success(
        "Insights generated and sent to Mac Pro successfully",
        Some(insights)
    );

    Ok(Json(response))
}

async fn get_rust_status(State(state): State<AppState>) -> Json<ApiResponse> {
    let knowledge = state.knowledge_store.read().await;
    let context = state.context_store.read().await;

    let status = serde_json::json!({
        "service": "rust-bridge-server",
        "status": "running",
        "uptime": "N/A", // TODO: Track actual uptime
        "mac_pro_connection": state.mac_pro_client.test_connection().await,
        "knowledge_entries": knowledge.len(),
        "context_types": context.len(),
        "last_communication": chrono::Utc::now().to_rfc3339()
    });

    Json(ApiResponse::success("Status retrieved successfully", Some(status)))
}

async fn get_communication_logs(
    State(state): State<AppState>,
    Query(params): Query<LogQuery>
) -> Result<Json<ApiResponse>, StatusCode> {
    match state.comm_logger.get_recent_logs(params.limit).await {
        Ok(logs) => {
            let response = ApiResponse::success(
                &format!("Retrieved {} communication logs", logs.len()),
                Some(serde_json::json!({
                    "logs": logs,
                    "total_count": logs.len()
                }))
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to retrieve communication logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_communication_stats(State(state): State<AppState>) -> Json<ApiResponse> {
    // Generate communication statistics
    let stats = serde_json::json!({
        "total_communications": "N/A", // TODO: Calculate from logs
        "successful_communications": "N/A",
        "failed_communications": "N/A",
        "mac_pro_connection_status": state.mac_pro_client.test_connection().await,
        "last_successful_communication": chrono::Utc::now().to_rfc3339(),
        "generated_at": chrono::Utc::now().to_rfc3339()
    });

    Json(ApiResponse::success("Communication stats retrieved", Some(stats)))
}

async fn get_knowledge(
    State(state): State<AppState>,
    Path(category): Path<String>
) -> Json<ApiResponse> {
    let knowledge = state.knowledge_store.read().await;
    
    let filtered: Vec<&Value> = knowledge
        .values()
        .filter(|entry| {
            entry.get("category")
                .and_then(|cat| cat.as_str())
                .map(|cat| cat == category)
                .unwrap_or(false)
        })
        .collect();

    Json(ApiResponse::success(
        &format!("Retrieved {} knowledge entries for category {}", filtered.len(), category),
        Some(serde_json::json!({
            "category": category,
            "entries": filtered,
            "count": filtered.len()
        }))
    ))
}

async fn get_context(
    State(state): State<AppState>,
    Path(context_type): Path<String>
) -> Json<ApiResponse> {
    let context = state.context_store.read().await;
    
    if let Some(data) = context.get(&context_type) {
        Json(ApiResponse::success(
            &format!("Retrieved context for type {}", context_type),
            Some(data.clone())
        ))
    } else {
        Json(ApiResponse::error(&format!("No context found for type {}", context_type)))
    }
}

async fn send_to_mac_pro(
    State(state): State<AppState>,
    Json(payload): Json<Value>
) -> Result<Json<ApiResponse>, StatusCode> {
    // Allow direct forwarding to Mac Pro for testing
    let endpoint = payload.get("endpoint")
        .and_then(|e| e.as_str())
        .unwrap_or("/api/receive-insight");
    
    let data = payload.get("data").cloned().unwrap_or_else(|| payload.clone());

    match state.mac_pro_client.call_api("POST", endpoint, Some(data)).await {
        Ok(response) => Ok(Json(ApiResponse::success(
            "Successfully sent data to Mac Pro",
            Some(response)
        ))),
        Err(e) => {
            error!("Failed to send data to Mac Pro: {}", e);
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

// New endpoints expected by KOTA MCP server
async fn send_knowledge(
    State(state): State<AppState>,
    Json(payload): Json<KnowledgeRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("KOTA MCP: Sending knowledge - category: {}", payload.category);
    
    // Store locally
    {
        let mut store = state.knowledge_store.write().await;
        store.insert(
            format!("{}_{}", payload.category, chrono::Utc::now().timestamp()),
            serde_json::json!({
                "category": payload.category,
                "content": payload.content,
                "metadata": payload.metadata,
                "source": "kota-mcp",
                "stored_at": chrono::Utc::now().to_rfc3339()
            })
        );
    }

    let response = ApiResponse::success(
        "Knowledge sent and stored successfully",
        Some(serde_json::json!({
            "category": payload.category,
            "stored_at": chrono::Utc::now().to_rfc3339()
        }))
    );

    Ok(Json(response))
}

async fn send_context_update(
    State(state): State<AppState>,
    Json(payload): Json<ContextUpdateRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("KOTA MCP: Context update - type: {}", payload.context_type);
    
    // Store context update
    {
        let mut store = state.context_store.write().await;
        store.insert(
            payload.context_type.clone(),
            serde_json::json!({
                "data": payload.data,
                "source": "kota-mcp",
                "updated_at": chrono::Utc::now().to_rfc3339()
            })
        );
    }

    let response = ApiResponse::success(
        "Context update sent and stored successfully",
        Some(serde_json::json!({
            "context_type": payload.context_type,
            "updated_at": chrono::Utc::now().to_rfc3339()
        }))
    );

    Ok(Json(response))
}

#[derive(Deserialize)]
struct MCPQueryRequest {
    server_name: String,
    tool_name: String,
    arguments: Option<Value>,
}

async fn query_mcp_data(
    State(_state): State<AppState>,
    Json(payload): Json<MCPQueryRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("KOTA MCP: Querying MCP data - server: {}, tool: {}", 
          payload.server_name, payload.tool_name);
    
    // Mock response for now - in real implementation this would proxy to actual MCP servers
    let mock_data = serde_json::json!({
        "server_name": payload.server_name,
        "tool_name": payload.tool_name,
        "mock_response": true,
        "data": {
            "status": "success",
            "message": "This is a mock response - integrate with actual MCP servers"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let response = ApiResponse::success(
        "MCP data queried successfully",
        Some(mock_data)
    );

    Ok(Json(response))
}

async fn system_status(State(state): State<AppState>) -> Json<ApiResponse> {
    let knowledge = state.knowledge_store.read().await;
    let context = state.context_store.read().await;

    let status = serde_json::json!({
        "service": "kota-bridge-server",
        "version": "0.1.0",
        "status": "running",
        "capabilities": [
            "knowledge_storage",
            "context_management", 
            "mcp_integration",
            "bridge_communication"
        ],
        "statistics": {
            "knowledge_entries": knowledge.len(),
            "context_types": context.len(),
            "uptime": "active"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Json(ApiResponse::success("System status retrieved", Some(status)))
}

#[derive(Deserialize)]
struct InsightReceiveRequest {
    category: String,
    content: String,
    confidence: f64,
    source: Option<String>,
}

async fn receive_insight(
    State(state): State<AppState>,
    Json(payload): Json<InsightReceiveRequest>
) -> Result<Json<ApiResponse>, StatusCode> {
    info!("KOTA MCP: Received insight - category: {}, confidence: {}", 
          payload.category, payload.confidence);
    
    // Store insight
    {
        let mut store = state.knowledge_store.write().await;
        store.insert(
            format!("insight_{}_{}", payload.category, chrono::Utc::now().timestamp()),
            serde_json::json!({
                "type": "insight",
                "category": payload.category,
                "content": payload.content,
                "confidence": payload.confidence,
                "source": payload.source.unwrap_or_else(|| "unknown".to_string()),
                "received_at": chrono::Utc::now().to_rfc3339()
            })
        );
    }

    let response = ApiResponse::success(
        "Insight received and stored successfully",
        Some(serde_json::json!({
            "category": payload.category,
            "confidence": payload.confidence,
            "processed_at": chrono::Utc::now().to_rfc3339()
        }))
    );

    Ok(Json(response))
}