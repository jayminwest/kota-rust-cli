// ResearchAgent: Specialized for research and investigation tasks

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::ContextManager;
use crate::llm::{self, ModelConfig};
use crate::memory::MemoryManager;

use super::traits::{Agent, AgentCapability, AgentMessage, TaskStatus};

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
                if task.description.contains("research")
                    || task.description.contains("investigate")
                    || task.description.contains("find")
                {
                    let task_id = task.id.clone();

                    // Execute research directly
                    let findings = self.research_topic(&task.description).await?;

                    // Store findings in memory
                    if let Some(memory) = &self.memory_manager {
                        let mm = memory.lock().await;
                        let _ = mm.store_learning("research_findings", &findings);
                    }

                    Ok(Some(AgentMessage::TaskUpdate(
                        task_id,
                        TaskStatus::Completed(
                            "Research completed. Key findings stored in knowledge base."
                                .to_string(),
                        ),
                    )))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
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

        let model_config = self
            .model_config
            .as_ref()
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
