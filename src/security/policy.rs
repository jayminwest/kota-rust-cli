// Security policy engine for command filtering and access control

use std::collections::HashMap;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Command execution policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    /// Policy name
    pub name: String,
    /// Allowed command rules
    pub allowed_commands: Vec<CommandRule>,
    /// Denied command rules (takes precedence)
    pub denied_commands: Vec<CommandRule>,
    /// Default action when no rules match
    pub default_action: PolicyAction,
    /// Require approval for matched commands
    pub require_approval: bool,
}

/// Rule for matching commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRule {
    /// Command name or pattern (supports regex)
    pub pattern: String,
    /// Allowed arguments (regex patterns)
    pub allowed_args: Option<Vec<String>>,
    /// Denied arguments (regex patterns)
    pub denied_args: Option<Vec<String>>,
    /// Custom message for this rule
    pub message: Option<String>,
}

/// Policy action
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    RequireApproval,
}

/// Policy engine for evaluating commands
pub struct PolicyEngine {
    policies: HashMap<String, ExecutionPolicy>,
    active_policy: String,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            policies: HashMap::new(),
            active_policy: "default".to_string(),
        };
        
        // Add default policies
        engine.add_policy(ExecutionPolicy::default());
        engine.add_policy(ExecutionPolicy::safe());
        engine.add_policy(ExecutionPolicy::strict());
        
        engine
    }
    
    /// Add a policy to the engine
    pub fn add_policy(&mut self, policy: ExecutionPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }
    
    /// Set the active policy
    pub fn set_active_policy(&mut self, name: &str) -> Result<()> {
        if self.policies.contains_key(name) {
            self.active_policy = name.to_string();
            Ok(())
        } else {
            bail!("Policy '{}' not found", name);
        }
    }
    
    /// Evaluate a command against the active policy
    pub fn evaluate_command(&self, command: &str, args: &[String]) -> Result<PolicyDecision> {
        let policy = self.policies.get(&self.active_policy)
            .ok_or_else(|| anyhow::anyhow!("Active policy not found"))?;
        
        // Check denied commands first (they take precedence)
        for rule in &policy.denied_commands {
            if let Some(decision) = self.match_rule(rule, command, args, PolicyAction::Deny)? {
                return Ok(decision);
            }
        }
        
        // Check allowed commands
        for rule in &policy.allowed_commands {
            let action = if policy.require_approval {
                PolicyAction::RequireApproval
            } else {
                PolicyAction::Allow
            };
            
            if let Some(decision) = self.match_rule(rule, command, args, action)? {
                return Ok(decision);
            }
        }
        
        // No rules matched, use default action
        Ok(PolicyDecision {
            action: policy.default_action,
            rule_message: None,
            matched_rule: None,
        })
    }
    
    /// Match a single rule against a command
    fn match_rule(
        &self,
        rule: &CommandRule,
        command: &str,
        args: &[String],
        action: PolicyAction,
    ) -> Result<Option<PolicyDecision>> {
        // Check if command matches the pattern
        let regex = Regex::new(&rule.pattern)?;
        if !regex.is_match(command) {
            return Ok(None);
        }
        
        // Check denied arguments first (they take precedence)
        if let Some(denied_patterns) = &rule.denied_args {
            let args_str = args.join(" ");
            
            for pattern in denied_patterns {
                let regex = Regex::new(pattern)?;
                if regex.is_match(&args_str) {
                    return Ok(Some(PolicyDecision {
                        action: PolicyAction::Deny,
                        rule_message: rule.message.clone(),
                        matched_rule: Some(rule.pattern.clone()),
                    }));
                }
            }
        }
        
        // Check allowed arguments
        if let Some(allowed_patterns) = &rule.allowed_args {
            let args_str = args.join(" ");
            let mut any_match = false;
            
            for pattern in allowed_patterns {
                let regex = Regex::new(pattern)?;
                if regex.is_match(&args_str) {
                    any_match = true;
                    break;
                }
            }
            
            if !any_match && !allowed_patterns.is_empty() {
                return Ok(None);
            }
        }
        
        Ok(Some(PolicyDecision {
            action,
            rule_message: rule.message.clone(),
            matched_rule: Some(rule.pattern.clone()),
        }))
    }
}

/// Result of policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub action: PolicyAction,
    pub rule_message: Option<String>,
    pub matched_rule: Option<String>,
}

impl ExecutionPolicy {
    /// Create a default policy (balanced security)
    pub fn default() -> Self {
        Self {
            name: "default".to_string(),
            allowed_commands: vec![
                CommandRule {
                    pattern: "^(ls|cat|echo|pwd|date|whoami)$".to_string(),
                    allowed_args: None,
                    denied_args: None,
                    message: Some("Basic safe command".to_string()),
                },
                CommandRule {
                    pattern: "^git$".to_string(),
                    allowed_args: Some(vec![
                        "^(status|log|diff|branch|show)".to_string(),
                    ]),
                    denied_args: Some(vec![
                        "push.*--force".to_string(),
                    ]),
                    message: Some("Git read-only operations".to_string()),
                },
            ],
            denied_commands: vec![
                CommandRule {
                    pattern: "^(rm|sudo|chmod|chown)$".to_string(),
                    allowed_args: None,
                    denied_args: None,
                    message: Some("Potentially dangerous command".to_string()),
                },
            ],
            default_action: PolicyAction::RequireApproval,
            require_approval: false,
        }
    }
    
    /// Create a safe policy (very restrictive)
    pub fn safe() -> Self {
        Self {
            name: "safe".to_string(),
            allowed_commands: vec![
                CommandRule {
                    pattern: "^(ls|cat|echo|pwd|date)$".to_string(),
                    allowed_args: None,
                    denied_args: None,
                    message: Some("Read-only safe command".to_string()),
                },
            ],
            denied_commands: vec![],
            default_action: PolicyAction::Deny,
            require_approval: true,
        }
    }
    
    /// Create a strict policy (deny by default)
    pub fn strict() -> Self {
        Self {
            name: "strict".to_string(),
            allowed_commands: vec![],
            denied_commands: vec![],
            default_action: PolicyAction::Deny,
            require_approval: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_policy() {
        let engine = PolicyEngine::new();
        
        // Test allowed command
        let decision = engine.evaluate_command("ls", &["-la".to_string()]).unwrap();
        assert_eq!(decision.action, PolicyAction::Allow);
        
        // Test denied command
        let decision = engine.evaluate_command("rm", &["-rf".to_string(), "/".to_string()]).unwrap();
        assert_eq!(decision.action, PolicyAction::Deny);
        
        // Test unknown command
        let decision = engine.evaluate_command("unknown", &[]).unwrap();
        assert_eq!(decision.action, PolicyAction::RequireApproval);
    }
    
    #[test]
    fn test_git_rules() {
        let engine = PolicyEngine::new();
        
        // Test allowed git command
        let decision = engine.evaluate_command("git", &["status".to_string()]).unwrap();
        assert_eq!(decision.action, PolicyAction::Allow);
        
        // Test denied git command
        let decision = engine.evaluate_command("git", &["push".to_string(), "--force".to_string()]).unwrap();
        assert_eq!(decision.action, PolicyAction::Deny);
        
        // Test unmatched git command
        let decision = engine.evaluate_command("git", &["commit".to_string()]).unwrap();
        assert_eq!(decision.action, PolicyAction::RequireApproval);
    }
}