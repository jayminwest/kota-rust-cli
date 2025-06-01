use std::process::Command;
use anyhow::Result;
use colored::*;

use crate::context::ContextManager;
use crate::llm::{LlmProvider, ModelConfig};

/// Represents the result of executing a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl CommandResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// Trait for handling different types of commands
pub trait CommandHandler {
    fn name(&self) -> &str;
    fn usage(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, arg: &str, context: &mut ContextManager, model_config: &mut ModelConfig) -> Result<CommandResult>;
}

/// Registry for managing all available commands
pub struct CommandRegistry {
    handlers: Vec<Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
        };
        
        // Register all built-in commands
        registry.register(Box::new(QuitCommand));
        registry.register(Box::new(AddFileCommand));
        registry.register(Box::new(AddSnippetCommand));
        registry.register(Box::new(ShowContextCommand));
        registry.register(Box::new(ClearContextCommand));
        registry.register(Box::new(RunCommand));
        registry.register(Box::new(RunAddCommand));
        registry.register(Box::new(GitAddCommand));
        registry.register(Box::new(GitCommitCommand));
        registry.register(Box::new(GitStatusCommand));
        registry.register(Box::new(GitDiffCommand));
        registry.register(Box::new(HelpCommand));
        registry.register(Box::new(ProviderCommand));
        registry.register(Box::new(ModelCommand));
        registry.register(Box::new(VersionCommand));
        
        registry
    }
    
    pub fn register(&mut self, handler: Box<dyn CommandHandler>) {
        self.handlers.push(handler);
    }
    
    pub fn execute(&self, command: &str, arg: &str, context: &mut ContextManager, model_config: &mut ModelConfig) -> Result<Option<CommandResult>> {
        for handler in &self.handlers {
            if handler.name() == command {
                return Ok(Some(handler.execute(arg, context, model_config)?));
            }
        }
        Ok(None)
    }
    
    pub fn get_help(&self) -> String {
        let mut help = String::new();
        help.push_str(&format!("{}\n", "─".repeat(60).bright_blue()));
        help.push_str(&format!("{}\n", "KOTA Commands".bright_white().bold()));
        help.push_str(&format!("{}\n\n", "─".repeat(60).bright_blue()));
        
        // Group commands by category
        let categories = vec![
            ("Context Management", vec!["/add_file", "/add_snippet", "/show_context", "/clear_context"]),
            ("Command Execution", vec!["/run", "/run_add"]),
            ("Git Operations", vec!["/git_add", "/git_commit", "/git_status", "/git_diff"]),
            ("Configuration", vec!["/provider", "/model"]),
            ("General", vec!["/help", "/version", "/quit"]),
        ];
        
        for (category, commands) in categories {
            help.push_str(&format!("{}:\n", category.bright_yellow().bold()));
            for cmd in commands {
                if let Some(handler) = self.handlers.iter().find(|h| h.name() == cmd) {
                    help.push_str(&format!("  {} - {}\n", handler.usage().cyan(), handler.description()));
                }
            }
            help.push('\n');
        }
        
        help.push_str(&format!("{}:\n", "AI Interactions".bright_yellow().bold()));
        help.push_str(&format!("  {} - {}\n", "Type any message".cyan(), "Ask AI to edit files or execute commands"));
        help.push_str(&format!("  {}\n\n", "AI can suggest file edits and shell commands".dimmed()));
        
        help.push_str(&format!("{}\n", "Important: File Access Control".bright_red().bold()));
        help.push_str(&format!("  {} - {}\n", "⚠️ ".yellow(), "Files must be added to context before editing"));
        help.push_str(&format!("  {}\n", "Use /add_file before asking AI to edit".dimmed()));
        help.push_str(&format!("  {}\n", "Edits to files not in context will be blocked".dimmed()));
        
        help
    }
}

