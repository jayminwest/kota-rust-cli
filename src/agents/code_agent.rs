use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::{self, ModelConfig};
use crate::memory::MemoryManager;
use crate::sr_parser;
use crate::editor;

use super::traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskStatus};

pub struct CodeAgent {
    name: String,
    context_manager: Option<Arc<Mutex<ContextManager>>>,
    model_config: Option<ModelConfig>,
    memory_manager: Option<Arc<Mutex<MemoryManager>>>,
}

impl CodeAgent {
    pub fn new() -> Self {
        Self {
            name: "CodeAgent".to_string(),
            context_manager: None,
            model_config: None,
            memory_manager: None,
        }
    }
}

#[async_trait]
impl Agent for CodeAgent {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::CodeGeneration,
            AgentCapability::CodeAnalysis,
            AgentCapability::FileEditing,
            AgentCapability::Testing,
            AgentCapability::Documentation,
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
                // Clone the task to work with it
                let mut working_task = task.clone();
                self.execute_task(&mut working_task).await?;
                Ok(Some(AgentMessage::TaskUpdate(
                    working_task.id.clone(),
                    working_task.status.clone(),
                )))
            }
            AgentMessage::QueryRequest(query) => {
                if query.contains("code") || query.contains("implement") || query.contains("function") {
                    let response = self.analyze_code_request(&query).await?;
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
        
        // Extract key information from task description
        let description = &task.description;
        
        if description.contains("implement") || description.contains("create") {
            match self.generate_code(description).await {
                Ok(result) => {
                    task.update_status(TaskStatus::Completed(result));
                }
                Err(e) => {
                    task.update_status(TaskStatus::Failed(e.to_string()));
                }
            }
        } else if description.contains("refactor") || description.contains("improve") {
            match self.refactor_code(description).await {
                Ok(result) => {
                    task.update_status(TaskStatus::Completed(result));
                }
                Err(e) => {
                    task.update_status(TaskStatus::Failed(e.to_string()));
                }
            }
        } else if description.contains("test") {
            match self.generate_tests(description).await {
                Ok(result) => {
                    task.update_status(TaskStatus::Completed(result));
                }
                Err(e) => {
                    task.update_status(TaskStatus::Failed(e.to_string()));
                }
            }
        } else {
            task.update_status(TaskStatus::Completed("Task analyzed and ready for implementation".to_string()));
        }
        
        Ok(())
    }
    
    async fn plan_task(&mut self, task: &AgentTask) -> Result<Vec<AgentTask>> {
        let mut subtasks = Vec::new();
        
        // Analyze the task description to create subtasks
        if task.description.contains("feature") {
            subtasks.push(AgentTask::new(
                "Analyze existing code structure".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Design feature architecture".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Implement core functionality".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Add tests for new feature".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Update documentation".to_string(),
                task.priority.clone(),
            ));
        }
        
        Ok(subtasks)
    }
    
    fn get_status(&self) -> String {
        "CodeAgent: Ready for code generation, analysis, and refactoring tasks".to_string()
    }
    
    async fn self_check(&self) -> Result<()> {
        // Verify all components are initialized
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
                "Task '{}' completed with status: {:?}",
                task.description, task.status
            );
            
            let mm = memory.lock().await;
            mm.store_learning("code_tasks", &learning)?;
        }
        Ok(())
    }
}

impl CodeAgent {
    async fn analyze_code_request(&self, query: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        // Ask the model for code analysis
        let prompt = format!(
            "As a code analysis expert, analyze this request: {}\n\nProvide a detailed response.",
            query
        );
        
        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
    
    async fn generate_code(&self, description: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            "Generate code for: {}\n\nProvide the implementation with S/R blocks for file changes.",
            description
        );
        
        let response = llm::ask_model_with_config(&prompt, &context, model_config).await?;
        
        // Check for S/R blocks and apply them
        if sr_parser::contains_sr_blocks(&response) {
            if let Ok(blocks) = sr_parser::parse_sr_blocks(&response) {
                if !blocks.is_empty() {
                    if let Some(cm) = &self.context_manager {
                        let cm = cm.lock().await;
                        editor::confirm_and_apply_blocks(blocks, &prompt, &cm).await?;
                        return Ok("Code generated and applied successfully".to_string());
                    }
                }
            }
        }
        
        Ok(response)
    }
    
    async fn refactor_code(&self, description: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            "Refactor the code as requested: {}\n\nProvide S/R blocks for the changes.",
            description
        );
        
        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
    
    async fn generate_tests(&self, description: &str) -> Result<String> {
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            "Generate tests for: {}\n\nProvide comprehensive test cases.",
            description
        );
        
        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
}