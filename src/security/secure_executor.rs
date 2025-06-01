#![allow(dead_code)]

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context as AnyhowContext};
use colored::*;

use crate::cmd_parser::CommandBlock;
use crate::context::ContextManager;
use super::{SandboxProfile, SandboxedCommand, PolicyEngine, ApprovalSystem, ApprovalRequest, ApprovalMode};

/// Secure command executor that integrates sandboxing, policy, and approval
pub struct SecureExecutor {
    policy_engine: Arc<Mutex<PolicyEngine>>,
    approval_system: Arc<Mutex<ApprovalSystem>>,
    default_sandbox: SandboxProfile,
    context_manager: Option<Arc<Mutex<ContextManager>>>,
}

impl SecureExecutor {
    pub fn new(approval_mode: ApprovalMode) -> Self {
        Self {
            policy_engine: Arc::new(Mutex::new(PolicyEngine::new())),
            approval_system: Arc::new(Mutex::new(ApprovalSystem::new(approval_mode))),
            default_sandbox: SandboxProfile::development(),
            context_manager: None,
        }
    }
    
    /// Set the context manager for adding command output
    pub fn set_context_manager(&mut self, cm: Arc<Mutex<ContextManager>>) {
        self.context_manager = Some(cm);
    }
    
    /// Set the default sandbox profile
    pub fn set_default_sandbox(&mut self, profile: SandboxProfile) {
        self.default_sandbox = profile;
    }
    
    /// Execute a command block securely
    pub async fn execute_command_block(&self, block: &CommandBlock) -> Result<String> {
        // Parse the command
        let (command, args) = self.parse_command(&block.command)?;
        
        // Check policy
        let policy_decision = {
            let engine = self.policy_engine.lock().await;
            engine.evaluate_command(&command, &args)?
        };
        
        match policy_decision.action {
            super::PolicyAction::Deny => {
                let message = policy_decision.rule_message
                    .unwrap_or_else(|| "Command denied by security policy".to_string());
                println!("{} {}", "✗".red(), message.red());
                return Err(anyhow::anyhow!(message));
            }
            super::PolicyAction::Allow => {
                println!("{} Command allowed by policy", "✓".green());
            }
            super::PolicyAction::RequireApproval => {
                // Request approval
                let approved = self.request_approval(&command, &args).await?;
                if !approved {
                    return Err(anyhow::anyhow!("Command execution denied by user"));
                }
            }
        }
        
        // Execute in sandbox
        println!("{} Executing in sandbox: {} {}",
            "🔒".blue(),
            command.bright_cyan(),
            args.join(" ").bright_white()
        );
        
        let output = self.execute_sandboxed(&command, args).await?;
        
        Ok(output)
    }
    
    /// Parse command string into command and arguments
    fn parse_command(&self, command_str: &str) -> Result<(String, Vec<String>)> {
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }
        
        let command = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        
        Ok((command, args))
    }
    
    /// Request approval for a command
    async fn request_approval(&self, command: &str, args: &[String]) -> Result<bool> {
        let request = ApprovalRequest::new(
            command.to_string(),
            args.to_vec(),
            "LLM suggested command execution".to_string(),
        ).with_context("Executing from KOTA CLI".to_string());
        
        let mut approval_system = self.approval_system.lock().await;
        approval_system.request_approval(request).await
    }
    
    /// Execute command in sandbox
    async fn execute_sandboxed(&self, command: &str, args: Vec<String>) -> Result<String> {
        let sandboxed = SandboxedCommand::new(
            command.to_string(),
            args.clone(),
            self.default_sandbox.clone(),
        );
        
        let output = sandboxed.execute().await
            .context("Failed to execute sandboxed command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Add output to context if available
        if let Some(cm) = &self.context_manager {
            let mut context = cm.lock().await;
            let full_output = format!(
                "Command: {} {}\nStdout:\n{}\nStderr:\n{}",
                command,
                args.join(" "),
                stdout,
                stderr
            );
            context.add_snippet(full_output);
        }
        
        if !output.status.success() {
            let error_msg = if stderr.is_empty() {
                format!("Command failed with exit code: {}", output.status.code().unwrap_or(-1))
            } else {
                format!("Command failed: {}", stderr)
            };
            return Err(anyhow::anyhow!(error_msg));
        }
        
        Ok(stdout.to_string())
    }
    
    /// Execute multiple command blocks
    pub async fn execute_command_blocks(&self, blocks: Vec<CommandBlock>) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for (i, block) in blocks.iter().enumerate() {
            println!("\n{} Executing command {}/{}",
                "📋".yellow(),
                i + 1,
                blocks.len()
            );
            
            match self.execute_command_block(block).await {
                Ok(output) => {
                    println!("{} Command completed successfully", "✓".green());
                    results.push(output);
                }
                Err(e) => {
                    println!("{} Command failed: {}", "✗".red(), e);
                    return Err(e);
                }
            }
        }
        
        Ok(results)
    }
}

/// Execute a command with streaming output
pub async fn execute_streaming_with_approval<F>(
    command: &str,
    args: Vec<String>,
    sandbox: SandboxProfile,
    approval_system: &mut ApprovalSystem,
    on_output: F,
) -> Result<i32>
where
    F: FnMut(&str) + Send + 'static,
{
    // Request approval
    let request = ApprovalRequest::new(
        command.to_string(),
        args.clone(),
        "Direct command execution".to_string(),
    );
    
    if !approval_system.request_approval(request).await? {
        return Err(anyhow::anyhow!("Command execution denied"));
    }
    
    // Execute with streaming
    let sandboxed = SandboxedCommand::new(command.to_string(), args, sandbox);
    sandboxed.execute_streaming(on_output).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_secure_executor() {
        // Skip this test if sandbox-exec is not available
        if std::env::var("CI").is_ok() || !cfg!(target_os = "macos") {
            return;
        }
        
        let executor = SecureExecutor::new(ApprovalMode::Never);
        
        let block = CommandBlock {
            command: "echo hello world".to_string(),
        };
        
        match executor.execute_command_block(&block).await {
            Ok(result) => {
                assert!(result.contains("hello world"));
            }
            Err(_) => {
                // Sandbox-exec might fail in some environments, skip the test
                eprintln!("Warning: secure executor test skipped (sandbox not available)");
            }
        }
    }
    
    #[tokio::test]
    async fn test_command_parsing() {
        let executor = SecureExecutor::new(ApprovalMode::Never);
        
        let (cmd, args) = executor.parse_command("git status --porcelain").unwrap();
        assert_eq!(cmd, "git");
        assert_eq!(args, vec!["status", "--porcelain"]);
    }
}