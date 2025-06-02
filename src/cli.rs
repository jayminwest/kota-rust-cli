use std::io;
use anyhow::Result;
use colored::*;

use crate::context::ContextManager;
use crate::llm::{LlmProvider, ModelConfig};
use crate::commands::{CommandRegistry, CommandResult};
use crate::{input, thinking, sr_parser, editor, cmd_parser, tui, render_markdown};
use crate::agents::AgentManager;
use crate::memory::MemoryManager;
use crate::security::{SecureExecutor, ApprovalMode};

/// Runs the classic CLI interface
pub async fn run_classic_cli(_context_manager: ContextManager, _model_config: ModelConfig) -> Result<()> {
    let header_width = 60;
    println!("{}", "═".repeat(header_width).bright_blue());
    println!("{}", "KOTA - AI Coding Assistant".bright_white().bold());
    println!("{}", "═".repeat(header_width).bright_blue());
    
    let mut context_manager = ContextManager::new();
    let mut model_config = ModelConfig::default();
    let command_registry = CommandRegistry::new();
    
    // Initialize agents
    let memory_manager = MemoryManager::new()?;
    let shared_context = std::sync::Arc::new(tokio::sync::Mutex::new(ContextManager::new()));
    let agent_manager = AgentManager::new(
        shared_context.clone(),
        model_config.clone(),
        std::sync::Arc::new(tokio::sync::Mutex::new(memory_manager)),
    ).await?;
    
    // Initialize secure executor
    let mut secure_executor = SecureExecutor::new(ApprovalMode::Policy);
    secure_executor.set_context_manager(shared_context.clone());
    secure_executor.set_model_config(std::sync::Arc::new(tokio::sync::Mutex::new(model_config.clone())));
    
    // Show provider status and check API key
    show_provider_status(&model_config);
    
    println!("{}", "─".repeat(header_width).dimmed());
    println!("{} Type '/help' for available commands", "💡".yellow());
    println!("{} Type anything else to chat with AI", "💬".bright_blue());
    println!("{} Type '/agents' to see available AI agents", "🤖".bright_green());
    println!();

    loop {
        let user_input = input::read_line_with_shortcuts()?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() {
            continue;
        }
        
        if trimmed_input.starts_with('/') {
            if let Err(e) = handle_command(trimmed_input, &mut context_manager, &mut model_config, &command_registry, &agent_manager, &shared_context).await {
                eprintln!("Command error: {}", e);
            }
        } else if let Err(e) = handle_ai_interaction(trimmed_input, &mut context_manager, &model_config, &secure_executor).await {
            eprintln!("Error in AI interaction: {}", e);
        }
        
        println!(); // Add spacing between interactions
    }
}

fn show_provider_status(model_config: &ModelConfig) {
    match model_config.provider {
        LlmProvider::Ollama => println!("{} {}", "Provider:".dimmed(), "Ollama (local)".cyan()),
        LlmProvider::Gemini => {
            if std::env::var("GEMINI_API_KEY").is_ok() {
                println!("{} {}", "Provider:".dimmed(), "Google Gemini (cloud)".cyan());
            } else {
                println!("{} {}", "Provider:".dimmed(), "Google Gemini (cloud) - Missing API key".yellow());
                println!("{} export GEMINI_API_KEY=your_api_key", "Set with:".dimmed());
                println!("{} Use /provider ollama to switch to local Ollama", "Alternative:".dimmed());
            }
        }
        LlmProvider::Anthropic => {
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                println!("{} {}", "Provider:".dimmed(), "Anthropic Claude (cloud)".cyan());
            } else {
                println!("{} {}", "Provider:".dimmed(), "Anthropic Claude (cloud) - Missing API key".yellow());
                println!("{} export ANTHROPIC_API_KEY=your_api_key", "Set with:".dimmed());
                println!("{} Use /provider ollama to switch to local Ollama", "Alternative:".dimmed());
            }
        }
    }
}

async fn handle_command(
    input: &str,
    context_manager: &mut ContextManager,
    model_config: &mut ModelConfig,
    command_registry: &CommandRegistry,
    agent_manager: &AgentManager,
    shared_context: &std::sync::Arc<tokio::sync::Mutex<ContextManager>>,
) -> Result<()> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let command = parts[0];
    let arg = if parts.len() > 1 { parts[1] } else { "" };

    // Special handling for commands that need different behavior
    match command {
        "/help" => {
            print!("{}", command_registry.get_help());
            Ok(())
        }
        "/tui" => {
            println!("Switching to TUI mode...");
            // Create new instances for TUI mode
            let new_context = ContextManager::new();
            let new_config = ModelConfig::default();
            tui::run_tui(new_context, new_config).await
        }
        "/quit" => {
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", "Goodbye!".bright_white());
            std::process::exit(0);
        }
        _ => {
            // Sync context to shared context before agent commands
            {
                let mut shared = shared_context.lock().await;
                *shared = ContextManager::new();
                for item in &context_manager.items {
                    shared.add_snippet(item.clone());
                }
                for path in &context_manager.file_paths {
                    let _ = shared.add_file(path);
                }
            }
            
            match command_registry.execute_with_agents(command, arg, context_manager, model_config, Some(agent_manager))? {
                Some(result) => {
                    display_command_result(result);
                    Ok(())
                }
                None => {
                    println!("Unknown command: {}. Type '/help' for available commands.", command);
                    Ok(())
                }
            }
        }
    }
}

fn display_command_result(result: CommandResult) {
    match result {
        CommandResult { success: true, output, .. } => {
            if !output.trim().is_empty() {
                println!("{}", output);
            }
        }
        CommandResult { success: false, error: Some(error), .. } => {
            println!("{} {}", "Error:".red(), error);
        }
        _ => {}
    }
}