/// Helper function to execute shell commands with consistent output formatting
pub fn execute_shell_command(command: &str, args: &[&str]) -> Result<CommandResult> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    
    let output = cmd.output()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", command, e))?;
    
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    
    let mut result_output = String::new();
    
    // Format stdout
    if !stdout_str.trim().is_empty() {
        result_output.push_str(&format!("--- stdout ---\n{}\n--- end stdout ---\n", stdout_str.trim()));
    }
    
    // Format stderr
    if !stderr_str.trim().is_empty() {
        result_output.push_str(&format!("--- stderr ---\n{}\n--- end stderr ---\n", stderr_str.trim()));
    }
    
    if output.status.success() {
        Ok(CommandResult::success(result_output))
    } else {
        let error_msg = format!("Command failed with status: {}", output.status);
        Ok(CommandResult::error(error_msg))
    }
}

// Command implementations

struct QuitCommand;
impl CommandHandler for QuitCommand {
    fn name(&self) -> &str { "/quit" }
    fn usage(&self) -> &str { "/quit" }
    fn description(&self) -> &str { "Exit KOTA" }
    fn execute(&self, _arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        std::process::exit(0);
    }
}

struct AddFileCommand;
impl CommandHandler for AddFileCommand {
    fn name(&self) -> &str { "/add_file" }
    fn usage(&self) -> &str { "/add_file <path>" }
    fn description(&self) -> &str { "Add file contents to context" }
    fn execute(&self, arg: &str, context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /add_file <path_to_file>".to_string()));
        }
        
        match context.add_file(arg) {
            Ok(_) => Ok(CommandResult::success(format!("Added file: {}", arg))),
            Err(e) => Ok(CommandResult::error(format!("Error: {}", e))),
        }
    }
}

struct AddSnippetCommand;
impl CommandHandler for AddSnippetCommand {
    fn name(&self) -> &str { "/add_snippet" }
    fn usage(&self) -> &str { "/add_snippet <text>" }
    fn description(&self) -> &str { "Add text snippet to context" }
    fn execute(&self, arg: &str, context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /add_snippet <text_snippet>".to_string()));
        }
        
        context.add_snippet(arg.to_string());
        Ok(CommandResult::success("Snippet added to context".to_string()))
    }
}

struct ShowContextCommand;
impl CommandHandler for ShowContextCommand {
    fn name(&self) -> &str { "/show_context" }
    fn usage(&self) -> &str { "/show_context" }
    fn description(&self) -> &str { "Display current context" }
    fn execute(&self, _arg: &str, context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        context.show_context();
        Ok(CommandResult::success("Context displayed".to_string()))
    }
}

struct ClearContextCommand;
impl CommandHandler for ClearContextCommand {
    fn name(&self) -> &str { "/clear_context" }
    fn usage(&self) -> &str { "/clear_context" }
    fn description(&self) -> &str { "Clear all context" }
    fn execute(&self, _arg: &str, context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        context.clear_context();
        Ok(CommandResult::success("Context cleared".to_string()))
    }
}

struct RunCommand;
impl CommandHandler for RunCommand {
    fn name(&self) -> &str { "/run" }
    fn usage(&self) -> &str { "/run <command>" }
    fn description(&self) -> &str { "Execute shell command" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /run <shell_command_here>".to_string()));
        }
        
        execute_shell_command("sh", &["-c", arg])
    }
}

struct RunAddCommand;
impl CommandHandler for RunAddCommand {
    fn name(&self) -> &str { "/run_add" }
    fn usage(&self) -> &str { "/run_add <command>" }
    fn description(&self) -> &str { "Execute command and add output to context" }
    fn execute(&self, arg: &str, context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /run_add <shell_command_here>".to_string()));
        }
        
        let result = execute_shell_command("sh", &["-c", arg])?;
        
        // Add command output to context
        if !result.output.trim().is_empty() {
            context.add_snippet(format!("Output of command '{}': \n{}", arg, result.output));
        } else if let Some(error) = &result.error {
            context.add_snippet(format!("Error output of command '{}': \n{}", arg, error));
        }
        
        Ok(result)
    }
}

struct GitAddCommand;
impl CommandHandler for GitAddCommand {
    fn name(&self) -> &str { "/git_add" }
    fn usage(&self) -> &str { "/git_add <file>" }
    fn description(&self) -> &str { "Stage file for commit" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /git_add <file_path>".to_string()));
        }
        
