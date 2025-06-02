// Security module for macOS-focused sandboxing and command execution
#[allow(dead_code)]
pub mod approval;
#[allow(dead_code)]
pub mod policy;
#[allow(dead_code)]
pub mod sandbox;
#[allow(dead_code)]
pub mod secure_executor;

#[allow(unused_imports)]
pub use approval::{ApprovalMode, ApprovalRequest, ApprovalSystem};
#[allow(unused_imports)]
pub use policy::{ExecutionPolicy, PolicyAction, PolicyEngine};
#[allow(unused_imports)]
pub use sandbox::{SandboxProfile, SandboxedCommand};
#[allow(unused_imports)]
pub use secure_executor::SecureExecutor;
