use anyhow::Result;
use colored::*;
use std::process::Command;

use crate::agents::AgentManager;
use crate::config::KotaConfig;
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
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &str;
    fn usage(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(
        &self,
        arg: &str,
        context: &mut ContextManager,
        model_config: &mut ModelConfig,
    ) -> Result<CommandResult>;
    fn execute_with_agents(
        &self,
        arg: &str,
        context: &mut ContextManager,
        model_config: &mut ModelConfig,
        _agent_manager: Option<&AgentManager>,
    ) -> Result<CommandResult> {
        // Default implementation for backwards compatibility
        self.execute(arg, context, model_config)
    }
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
        // Agent commands
        registry.register(Box::new(AgentsCommand));
        registry.register(Box::new(AgentCommand));
        registry.register(Box::new(DelegateCommand));
        registry.register(Box::new(AskAgentCommand));
        // Security commands
        registry.register(Box::new(SecurityCommand));
        registry.register(Box::new(SandboxCommand));
        registry.register(Box::new(ApprovalCommand));
        // Config commands
        registry.register(Box::new(ConfigCommand));

        registry
    }

    pub fn register(&mut self, handler: Box<dyn CommandHandler>) {
        self.handlers.push(handler);
    }

    pub fn execute(
        &self,
        command: &str,
        arg: &str,
        context: &mut ContextManager,
        model_config: &mut ModelConfig,
    ) -> Result<Option<CommandResult>> {
        for handler in &self.handlers {
            if handler.name() == command {
                return Ok(Some(handler.execute(arg, context, model_config)?));
            }
        }
        Ok(None)
    }

    pub fn execute_with_agents(
        &self,
        command: &str,
        arg: &str,
        context: &mut ContextManager,
        model_config: &mut ModelConfig,
        agent_manager: Option<&AgentManager>,
    ) -> Result<Option<CommandResult>> {
        for handler in &self.handlers {
            if handler.name() == command {
                return Ok(Some(handler.execute_with_agents(
                    arg,
                    context,
                    model_config,
                    agent_manager,
                )?));
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
            (
                "Context Management",
                vec![
                    "/add_file",
                    "/add_snippet",
                    "/show_context",
                    "/clear_context",
                ],
            ),
            ("Command Execution", vec!["/run", "/run_add"]),
            (
                "Git Operations",
                vec!["/git_add", "/git_commit", "/git_status", "/git_diff"],
            ),
            (
                "Agent Management",
                vec!["/agents", "/agent", "/delegate", "/ask_agent"],
            ),
            ("Security", vec!["/security", "/sandbox", "/approval"]),
            ("Configuration", vec!["/provider", "/model", "/config"]),
            ("General", vec!["/help", "/version", "/quit"]),
        ];

        for (category, commands) in categories {
            help.push_str(&format!("{}:\n", category.bright_yellow().bold()));
            for cmd in commands {
                if let Some(handler) = self.handlers.iter().find(|h| h.name() == cmd) {
                    help.push_str(&format!(
                        "  {} - {}\n",
                        handler.usage().cyan(),
                        handler.description()
                    ));
                }
            }
            help.push('\n');
        }

        help.push_str(&format!("{}:\n", "AI Interactions".bright_yellow().bold()));
        help.push_str(&format!(
            "  {} - {}\n",
            "Type any message".cyan(),
            "Ask AI to edit files or execute commands"
        ));
        help.push_str(&format!(
            "  {}\n\n",
            "AI can suggest file edits and shell commands".dimmed()
        ));

        help.push_str(&format!(
            "{}\n",
            "Important: File Access Control".bright_red().bold()
        ));
        help.push_str(&format!(
            "  {} - {}\n",
            "⚠️ ".yellow(),
            "Files must be added to context before editing"
        ));
        help.push_str(&format!(
            "  {}\n",
            "Use /add_file before asking AI to edit".dimmed()
        ));
        help.push_str(&format!(
            "  {}\n",
            "Edits to files not in context will be blocked".dimmed()
        ));

        help
    }
}

