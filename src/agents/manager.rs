use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
// AgentManager: Coordinates multiple agents for specialized tasks

use crate::context::ContextManager;
use crate::llm::ModelConfig;
use crate::memory::MemoryManager;

use super::traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskStatus};
use super::{CodeAgent, PlanningAgent, ResearchAgent};

#[derive(Clone)]
pub struct AgentManager {
    agents: Arc<Mutex<HashMap<String, Box<dyn Agent>>>>,
    context_manager: Arc<Mutex<ContextManager>>,
    model_config: ModelConfig,
    memory_manager: Arc<Mutex<MemoryManager>>,
    active_tasks: Arc<Mutex<HashMap<String, AgentTask>>>,
}

impl AgentManager {
    pub async fn new(
        context_manager: Arc<Mutex<ContextManager>>,
        model_config: ModelConfig,
        memory_manager: Arc<Mutex<MemoryManager>>,
    ) -> Result<Self> {
        let manager = Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            context_manager: context_manager.clone(),
            model_config: model_config.clone(),
            memory_manager: memory_manager.clone(),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        };

        // Initialize all available agents
        manager.initialize_agents().await?;

        Ok(manager)
    }

    async fn initialize_agents(&self) -> Result<()> {
        let mut agents = self.agents.lock().await;

        // Initialize CodeAgent
        let mut code_agent = CodeAgent::new();
        code_agent
            .initialize(
                self.context_manager.clone(),
                self.model_config.clone(),
                self.memory_manager.clone(),
            )
            .await?;
        agents.insert("code".to_string(), Box::new(code_agent));

        // Initialize PlanningAgent
        let mut planning_agent = PlanningAgent::new();
        planning_agent
            .initialize(
                self.context_manager.clone(),
                self.model_config.clone(),
                self.memory_manager.clone(),
            )
            .await?;
        agents.insert("planning".to_string(), Box::new(planning_agent));

        // Initialize ResearchAgent
        let mut research_agent = ResearchAgent::new();
        research_agent
            .initialize(
                self.context_manager.clone(),
                self.model_config.clone(),
                self.memory_manager.clone(),
            )
            .await?;
        agents.insert("research".to_string(), Box::new(research_agent));

        Ok(())
    }

    pub async fn list_agents(&self) -> Vec<(String, String)> {
        let agents = self.agents.lock().await;
        let mut agent_list = Vec::new();

        for (name, agent) in agents.iter() {
            agent_list.push((name.clone(), agent.get_status()));
        }

        agent_list
    }

    pub async fn get_agent_capabilities(&self, agent_name: &str) -> Option<Vec<AgentCapability>> {
        let agents = self.agents.lock().await;
        agents.get(agent_name).map(|agent| agent.capabilities())
    }

    pub async fn delegate_task(&self, task_description: String, agent_name: Option<String>) -> Result<String> {
        let task = AgentTask::new(task_description, super::traits::TaskPriority::Normal);
        let task_id = task.id.clone();

        // Store the task
        {
            let mut active_tasks = self.active_tasks.lock().await;
            active_tasks.insert(task_id.clone(), task.clone());
        }

        let agent_name = if let Some(name) = agent_name {
            name
        } else {
            // Auto-select the best agent based on task description
            self.select_best_agent(&task.description).await?
        };

        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(&agent_name) {
            let message = AgentMessage::TaskRequest(task);
            if let Some(response) = agent.process_message(message).await? {
                match response {
                    AgentMessage::TaskUpdate(id, status) => {
                        // Update task status
                        let mut active_tasks = self.active_tasks.lock().await;
                        if let Some(task) = active_tasks.get_mut(&id) {
                            task.update_status(status.clone());
                        }
                        
                        match status {
                            TaskStatus::Completed(msg) => Ok(format!("✓ Task completed by {}: {}", agent_name, msg)),
                            TaskStatus::Failed(msg) => Ok(format!("✗ Task failed in {}: {}", agent_name, msg)),
                            TaskStatus::InProgress => Ok(format!("⏳ Task started by {}", agent_name)),
                            TaskStatus::Blocked(msg) => Ok(format!("⏸ Task blocked in {}: {}", agent_name, msg)),
                            TaskStatus::Pending => Ok(format!("📋 Task queued for {}", agent_name)),
                        }
                    }
                    _ => Ok(format!("Agent {} is processing the task", agent_name)),
                }
            } else {
                Ok(format!("Agent {} cannot handle this task", agent_name))
            }
        } else {
            Err(anyhow::anyhow!("Agent '{}' not found", agent_name))
        }
    }

    pub async fn ask_agent(&self, query: String, agent_name: Option<String>) -> Result<String> {
        let agent_name = if let Some(name) = agent_name {
            name
        } else {
            // Auto-select the best agent based on query
            self.select_best_agent(&query).await?
        };

        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(&agent_name) {
            let message = AgentMessage::QueryRequest(query.clone());
            if let Some(response) = agent.process_message(message).await? {
                match response {
                    AgentMessage::QueryResponse(_, answer) => Ok(format!("**{}:** {}", agent_name, answer)),
                    _ => Ok(format!("Agent {} is processing your query", agent_name)),
                }
            } else {
                Ok(format!("Agent {} cannot answer this query", agent_name))
            }
        } else {
            Err(anyhow::anyhow!("Agent '{}' not found", agent_name))
        }
    }

    async fn select_best_agent(&self, task_or_query: &str) -> Result<String> {
        let text = task_or_query.to_lowercase();

        // Code-related keywords
        if text.contains("code") || text.contains("implement") || text.contains("function") 
            || text.contains("refactor") || text.contains("test") || text.contains("debug") {
            return Ok("code".to_string());
        }

        // Planning-related keywords
        if text.contains("plan") || text.contains("strategy") || text.contains("approach") 
            || text.contains("organize") || text.contains("structure") {
            return Ok("planning".to_string());
        }

        // Research-related keywords
        if text.contains("research") || text.contains("investigate") || text.contains("find") 
            || text.contains("learn") || text.contains("explain") || text.contains("what") {
            return Ok("research".to_string());
        }

        // Default to planning agent for general tasks
        Ok("planning".to_string())
    }

    pub async fn get_active_tasks(&self) -> Vec<AgentTask> {
        let active_tasks = self.active_tasks.lock().await;
        active_tasks.values().cloned().collect()
    }

    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let active_tasks = self.active_tasks.lock().await;
        active_tasks.get(task_id).map(|task| task.status.clone())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut active_tasks = self.active_tasks.lock().await;
        if let Some(task) = active_tasks.get_mut(task_id) {
            task.update_status(TaskStatus::Failed("Cancelled by user".to_string()));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task '{}' not found", task_id))
        }
    }

    pub async fn run_self_checks(&self) -> Result<Vec<String>> {
        let agents = self.agents.lock().await;
        let mut results = Vec::new();

        for (name, agent) in agents.iter() {
            match agent.self_check().await {
                Ok(()) => results.push(format!("✓ {}: OK", name)),
                Err(e) => results.push(format!("✗ {}: {}", name, e)),
            }
        }

        Ok(results)
    }

    pub async fn broadcast_context_update(&self, context_info: String) -> Result<()> {
        let mut agents = self.agents.lock().await;
        let message = AgentMessage::ContextUpdate(context_info);

        for agent in agents.values_mut() {
            let _ = agent.process_message(message.clone()).await;
        }

        Ok(())
    }
}