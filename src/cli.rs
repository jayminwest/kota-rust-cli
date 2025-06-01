use std::io;
use anyhow::Result;
use colored::*;

use crate::context::ContextManager;
use crate::llm::{LlmProvider, ModelConfig};
use crate::commands::{CommandRegistry, CommandResult};
use crate::{input, thinking, sr_parser, editor, cmd_parser, tui, render_markdown};

/// Runs the classic CLI interface
pub async fn run_classic_cli(_context_manager: ContextManager, _model_config: ModelConfig) -> Result<()> {
    let header_width = 60;
    println!("{}", "═".repeat(header_width).bright_blue());
    println!("{}", "KOTA - AI Coding Assistant".bright_white().bold());
    println!("{}", "═".repeat(header_width).bright_blue());
    
    let mut context_manager = ContextManager::new();
    let mut model_config = ModelConfig::default();
    let command_registry = CommandRegistry::new();
    
    // Show provider status and check API key
    show_provider_status(&model_config);
    
    println!("{}", "─".repeat(header_width).dimmed());
    println!("{} Type '/help' for available commands", "💡".yellow());
    println!("{} Type anything else to chat with AI", "💬".bright_blue());
    println!();

    loop {
        let user_input = input::read_line_with_shortcuts()?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() {
            continue;
        }
        
        if trimmed_input.starts_with('/') {
            if let Err(e) = handle_command(trimmed_input, &mut context_manager, &mut model_config, &command_registry).await {
                eprintln!("Command error: {}", e);
            }
        } else if let Err(e) = handle_ai_interaction(trimmed_input, &mut context_manager, &model_config).await {
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
            match command_registry.execute(command, arg, context_manager, model_config)? {
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
            handle_command_blocks(&response, context_manager).await?;
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

async fn handle_command_blocks(response: &str, context_manager: &mut ContextManager) -> Result<()> {
    let command_blocks = cmd_parser::parse_command_blocks(response)?;
    if !command_blocks.is_empty() {
        println!("\n{}", "The AI suggested the following commands:".yellow().bold());
        for (i, cmd_block) in command_blocks.iter().enumerate() {
            println!("{}. {}", i + 1, cmd_block.command.bright_cyan());
        }
        
        println!("\n{}", "Do you want to execute these commands? [y/N/a(ll)/q(uit)]".yellow());
        
        let mut user_response = String::new();
        io::stdin().read_line(&mut user_response)?;
        let user_response = user_response.trim().to_lowercase();
        
        if user_response == "y" || user_response == "yes" || user_response == "a" || user_response == "all" {
            for cmd_block in &command_blocks {
                println!("\n{} {}", "Executing:".green().bold(), cmd_block.command);
                let output = execute_shell_command(&cmd_block.command).await;
                match output {
                    Ok((stdout, stderr, success)) => {
                        if !stdout.trim().is_empty() {
                            println!("--- stdout ---\n{}\n--- end stdout ---", stdout);
                        }
                        if !stderr.trim().is_empty() {
                            eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr);
                        }
                        // Add command output to context for potential follow-up
                        if !stdout.trim().is_empty() {
                            context_manager.add_snippet(format!("Output of command '{}': \n{}", cmd_block.command, stdout));
                        }
                        if !stderr.trim().is_empty() {
                            context_manager.add_snippet(format!("Error output of command '{}': \n{}", cmd_block.command, stderr));
                        }
                        if !success {
                            eprintln!("Command '{}' failed", cmd_block.command);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error executing command: {}", e);
                        // Add error to context as well
                        context_manager.add_snippet(format!("Error executing command '{}': {}", cmd_block.command, e));
                    }
                }
            }
        } else if user_response == "q" || user_response == "quit" {
            std::process::exit(0);
        }
    }
    Ok(())
}

async fn execute_shell_command(command: &str) -> Result<(String, String, bool)> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();
    
    Ok((stdout, stderr, success))
}