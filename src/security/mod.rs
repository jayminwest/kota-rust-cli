// Security module for macOS-focused sandboxing and command execution
pub mod sandbox;
pub mod policy;
pub mod approval;
pub mod secure_executor;

pub use sandbox::{SandboxProfile, SandboxedCommand};
pub use policy::{ExecutionPolicy, PolicyEngine, PolicyAction};
pub use approval::{ApprovalMode, ApprovalSystem, ApprovalRequest};
// pub use secure_executor::SecureExecutor;  // Not currently used in main code