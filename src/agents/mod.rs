// Agent modules for proactive, autonomous behavior
#[allow(dead_code)]
pub mod code_agent;
#[allow(dead_code)]
pub mod manager;
#[allow(dead_code)]
pub mod planning_agent;
#[allow(dead_code)]
pub mod research_agent;
#[allow(dead_code)]
pub mod traits;

#[allow(unused_imports)]
pub use code_agent::CodeAgent;
pub use manager::AgentManager;
#[allow(unused_imports)]
pub use planning_agent::PlanningAgent;
#[allow(unused_imports)]
pub use research_agent::ResearchAgent;
#[allow(unused_imports)]
pub use traits::{Agent, AgentCapability, AgentMessage, AgentTask, TaskPriority, TaskStatus};
