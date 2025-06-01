// ResearchAgent: Specialized for research and investigation tasks

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::{self, ModelConfig};
use crate::memory::MemoryManager;

use super::traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskStatus};

pub struct ResearchAgent {
    name: String,
    context_manager: Option<Arc<Mutex<ContextManager>>>,
    model_config: Option<ModelConfig>,
    memory_manager: Option<Arc<Mutex<MemoryManager>>>,
}

impl ResearchAgent {
    pub fn new() -> Self {
        Self {
            name: "ResearchAgent".to_string(),
            context_manager: None,
            model_config: None,
            memory_manager: None,
        }
    }
}

#[async_trait]
impl Agent for ResearchAgent {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::Research,
            AgentCapability::Documentation,
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
            AgentMessage::QueryRequest(query) => {
                let response = self.research_topic(&query).await?;
                Ok(Some(AgentMessage::QueryResponse(query, response)))
            }
            AgentMessage::TaskRequest(task) => {
                if task.description.contains("research") || 
                   task.description.contains("investigate") ||
                   task.description.contains("find") {
                    let mut working_task = task.clone();
                    self.execute_task(&mut working_task).await?;
                    Ok(Some(AgentMessage::TaskUpdate(
                        working_task.id.clone(),
                        working_task.status.clone(),
                    )))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    
    async fn execute_task(&mut self, task: &mut AgentTask) -> Result<()> {
        task.update_status(TaskStatus::InProgress);
        
        // Extract the research topic from the task description
        let findings = self.research_topic(&task.description).await?;
        
        // Store findings in memory
        if let Some(memory) = &self.memory_manager {
            let mm = memory.lock().await;
            mm.store_learning("research_findings", &findings)?;
        }
        
        task.update_status(TaskStatus::Completed(
            "Research completed. Key findings stored in knowledge base.".to_string()
        ));
        
        Ok(())
    }
    
    async fn plan_task(&mut self, task: &AgentTask) -> Result<Vec<AgentTask>> {
        let mut subtasks = Vec::new();
        
        if task.description.contains("research") {
            subtasks.push(AgentTask::new(
                "Search existing knowledge base".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Analyze codebase for relevant patterns".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Review documentation and comments".to_string(),
                task.priority.clone(),
            ));
            subtasks.push(AgentTask::new(
                "Synthesize findings into actionable insights".to_string(),
                task.priority.clone(),
            ));
        }
        
        Ok(subtasks)
    }
    
    fn get_status(&self) -> String {
        "ResearchAgent: Ready for research and investigation tasks".to_string()
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
                "Research task '{}' completed with status: {:?}",
                task.description, task.status
            );
            
            let mm = memory.lock().await;
            mm.store_learning("research_methods", &learning)?;
        }
        Ok(())
    }
}

impl ResearchAgent {
    async fn research_topic(&self, topic: &str) -> Result<String> {
        // First, check memory for existing knowledge
        let existing_knowledge = if let Some(memory) = &self.memory_manager {
            let mm = memory.lock().await;
            mm.search_knowledge(topic).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        let context = if let Some(cm) = &self.context_manager {
            let cm = cm.lock().await;
            let mut full_context = cm.get_formatted_context();
            
            // Add existing knowledge to context
            if !existing_knowledge.is_empty() {
                full_context.push_str("\n\nExisting knowledge on this topic:\n");
                for item in existing_knowledge {
                    full_context.push_str(&format!("- {}\n", item));
                }
            }
            
            full_context
        } else {
            String::new()
        };
        
        let model_config = self.model_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model config not initialized"))?;
        
        let prompt = format!(
            r#"Research the following topic: {}

            Provide a comprehensive analysis including:
            1. Key concepts and definitions
            2. Current best practices
            3. Common patterns and approaches
            4. Potential pitfalls and considerations
            5. Relevant examples from the codebase (if any)
            
            Base your research on the provided context and your knowledge."#,
            topic
        );
        
        llm::ask_model_with_config(&prompt, &context, model_config).await
    }
}