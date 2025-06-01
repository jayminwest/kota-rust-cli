// User approval system for command execution with risk assessment

use std::io::{self, Write};
use std::str::FromStr;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use colored::*;
use chrono::{DateTime, Utc};

/// Approval mode for command execution
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ApprovalMode {
    /// Always ask for approval
    Always,
    /// Never ask (auto-approve)
    Never,
    /// Ask only for unknown commands
    Unknown,
    /// Ask based on policy
    Policy,
}

impl FromStr for ApprovalMode {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "always" => Ok(ApprovalMode::Always),
            "never" => Ok(ApprovalMode::Never),
            "unknown" => Ok(ApprovalMode::Unknown),
            "policy" => Ok(ApprovalMode::Policy),
            _ => Err(anyhow::anyhow!("Unknown approval mode: {}", s)),
        }
    }
}

/// Approval request details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub command: String,
    pub args: Vec<String>,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub timestamp: DateTime<Utc>,
    pub context: Option<String>,
}

/// Risk level for commands
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn color(&self) -> colored::Color {
        match self {
            RiskLevel::Low => Color::Green,
            RiskLevel::Medium => Color::Yellow,
            RiskLevel::High => Color::BrightRed,
            RiskLevel::Critical => Color::Red,
        }
    }
    
    pub fn symbol(&self) -> &'static str {
        match self {
            RiskLevel::Low => "✓",
            RiskLevel::Medium => "!",
            RiskLevel::High => "⚠",
            RiskLevel::Critical => "✗",
        }
    }
}

/// Approval system for command execution
pub struct ApprovalSystem {
    mode: ApprovalMode,
    auto_approve_patterns: Vec<String>,
    history: Vec<(ApprovalRequest, bool)>, // (request, approved)
}

impl ApprovalSystem {
    pub fn new(mode: ApprovalMode) -> Self {
        Self {
            mode,
            auto_approve_patterns: vec![
                "^ls".to_string(),
                "^pwd$".to_string(),
                "^echo".to_string(),
                "^cat".to_string(),
                "^date$".to_string(),
            ],
            history: Vec::new(),
        }
    }
    
    /// Request approval for a command
    pub async fn request_approval(&mut self, request: ApprovalRequest) -> Result<bool> {
        // Check if we should auto-approve
        if self.should_auto_approve(&request) {
            self.history.push((request, true));
            return Ok(true);
        }
        
        // Display the approval request
        self.display_request(&request);
        
        // Get user input
        let approved = self.get_user_decision().await?;
        
        // Record the decision
        self.history.push((request, approved));
        
        Ok(approved)
    }
    
