// PlanningAgent: Specialized for task planning and strategic thinking

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::{self, ModelConfig};
use crate::memory::MemoryManager;

use super::traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskStatus};

pub struct PlanningAgent {
    name: String,
    context_manager: Option<Arc<Mutex<ContextManager>>>,
    model_config: Option<ModelConfig>,
    memory_manager: Option<Arc<Mutex<MemoryManager>>>,
    active_plans: Vec<AgentTask>,
}

impl PlanningAgent {
    pub fn new() -> Self {
        Self {
            name: "PlanningAgent".to_string(),
            context_manager: None,
            model_config: None,
            memory_manager: None,
            active_plans: Vec::new(),
        }
    }
}

#[async_trait]
impl Agent for PlanningAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::Planning,
            AgentCapability::Research,
            AgentCapability::Learning,
        ]
    }

    async fn initialize(
        &mut self,
        context_manager: Arc<Mutex<ContextManager>>,
        model_config: ModelConfig,
        memory_manager: Arc<Mutex<MemoryManager>>,
    ) -> Result<()> {
        self.context_manager = Some(context_manager);
        self.model_config = Some(model_config);
        self.memory_manager = Some(memory_manager);
        Ok(())
    }

    async fn process_message(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>> {
        match message {
            AgentMessage::TaskRequest(task) => {
                let plan = self.create_comprehensive_plan(&task).await?;
                Ok(Some(AgentMessage::TaskUpdate(
                    task.id.clone(),
                    TaskStatus::Completed(format!("Created plan with {} subtasks", plan.len())),
                )))
            }
            AgentMessage::QueryRequest(query) => {
                if query.contains("plan")
                    || query.contains("strategy")
                    || query.contains("approach")
                {
                    let response = self.analyze_planning_request(&query).await?;
                    Ok(Some(AgentMessage::QueryResponse(query, response)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn get_status(&self) -> String {
        format!(
            "PlanningAgent: Managing {} active plans",
            self.active_plans.len()
        )
    }

    async fn self_check(&self) -> Result<()> {
        if self.context_manager.is_none() {
            return Err(anyhow::anyhow!("Context manager not initialized"));
        }
        if self.model_config.is_none() {
            return Err(anyhow::anyhow!("Model config not initialized"));
        }
        if self.memory_manager.is_none() {
            return Err(anyhow::anyhow!("Memory manager not initialized"));
        }
        Ok(())
    }
}

impl PlanningAgent {
    async fn create_comprehensive_plan(&self, task: &AgentTask) -> Result<Vec<String>> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };

        let model_config = self
            .model_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;

        let prompt = format!(
            "Create a comprehensive plan for this task: {}\n\nBreak it down into clear, actionable steps.",
            task.description
        );

        let response = llm::ask_model_with_config(&prompt, &context, model_config).await?;

        // Parse the response to extract steps
        let mut steps = Vec::new();
        for line in response.lines() {
            let line = line.trim();
            if !line.is_empty()
                && (line.starts_with(char::is_numeric)
                    || line.starts_with("-")
                    || line.starts_with("*"))
            {
                steps.push(line.to_string());
            }
        }

        if steps.is_empty() {
            steps.push("1. Analyze requirements".to_string());
            steps.push("2. Design solution".to_string());
            steps.push("3. Implement functionality".to_string());
            steps.push("4. Test and validate".to_string());
        }

        Ok(steps)
    }

    async fn analyze_planning_request(&self, query: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };

        let model_config = self
            .model_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;

        let prompt = format!(
            "As a strategic planning expert, analyze this request: {}\n\nProvide a comprehensive planning approach.",
            query
        );

        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
}