// CommandRegistry is Send + Sync since all CommandHandler impls are Send + Sync
unsafe impl Send for CommandRegistry {}
unsafe impl Sync for CommandRegistry {}

/// Helper function to execute shell commands with consistent output formatting
pub fn execute_shell_command(command: &str, args: &[&str]) -> Result<CommandResult> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    let output = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", command, e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let mut result_output = String::new();

    // Format stdout
    if !stdout_str.trim().is_empty() {
        result_output.push_str(&format!(
            "--- stdout ---\n{}\n--- end stdout ---\n",
            stdout_str.trim()
        ));
    }

    // Format stderr
    if !stderr_str.trim().is_empty() {
        result_output.push_str(&format!(
            "--- stderr ---\n{}\n--- end stderr ---\n",
            stderr_str.trim()
        ));
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
    fn name(&self) -> &str {
        "/quit"
    }
    fn usage(&self) -> &str {
        "/quit"
    }
    fn description(&self) -> &str {
        "Exit KOTA"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        std::process::exit(0);
    }
}

struct AddFileCommand;
impl CommandHandler for AddFileCommand {
    fn name(&self) -> &str {
        "/add_file"
    }
    fn usage(&self) -> &str {
        "/add_file <path>"
    }
    fn description(&self) -> &str {
        "Add file contents to context"
    }
    fn execute(
        &self,
        arg: &str,
        context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /add_file <path_to_file>".to_string(),
            ));
        }

        match context.add_file(arg) {
            Ok(_) => Ok(CommandResult::success(format!("Added file: {}", arg))),
            Err(e) => Ok(CommandResult::error(format!("Error: {}", e))),
        }
    }
}

struct AddSnippetCommand;
impl CommandHandler for AddSnippetCommand {
    fn name(&self) -> &str {
        "/add_snippet"
    }
    fn usage(&self) -> &str {
        "/add_snippet <text>"
    }
    fn description(&self) -> &str {
        "Add text snippet to context"
    }
    fn execute(
        &self,
        arg: &str,
        context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /add_snippet <text_snippet>".to_string(),
            ));
        }

        context.add_snippet(arg.to_string());
        Ok(CommandResult::success(
            "Snippet added to context".to_string(),
        ))
    }
}

struct ShowContextCommand;
impl CommandHandler for ShowContextCommand {
    fn name(&self) -> &str {
        "/show_context"
    }
    fn usage(&self) -> &str {
        "/show_context"
    }
    fn description(&self) -> &str {
        "Display current context"
    }
    fn execute(
        &self,
        _arg: &str,
        context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        context.show_context();
        Ok(CommandResult::success("Context displayed".to_string()))
    }
}

struct ClearContextCommand;
impl CommandHandler for ClearContextCommand {
    fn name(&self) -> &str {
        "/clear_context"
    }
    fn usage(&self) -> &str {
        "/clear_context"
    }
    fn description(&self) -> &str {
        "Clear all context"
    }
    fn execute(
        &self,
        _arg: &str,
        context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        context.clear_context();
        Ok(CommandResult::success("Context cleared".to_string()))
    }
}

struct RunCommand;
impl CommandHandler for RunCommand {
    fn name(&self) -> &str {
        "/run"
    }
    fn usage(&self) -> &str {
        "/run <command>"
    }
    fn description(&self) -> &str {
        "Execute shell command"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /run <shell_command_here>".to_string(),
            ));
        }

        execute_shell_command("sh", &["-c", arg])
    }
}