        execute_shell_command("git", &["add", arg])
    }
}

struct GitCommitCommand;
impl CommandHandler for GitCommitCommand {
    fn name(&self) -> &str { "/git_commit" }
    fn usage(&self) -> &str { "/git_commit \"<message>\"" }
    fn description(&self) -> &str { "Create git commit" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error("Usage: /git_commit \"<commit_message>\"".to_string()));
        }
        
        execute_shell_command("git", &["commit", "-m", arg])
    }
}

struct GitStatusCommand;
impl CommandHandler for GitStatusCommand {
    fn name(&self) -> &str { "/git_status" }
    fn usage(&self) -> &str { "/git_status" }
    fn description(&self) -> &str { "Show git status" }
    fn execute(&self, _arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        execute_shell_command("git", &["status"])
    }
}

struct GitDiffCommand;
impl CommandHandler for GitDiffCommand {
    fn name(&self) -> &str { "/git_diff" }
    fn usage(&self) -> &str { "/git_diff [<path>]" }
    fn description(&self) -> &str { "Show git diff" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            execute_shell_command("git", &["diff"])
        } else {
            execute_shell_command("git", &["diff", arg])
        }
    }
}

struct HelpCommand;
impl CommandHandler for HelpCommand {
    fn name(&self) -> &str { "/help" }
    fn usage(&self) -> &str { "/help" }
    fn description(&self) -> &str { "Show this help message" }
    fn execute(&self, _arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        // This will be handled by the registry
        Ok(CommandResult::success("Help displayed".to_string()))
    }
}

struct ProviderCommand;
impl CommandHandler for ProviderCommand {
    fn name(&self) -> &str { "/provider" }
    fn usage(&self) -> &str { "/provider <ollama|gemini|anthropic>" }
    fn description(&self) -> &str { "Switch LLM provider" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            let current = match model_config.provider {
                LlmProvider::Ollama => "Ollama",
                LlmProvider::Gemini => "Google Gemini",
                LlmProvider::Anthropic => "Anthropic Claude",
            };
            return Ok(CommandResult::success(format!("Current provider: {}\nUsage: /provider <ollama|gemini|anthropic>", current)));
        }
        
        match arg.to_lowercase().as_str() {
            "ollama" => {
                model_config.provider = LlmProvider::Ollama;
                Ok(CommandResult::success("Switched to Ollama provider".to_string()))
            }
            "gemini" => {
                model_config.provider = LlmProvider::Gemini;
                Ok(CommandResult::success("Switched to Gemini provider".to_string()))
            }
            "anthropic" => {
                model_config.provider = LlmProvider::Anthropic;
                Ok(CommandResult::success("Switched to Anthropic provider".to_string()))
            }
            _ => Ok(CommandResult::error("Invalid provider. Use: ollama, gemini, or anthropic".to_string()))
        }
    }
}

struct ModelCommand;
impl CommandHandler for ModelCommand {
    fn name(&self) -> &str { "/model" }
    fn usage(&self) -> &str { "/model <model_name>" }
    fn description(&self) -> &str { "Set model for current provider" }
    fn execute(&self, arg: &str, _context: &mut ContextManager, model_config: &mut ModelConfig) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::success(format!("Current model: {}\nUsage: /model <model_name>", model_config.get_model_name())));
        }
        
        model_config.model_name = Some(arg.to_string());
        Ok(CommandResult::success(format!("Model set to: {}", arg)))
    }
}

struct VersionCommand;
impl CommandHandler for VersionCommand {
    fn name(&self) -> &str { "/version" }
    fn usage(&self) -> &str { "/version" }
    fn description(&self) -> &str { "Show KOTA version" }
    fn execute(&self, _arg: &str, _context: &mut ContextManager, _model_config: &mut ModelConfig) -> Result<CommandResult> {
        Ok(CommandResult::success(format!("KOTA version: {}", env!("CARGO_PKG_VERSION"))))
    }
}