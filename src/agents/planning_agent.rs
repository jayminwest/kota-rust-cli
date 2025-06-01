#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::{self, ModelConfig};
use crate::memory::MemoryManager;

use super::traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskStatus, TaskPriority};

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
                if query.contains("plan") || query.contains("strategy") || query.contains("approach") {
                    let response = self.analyze_planning_request(&query).await?;
                    Ok(Some(AgentMessage::QueryResponse(query, response)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    
    async fn execute_task(&mut self, task: &mut AgentTask) -> Result<()> {
        task.update_status(TaskStatus::InProgress);
        
        // Create a comprehensive plan for the task
        let subtasks = self.plan_task(task).await?;
        
        // Add subtasks to the main task
        for subtask in subtasks {
            task.add_subtask(subtask);
        }
        
        task.update_status(TaskStatus::Completed(
            format!("Created comprehensive plan with {} subtasks", task.subtasks.len())
        ));
        
        Ok(())
    }
    
    async fn plan_task(&mut self, task: &AgentTask) -> Result<Vec<AgentTask>> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            r#"Create a detailed execution plan for this task: {}

            Break it down into specific, actionable subtasks. For each subtask, provide:
            1. A clear description
            2. Priority level (Critical/High/Normal/Low)
            3. Any dependencies on other subtasks
            
            Format your response as a numbered list."#,
            task.description
        );
        
        let response = llm::ask_model_with_config(&prompt, &context, model_config).await?;
        
        // Parse the response into subtasks
        let subtasks = self.parse_plan_response(&response)?;
        
        Ok(subtasks)
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
    
    async fn learn_from_task(&mut self, task: &AgentTask) -> Result<()> {
        if let Some(memory) = &self.memory_manager {
            let learning = format!(
                "Planning task '{}' completed with {} subtasks. Status: {:?}",
                task.description, task.subtasks.len(), task.status
            );
            
            let mm = memory.lock().await;
            mm.store_learning("planning_strategies", &learning)?;
        }
        Ok(())
    }
}

impl PlanningAgent {
    async fn create_comprehensive_plan(&mut self, task: &AgentTask) -> Result<Vec<AgentTask>> {
        // Use the plan_task method to create a plan
        let subtasks = self.plan_task(task).await?;
        
        // Store the plan
        let mut plan = task.clone();
        for subtask in &subtasks {
            plan.add_subtask(subtask.clone());
        }
        self.active_plans.push(plan);
        
        Ok(subtasks)
    }
    
    async fn analyze_planning_request(&self, query: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            "As a strategic planning expert, analyze this request: {}\n\nProvide a comprehensive planning approach.",
            query
        );
        
        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
    
    fn parse_plan_response(&self, response: &str) -> Result<Vec<AgentTask>> {
        let mut subtasks = Vec::new();
        let mut current_priority = TaskPriority::Normal;
        
        for line in response.lines() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            
            // Check for priority indicators
            if line.to_lowercase().contains("critical") {
                current_priority = TaskPriority::Critical;
            } else if line.to_lowercase().contains("high priority") {
                current_priority = TaskPriority::High;
            } else if line.to_lowercase().contains("low priority") {
                current_priority = TaskPriority::Low;
            }
            
            // Look for numbered items
            if line.starts_with(char::is_numeric) || line.starts_with("- ") || line.starts_with("* ") {
                let description = line
                    .trim_start_matches(char::is_numeric)
                    .trim_start_matches('.')
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim();
                
                if !description.is_empty() {
                    subtasks.push(AgentTask::new(
                        description.to_string(),
                        current_priority.clone(),
                    ));
                }
            }
        }
        
        // If no subtasks were parsed, create some default ones
        if subtasks.is_empty() {
            subtasks.push(AgentTask::new(
                "Analyze requirements and constraints".to_string(),
                TaskPriority::High,
            ));
            subtasks.push(AgentTask::new(
                "Design solution architecture".to_string(),
                TaskPriority::High,
            ));
            subtasks.push(AgentTask::new(
                "Implement core functionality".to_string(),
                TaskPriority::Normal,
            ));
            subtasks.push(AgentTask::new(
                "Test and validate solution".to_string(),
                TaskPriority::Normal,
            ));
        }
        
        Ok(subtasks)
    }
}