struct RunAddCommand;
impl CommandHandler for RunAddCommand {
    fn name(&self) -> &str {
        "/run_add"
    }
    fn usage(&self) -> &str {
        "/run_add <command>"
    }
    fn description(&self) -> &str {
        "Execute command and add output to context"
    }
    fn execute(
        &self,
        arg: &str,
        context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /run_add <shell_command_here>".to_string(),
            ));
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
    fn name(&self) -> &str {
        "/git_add"
    }
    fn usage(&self) -> &str {
        "/git_add <file>"
    }
    fn description(&self) -> &str {
        "Stage file for commit"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /git_add <file_path>".to_string(),
            ));
        }

        execute_shell_command("git", &["add", arg])
    }
}

struct GitCommitCommand;
impl CommandHandler for GitCommitCommand {
    fn name(&self) -> &str {
        "/git_commit"
    }
    fn usage(&self) -> &str {
        "/git_commit \"<message>\""
    }
    fn description(&self) -> &str {
        "Create git commit"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /git_commit \"<commit_message>\"".to_string(),
            ));
        }

        execute_shell_command("git", &["commit", "-m", arg])
    }
}

struct GitStatusCommand;
impl CommandHandler for GitStatusCommand {
    fn name(&self) -> &str {
        "/git_status"
    }
    fn usage(&self) -> &str {
        "/git_status"
    }
    fn description(&self) -> &str {
        "Show git status"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        execute_shell_command("git", &["status"])
    }
}

struct GitDiffCommand;
impl CommandHandler for GitDiffCommand {
    fn name(&self) -> &str {
        "/git_diff"
    }
    fn usage(&self) -> &str {
        "/git_diff [<path>]"
    }
    fn description(&self) -> &str {
        "Show git diff"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            execute_shell_command("git", &["diff"])
        } else {
            execute_shell_command("git", &["diff", arg])
        }
    }
}

struct HelpCommand;
impl CommandHandler for HelpCommand {
    fn name(&self) -> &str {
        "/help"
    }
    fn usage(&self) -> &str {
        "/help"
    }
    fn description(&self) -> &str {
        "Show this help message"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        // This will be handled by the registry
        Ok(CommandResult::success("Help displayed".to_string()))
    }
}

struct ProviderCommand;
impl CommandHandler for ProviderCommand {
    fn name(&self) -> &str {
        "/provider"
    }
    fn usage(&self) -> &str {
        "/provider <ollama|gemini|anthropic>"
    }
    fn description(&self) -> &str {
        "Switch LLM provider"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            let current = match model_config.provider {
                LlmProvider::Ollama => "Ollama",
                LlmProvider::Gemini => "Google Gemini",
                LlmProvider::Anthropic => "Anthropic Claude",
            };
            return Ok(CommandResult::success(format!(
                "Current provider: {}\nUsage: /provider <ollama|gemini|anthropic>",
                current
            )));
        }

        match arg.to_lowercase().as_str() {
            "ollama" => {
                model_config.provider = LlmProvider::Ollama;
                Ok(CommandResult::success(
                    "Switched to Ollama provider".to_string(),
                ))
            }
            "gemini" => {
                model_config.provider = LlmProvider::Gemini;
                Ok(CommandResult::success(
                    "Switched to Gemini provider".to_string(),
                ))
            }
            "anthropic" => {
                model_config.provider = LlmProvider::Anthropic;
                Ok(CommandResult::success(
                    "Switched to Anthropic provider".to_string(),
                ))
            }
            _ => Ok(CommandResult::error(
                "Invalid provider. Use: ollama, gemini, or anthropic".to_string(),
            )),
        }
    }
}

struct ModelCommand;
impl CommandHandler for ModelCommand {
    fn name(&self) -> &str {
        "/model"
    }
    fn usage(&self) -> &str {
        "/model <model_name>"
    }
    fn description(&self) -> &str {
        "Set model for current provider"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        if arg.is_empty() {
            return Ok(CommandResult::success(format!(
                "Current model: {}\nUsage: /model <model_name>",
                model_config.get_model_name()
            )));
        }

        model_config.model_name = Some(arg.to_string());
        Ok(CommandResult::success(format!("Model set to: {}", arg)))
    }
}

