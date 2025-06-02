// Core agent traits and data structures

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::ModelConfig;
use crate::memory::MemoryManager;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed(String), // Success message
    Failed(String),    // Error message
    Blocked(String),   // Reason for block
}

#[derive(Debug, Clone)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    TaskRequest(AgentTask),
    TaskUpdate(String, TaskStatus), // Task ID, new status
    QueryRequest(String),           // Question to answer
    QueryResponse(String, String),  // Question, Answer
    ContextUpdate(String),          // New context information
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentCapability {
    CodeGeneration,
    CodeAnalysis,
    FileEditing,
    Testing,
    Documentation,
    Research,
    Planning,
    Execution,
    Learning,
    SelfModification,
}

#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's name
    fn name(&self) -> &str;

    /// Get the agent's capabilities
    fn capabilities(&self) -> Vec<AgentCapability>;

    /// Get current status
    fn get_status(&self) -> String;

    /// Initialize the agent with necessary resources
    async fn initialize(
        &mut self,
        context_manager: Arc<Mutex<ContextManager>>,
        model_config: ModelConfig,
        memory_manager: Arc<Mutex<MemoryManager>>,
    ) -> Result<()>;

    /// Process an incoming message (main execution method)
    async fn process_message(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>>;

    /// Simple self-check for health status
    async fn self_check(&self) -> Result<()> {
        Ok(())
    }
}

impl AgentTask {
    pub fn new(description: String, priority: TaskPriority) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description,
            priority,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self.status, TaskStatus::Blocked(_))
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status, TaskStatus::Completed(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed(_))
    }
}