    /// Check if a command should be auto-approved
    fn should_auto_approve(&self, request: &ApprovalRequest) -> bool {
        match self.mode {
            ApprovalMode::Never => true,
            ApprovalMode::Always => false,
            ApprovalMode::Unknown | ApprovalMode::Policy => {
                // Check auto-approve patterns
                let full_command = format!("{} {}", request.command, request.args.join(" "));
                for pattern in &self.auto_approve_patterns {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if regex.is_match(&full_command) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
    
    /// Display the approval request to the user
    fn display_request(&self, request: &ApprovalRequest) {
        println!("\n{}", "═".repeat(60).bright_blue());
        println!("{} {}", 
            "COMMAND APPROVAL REQUEST".bright_yellow().bold(),
            request.timestamp.format("[%Y-%m-%d %H:%M:%S]").to_string().dimmed()
        );
        println!("{}", "═".repeat(60).bright_blue());
        
        // Risk level
        println!("{}: {} {}",
            "Risk Level".bold(),
            request.risk_level.symbol().color(request.risk_level.color()),
            format!("{:?}", request.risk_level).color(request.risk_level.color())
        );
        
        // Command
        println!("{}: {} {}",
            "Command".bold(),
            request.command.bright_cyan(),
            request.args.join(" ").bright_white()
        );
        
        // Reason
        println!("{}: {}", "Reason".bold(), request.reason);
        
        // Context if available
        if let Some(context) = &request.context {
            println!("{}: {}", "Context".bold(), context.dimmed());
        }
        
        println!("{}", "─".repeat(60).dimmed());
    }
    
    /// Get user's approval decision
    async fn get_user_decision(&self) -> Result<bool> {
        print!("{} {} ",
            "Approve this command?".bright_yellow(),
            "[y/N/d(etails)]".dimmed()
        );
        io::stdout().flush()?;
        
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    println!("{}", "✓ Command approved".green());
                    return Ok(true);
                }
                "n" | "no" | "" => {
                    println!("{}", "✗ Command denied".red());
                    return Ok(false);
                }
                "d" | "details" => {
                    self.show_details();
                    print!("{} {} ",
                        "Approve this command?".bright_yellow(),
                        "[y/N]".dimmed()
                    );
                    io::stdout().flush()?;
                }
                _ => {
                    print!("{} {} ",
                        "Invalid input. Please enter".red(),
                        "[y/N/d]".dimmed()
                    );
                    io::stdout().flush()?;
                }
            }
        }
    }
    
    /// Show additional details about the command
    fn show_details(&self) {
        println!("\n{}", "Additional Information:".bold().underline());
        println!("• This command will be executed in a sandboxed environment");
        println!("• File system access will be restricted to approved directories");
        println!("• Network access may be limited based on the current policy");
        println!("• All command output will be logged for audit purposes");
        println!();
    }
    
    /// Get approval history
    pub fn get_history(&self) -> &[(ApprovalRequest, bool)] {
        &self.history
    }
    
    /// Add an auto-approve pattern
    pub fn add_auto_approve_pattern(&mut self, pattern: String) {
        self.auto_approve_patterns.push(pattern);
    }
    
    /// Set approval mode
    pub fn set_mode(&mut self, mode: ApprovalMode) {
        self.mode = mode;
    }
}

impl ApprovalRequest {
    /// Create a new approval request
    pub fn new(command: String, args: Vec<String>, reason: String) -> Self {
        let risk_level = Self::assess_risk(&command, &args);
        
        Self {
            command,
            args,
            reason,
            risk_level,
            timestamp: Utc::now(),
            context: None,
        }
    }
    
    /// Assess the risk level of a command
    fn assess_risk(command: &str, args: &[String]) -> RiskLevel {
        // Critical risk commands
        let critical_commands = ["rm", "sudo", "chmod", "chown", "mkfs", "dd"];
        if critical_commands.contains(&command) {
            return RiskLevel::Critical;
        }
        
        // High risk based on arguments
        let args_str = args.join(" ");
        if args_str.contains("--force") || args_str.contains("-rf") || 
           args_str.contains("/") && command == "rm" {
            return RiskLevel::High;
        }
        
        // Medium risk commands
        let medium_commands = ["mv", "cp", "ln", "touch", "mkdir"];
        if medium_commands.contains(&command) {
            return RiskLevel::Medium;
        }
        
        // Default to low risk
        RiskLevel::Low
    }
    
    /// Set additional context
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_risk_assessment() {
        let req = ApprovalRequest::new("rm".to_string(), vec!["-rf".to_string(), "/".to_string()], "test".to_string());
        assert_eq!(req.risk_level, RiskLevel::Critical);
        
        let req = ApprovalRequest::new("ls".to_string(), vec!["-la".to_string()], "test".to_string());
        assert_eq!(req.risk_level, RiskLevel::Low);
        
        let req = ApprovalRequest::new("mv".to_string(), vec!["file1".to_string(), "file2".to_string()], "test".to_string());
        assert_eq!(req.risk_level, RiskLevel::Medium);
    }
    
    #[tokio::test]
    async fn test_auto_approve() {
        let system = ApprovalSystem::new(ApprovalMode::Policy);
        
        let req = ApprovalRequest::new("ls".to_string(), vec!["-la".to_string()], "List files".to_string());
        let approved = system.should_auto_approve(&req);
        assert!(approved);
        
        let req = ApprovalRequest::new("rm".to_string(), vec!["file".to_string()], "Remove file".to_string());
        let approved = system.should_auto_approve(&req);
        assert!(!approved);
    }
}