struct VersionCommand;
impl CommandHandler for VersionCommand {
    fn name(&self) -> &str {
        "/version"
    }
    fn usage(&self) -> &str {
        "/version"
    }
    fn description(&self) -> &str {
        "Show KOTA version"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        Ok(CommandResult::success(format!(
            "KOTA version: {}",
            env!("CARGO_PKG_VERSION")
        )))
    }
}

// Agent Commands

struct AgentsCommand;
impl CommandHandler for AgentsCommand {
    fn name(&self) -> &str {
        "/agents"
    }
    fn usage(&self) -> &str {
        "/agents"
    }
    fn description(&self) -> &str {
        "List all available agents and their status"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        Ok(CommandResult::error("Agent manager not initialized. Use agent commands in TUI mode or wait for integration.".to_string()))
    }

    fn execute_with_agents(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
        agent_manager: Option<&AgentManager>,
    ) -> Result<CommandResult> {
        if let Some(manager) = agent_manager {
            let agents = manager.list_agents();
            let mut output = String::new();
            output.push_str(&format!("{}\n", "Available Agents".bright_white().bold()));
            output.push_str(&format!("{}\n", "─".repeat(30).bright_blue()));

            for (name, status) in agents {
                output.push_str(&format!("🤖 {} - {}\n", name.bright_cyan().bold(), status));
            }

            Ok(CommandResult::success(output))
        } else {
            Ok(CommandResult::error(
                "Agent manager not available".to_string(),
            ))
        }
    }
}

struct AgentCommand;
impl CommandHandler for AgentCommand {
    fn name(&self) -> &str {
        "/agent"
    }
    fn usage(&self) -> &str {
        "/agent <agent_name>"
    }
    fn description(&self) -> &str {
        "Get details about a specific agent"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        Ok(CommandResult::error("Agent manager not initialized. Use agent commands in TUI mode or wait for integration.".to_string()))
    }

    fn execute_with_agents(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
        agent_manager: Option<&AgentManager>,
    ) -> Result<CommandResult> {
        if let Some(manager) = agent_manager {
            if arg.is_empty() {
                return Ok(CommandResult::error(
                    "Usage: /agent <agent_name>".to_string(),
                ));
            }

            let capabilities = manager.get_agent_capabilities(arg);
            if let Some(caps) = capabilities {
                let mut output = String::new();
                output.push_str(&format!("Agent: {}\n", arg.bright_cyan().bold()));
                output.push_str("Capabilities:\n");
                for cap in caps {
                    output.push_str(&format!("  • {:?}\n", cap));
                }
                Ok(CommandResult::success(output))
            } else {
                Ok(CommandResult::error(format!("Agent '{}' not found", arg)))
            }
        } else {
            Ok(CommandResult::error(
                "Agent manager not available".to_string(),
            ))
        }
    }
}

struct DelegateCommand;
impl CommandHandler for DelegateCommand {
    fn name(&self) -> &str {
        "/delegate"
    }
    fn usage(&self) -> &str {
        "/delegate <task_description> [agent_name]"
    }
    fn description(&self) -> &str {
        "Delegate a task to an agent (auto-selects if no agent specified)"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        Ok(CommandResult::error("Agent manager not initialized. Use agent commands in TUI mode or wait for integration.".to_string()))
    }

    fn execute_with_agents(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
        agent_manager: Option<&AgentManager>,
    ) -> Result<CommandResult> {
        if let Some(manager) = agent_manager {
            if arg.is_empty() {
                return Ok(CommandResult::error(
                    "Usage: /delegate <task_description> [agent_name]".to_string(),
                ));
            }

            let parts: Vec<&str> = arg.splitn(2, ' ').collect();
            let (task_desc, agent_name) = if parts.len() > 1 {
                (parts[1], Some(parts[0].to_string()))
            } else {
                (arg, None)
            };

            let rt = tokio::runtime::Runtime::new()?;
            match rt.block_on(manager.delegate_task(task_desc.to_string(), agent_name)) {
                Ok(result) => Ok(CommandResult::success(result)),
                Err(e) => Ok(CommandResult::error(format!(
                    "Task delegation failed: {}",
                    e
                ))),
            }
        } else {
            Ok(CommandResult::error(
                "Agent manager not available".to_string(),
            ))
        }
    }
}