async fn handle_ai_interaction(
    input: &str,
    context_manager: &mut ContextManager,
    model_config: &ModelConfig,
    secure_executor: &SecureExecutor,
) -> Result<()> {
    let spinner = thinking::show_llm_thinking();
    
    // Get the formatted context
    let context_string = context_manager.get_formatted_context();
    
    let llm_response = crate::llm::ask_model_with_config(input, &context_string, model_config).await;
    spinner.finish();
    
    match llm_response {
        Ok(response) => {
            // Render the response using termimad
            let _ = render_markdown(&response);
            
            // Handle S/R blocks
            handle_sr_blocks(&response, context_manager).await?;
            
            // Handle command blocks
            handle_command_blocks(&response, context_manager, secure_executor).await?;
        }
        Err(e) => {
            eprintln!("Error sending request to LLM: {}", e);
        }
    }
    
    Ok(())
}

async fn handle_sr_blocks(response: &str, context_manager: &ContextManager) -> Result<()> {
    let sr_blocks = sr_parser::parse_sr_blocks(response)?;
    if !sr_blocks.is_empty() {
        match editor::confirm_and_apply_blocks(sr_blocks, response, context_manager).await {
            Ok(()) => {
                // S/R blocks processed successfully, the editor handles notifications
            }
            Err(e) => eprintln!("Error applying edits: {}", e),
        }
    }
    Ok(())
}

fn is_autonomous_command(command: &str) -> bool {
    let cmd = command.trim();
    
    // Agent commands (autonomous)
    if cmd.starts_with("/agents") || 
       cmd.starts_with("/agent ") ||
       cmd.starts_with("/delegate ") ||
       cmd.starts_with("/ask_agent ") {
        return true;
    }
    
    // Security commands (autonomous)
    if cmd.starts_with("/security") ||
       cmd.starts_with("/sandbox ") ||
       cmd.starts_with("/approval ") {
        return true;
    }
    
    // Memory commands (autonomous) 
    if cmd.starts_with("/memory") ||
       cmd.starts_with("/search ") ||
       cmd.starts_with("/learn ") {
        return true;
    }
    
    // File management commands (autonomous)
    if cmd.starts_with("/add_file ") ||
       cmd.starts_with("/add_snippet ") ||
       cmd.starts_with("/show_context") ||
       cmd.starts_with("/clear_context") {
        return true;
    }
    
    // Restricted commands (require user approval)
    if cmd.starts_with("/config") ||
       cmd.starts_with("/provider ") ||
       cmd.starts_with("/model ") {
        return false;
    }
    
    // All other commands require approval by default
    false
}

async fn handle_command_blocks(response: &str, context_manager: &mut ContextManager, secure_executor: &SecureExecutor) -> Result<()> {
    let command_blocks = cmd_parser::parse_command_blocks(response)?;
    if command_blocks.is_empty() {
        return Ok(());
    }
    
    // Separate autonomous commands from those requiring approval
    let (autonomous_commands, approval_commands): (Vec<_>, Vec<_>) = 
        command_blocks.iter().partition(|cmd| is_autonomous_command(&cmd.command));
    
    // Execute autonomous commands immediately
    if !autonomous_commands.is_empty() {
        println!("\n{}", "Executing autonomous AI commands:".green().bold());
        for cmd_block in &autonomous_commands {
            println!("🤖 {}", cmd_block.command.bright_cyan());
            match secure_executor.execute_command_block(cmd_block).await {
                Ok(output) => {
                    if !output.trim().is_empty() {
                        println!("{}", output);
                    }
                    // Add command output to context for potential follow-up
                    context_manager.add_snippet(format!("Autonomous execution of '{}': \n{}", cmd_block.command, output));
                }
                Err(e) => {
                    eprintln!("Autonomous execution failed: {}", e);
                    // Add error to context as well
                    context_manager.add_snippet(format!("Autonomous execution error for '{}': {}", cmd_block.command, e));
                }
            }
        }
    }
    
    // Handle commands requiring approval
    if !approval_commands.is_empty() {
        println!("\n{}", "The AI suggested the following commands that require approval:".yellow().bold());
        for (i, cmd_block) in approval_commands.iter().enumerate() {
            println!("{}. {}", i + 1, cmd_block.command.bright_cyan());
        }
        
        println!("\n{}", "Do you want to execute these commands? [y/N/a(ll)/q(uit)]".yellow());
        
        let mut user_response = String::new();
        io::stdin().read_line(&mut user_response)?;
        let user_response = user_response.trim().to_lowercase();
        
        if user_response == "y" || user_response == "yes" || user_response == "a" || user_response == "all" {
            for cmd_block in &approval_commands {
                println!("\n{} {}", "Executing with approval:".green().bold(), cmd_block.command);
                match secure_executor.execute_command_block(cmd_block).await {
                    Ok(output) => {
                        if !output.trim().is_empty() {
                            println!("{}", output);
                        }
                        // Add command output to context for potential follow-up
                        context_manager.add_snippet(format!("Approved execution of '{}': \n{}", cmd_block.command, output));
                    }
                    Err(e) => {
                        eprintln!("Approved execution failed: {}", e);
                        // Add error to context as well
                        context_manager.add_snippet(format!("Approved execution error for '{}': {}", cmd_block.command, e));
                    }
                }
            }
        } else if user_response == "q" || user_response == "quit" {
            std::process::exit(0);
        }
    }
    
    Ok(())
}

// Secure command execution is now handled by SecureExecutor