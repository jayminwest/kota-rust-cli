// Agent modules for proactive, autonomous behavior
pub mod traits;
pub mod code_agent;
pub mod planning_agent;
pub mod research_agent;
pub mod manager;

pub use manager::AgentManager;
pub use traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskPriority, TaskStatus};
pub use code_agent::CodeAgent;
pub use planning_agent::PlanningAgent;
pub use research_agent::ResearchAgent;