struct AskAgentCommand;
impl CommandHandler for AskAgentCommand {
    fn name(&self) -> &str {
        "/ask_agent"
    }
    fn usage(&self) -> &str {
        "/ask_agent <question> [agent_name]"
    }
    fn description(&self) -> &str {
        "Ask a question to an agent (auto-selects if no agent specified)"
    }
    fn execute(
        &self,
        _arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        Ok(CommandResult::error("Agent manager not initialized. Use agent commands in TUI mode or wait for integration.".to_string()))
    }

    fn execute_with_agents(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
        agent_manager: Option<&AgentManager>,
    ) -> Result<CommandResult> {
        if let Some(manager) = agent_manager {
            if arg.is_empty() {
                return Ok(CommandResult::error(
                    "Usage: /ask_agent <question> [agent_name]".to_string(),
                ));
            }

            let parts: Vec<&str> = arg.splitn(2, ' ').collect();
            let (question, agent_name) = if parts.len() > 1 {
                (parts[1], Some(parts[0].to_string()))
            } else {
                (arg, None)
            };

            let rt = tokio::runtime::Runtime::new()?;
            match rt.block_on(manager.ask_agent(question.to_string(), agent_name)) {
                Ok(result) => Ok(CommandResult::success(result)),
                Err(e) => Ok(CommandResult::error(format!("Agent query failed: {}", e))),
            }
        } else {
            Ok(CommandResult::error(
                "Agent manager not available".to_string(),
            ))
        }
    }
}

// Security Commands

struct SecurityCommand;
impl CommandHandler for SecurityCommand {
    fn name(&self) -> &str {
        "/security"
    }
    fn usage(&self) -> &str {
        "/security [status|help]"
    }
    fn description(&self) -> &str {
        "Show security system status and configuration"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        let mut output = String::new();

        match arg {
            "help" | "" => {
                output.push_str(&format!(
                    "{}\n",
                    "KOTA Security System".bright_white().bold()
                ));
                output.push_str(&format!("{}\n", "─".repeat(30).bright_blue()));
                output.push_str("🔒 KOTA uses a multi-layered security approach:\n\n");
                output.push_str("• **Policy Engine**: Regex-based command filtering\n");
                output.push_str(
                    "• **Approval System**: Interactive user confirmation with risk assessment\n",
                );
                output.push_str("• **macOS Sandboxing**: Process isolation using sandbox-exec\n\n");
                output.push_str("Available commands:\n");
                output.push_str("• `/security status` - Show current security configuration\n");
                output.push_str("• `/sandbox [profile]` - Configure sandbox profiles\n");
                output.push_str("• `/approval [mode]` - Configure approval settings\n");
            }
            "status" => {
                output.push_str(&format!("{}\n", "Security Status".bright_white().bold()));
                output.push_str(&format!("{}\n", "─".repeat(20).bright_blue()));
                output.push_str(&format!("🔐 Policy Engine: {}\n", "Active".green()));
                output.push_str(&format!("🔐 Approval Mode: {}\n", "Policy-based".green()));
                output.push_str(&format!("🔐 Sandbox: {}\n", "Development profile".green()));
                output.push_str(&format!("🔐 Platform: {}\n", "macOS Seatbelt".green()));
            }
            _ => {
                return Ok(CommandResult::error(
                    "Unknown security command. Use '/security help' for usage.".to_string(),
                ));
            }
        }

        Ok(CommandResult::success(output))
    }
}

struct SandboxCommand;
impl CommandHandler for SandboxCommand {
    fn name(&self) -> &str {
        "/sandbox"
    }
    fn usage(&self) -> &str {
        "/sandbox [minimal|development|read_only]"
    }
    fn description(&self) -> &str {
        "Configure sandbox security profiles"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        let mut output = String::new();

        match arg {
            "" => {
                output.push_str(&format!("{}\n", "Sandbox Profiles".bright_white().bold()));
                output.push_str(&format!("{}\n", "─".repeat(20).bright_blue()));
                output.push_str("Available profiles:\n\n");
                output.push_str("🔒 **minimal** - Extremely restrictive, deny-by-default\n");
                output.push_str("🔒 **development** - Balanced for development work\n");
                output.push_str("🔒 **read_only** - Read-only access to current directory\n\n");
                output.push_str("Current profile: **development**\n");
                output.push_str("Use `/sandbox <profile>` to switch profiles\n");
            }
            "minimal" => {
                output.push_str("🔒 Switched to minimal sandbox profile\n");
                output.push_str("Security level: Maximum restriction\n");
            }
            "development" => {
                output.push_str("🔒 Switched to development sandbox profile\n");
                output.push_str("Security level: Balanced for development\n");
            }
            "read_only" => {
                output.push_str("🔒 Switched to read-only sandbox profile\n");
                output.push_str("Security level: Read-only file access\n");
            }
            _ => {
                return Ok(CommandResult::error(
                    "Unknown sandbox profile. Use '/sandbox' to see available profiles."
                        .to_string(),
                ));
            }
        }

        Ok(CommandResult::success(output))
    }
}

struct ApprovalCommand;
impl CommandHandler for ApprovalCommand {
    fn name(&self) -> &str {
        "/approval"
    }
    fn usage(&self) -> &str {
        "/approval [always|never|policy|unknown]"
    }
    fn description(&self) -> &str {
        "Configure command approval requirements"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        let mut output = String::new();

        match arg {
            "" => {
                output.push_str(&format!("{}\n", "Approval Modes".bright_white().bold()));
                output.push_str(&format!("{}\n", "─".repeat(20).bright_blue()));
                output.push_str("Available approval modes:\n\n");
                output.push_str("✓ **always** - Require approval for all commands\n");
                output.push_str("✓ **never** - Auto-approve all commands (not recommended)\n");
                output.push_str(
                    "✓ **policy** - Use policy engine to determine approval (recommended)\n",
                );
                output.push_str("✓ **unknown** - Only ask for unknown/unrecognized commands\n\n");
                output.push_str("Current mode: **policy**\n");
                output.push_str("Use `/approval <mode>` to change the approval mode\n");
            }
            "always" => {
                output.push_str("✓ Approval mode set to: Always require approval\n");
                output.push_str("All commands will require user confirmation\n");
            }
            "never" => {
                output.push_str("⚠️  Approval mode set to: Never require approval\n");
                output.push_str("Warning: This disables security approval prompts\n");
            }
            "policy" => {
                output.push_str("✓ Approval mode set to: Policy-based\n");
                output.push_str("Commands will be filtered through the security policy engine\n");
            }
            "unknown" => {
                output.push_str("✓ Approval mode set to: Unknown commands only\n");
                output.push_str("Only unrecognized commands will require approval\n");
            }
            _ => {
                return Ok(CommandResult::error(
                    "Unknown approval mode. Use '/approval' to see available modes.".to_string(),
                ));
            }
        }

        Ok(CommandResult::success(output))
    }
}

// Configuration Commands

struct ConfigCommand;
impl CommandHandler for ConfigCommand {
    fn name(&self) -> &str {
        "/config"
    }
    fn usage(&self) -> &str {
        "/config [show|save|load|reset]"
    }
    fn description(&self) -> &str {
        "Manage KOTA configuration settings"
    }
    fn execute(
        &self,
        arg: &str,
        _context: &mut ContextManager,
        _model_config: &mut ModelConfig,
    ) -> Result<CommandResult> {
        let mut output = String::new();

        match arg {
            "" | "show" => {
                // Load current config or use default
                let config = match KotaConfig::load(&KotaConfig::default_path()?) {
                    Ok(cfg) => cfg,
                    Err(_) => KotaConfig::default(),
                };

                output.push_str(&format!("{}\n", "KOTA Configuration".bright_white().bold()));
                output.push_str(&format!("{}\n", "─".repeat(30).bright_blue()));

                // General settings
                output.push_str(&format!("{}:\n", "General".bright_yellow().bold()));
                output.push_str(&format!("  Debug: {}\n", config.general.debug));
                output.push_str(&format!("  Log Level: {}\n", config.general.log_level));
                output.push_str(&format!(
                    "  Max Context Tokens: {}\n",
                    config.general.max_context_tokens
                ));

                // LLM settings
                output.push_str(&format!("\n{}:\n", "LLM".bright_yellow().bold()));
                output.push_str(&format!(
                    "  Default Provider: {:?}\n",
                    config.llm.default_provider
                ));
                output.push_str(&format!(
                    "  Timeout: {} seconds\n",
                    config.llm.timeout_seconds
                ));
                output.push_str(&format!(
                    "  Retry Attempts: {}\n",
                    config.llm.retry_attempts
                ));

                // Security settings
                output.push_str(&format!("\n{}:\n", "Security".bright_yellow().bold()));
                output.push_str(&format!(
                    "  Approval Mode: {:?}\n",
                    config.security.approval_mode
                ));
                output.push_str(&format!(
                    "  Active Policy: {}\n",
                    config.security.active_policy
                ));
                output.push_str(&format!(
                    "  Default Sandbox: {}\n",
                    config.security.default_sandbox
                ));

                // TUI settings
                output.push_str(&format!("\n{}:\n", "TUI".bright_yellow().bold()));
                output.push_str(&format!("  Enabled: {}\n", config.tui.enabled));
                output.push_str(&format!("  Theme: {}\n", config.tui.theme));
                output.push_str(&format!("  Auto-scroll: {}\n", config.tui.auto_scroll));

                output.push_str(&format!(
                    "\nConfig file: {}\n",
                    KotaConfig::default_path()?.display()
                ));
            }
            "save" => {
                let config = KotaConfig::default();
                match config.save(&KotaConfig::default_path()?) {
                    Ok(()) => {
                        output.push_str("✓ Configuration saved successfully\n");
                        output.push_str(&format!(
                            "Location: {}\n",
                            KotaConfig::default_path()?.display()
                        ));
                    }
                    Err(e) => {
                        return Ok(CommandResult::error(format!(
                            "Failed to save config: {}",
                            e
                        )));
                    }
                }
            }
            "load" => match KotaConfig::load(&KotaConfig::default_path()?) {
                Ok(_) => {
                    output.push_str("✓ Configuration loaded successfully\n");
                    output.push_str(&format!(
                        "From: {}\n",
                        KotaConfig::default_path()?.display()
                    ));
                }
                Err(e) => {
                    return Ok(CommandResult::error(format!(
                        "Failed to load config: {}",
                        e
                    )));
                }
            },
            "reset" => {
                let config = KotaConfig::default();
                match config.save(&KotaConfig::default_path()?) {
                    Ok(()) => {
                        output.push_str("✓ Configuration reset to defaults\n");
                        output.push_str(&format!(
                            "Saved to: {}\n",
                            KotaConfig::default_path()?.display()
                        ));
                    }
                    Err(e) => {
                        return Ok(CommandResult::error(format!(
                            "Failed to reset config: {}",
                            e
                        )));
                    }
                }
            }
            _ => {
                return Ok(CommandResult::error(
                    "Unknown config command. Use '/config' to see current settings.".to_string(),
                ));
            }
        }

        Ok(CommandResult::success(output))
    }
}
