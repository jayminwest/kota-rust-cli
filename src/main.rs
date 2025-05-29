use std::io::{self, Write};
use std::process::Command;
use std::env;
use colored::*;
use termimad::MadSkin;

mod llm;
mod context;
mod sr_parser;
mod editor;
mod cmd_parser;
mod input;
mod thinking;
mod prompts;
mod tui;
mod dynamic_prompts;
mod file_browser;

use context::ContextManager;
use llm::LlmProvider;

fn render_markdown(content: &str) -> anyhow::Result<()> {
    // Create a markdown renderer with customized skin
    let mut skin = MadSkin::default();
    
    // Set consistent spacing and wrapping
    skin.paragraph.align = termimad::Alignment::Left;
    
    // Import the correct Color type from crossterm
    use termimad::crossterm::style::Color;
    use termimad::crossterm::terminal;
    
    // Get terminal dimensions
    let (width, _height) = terminal::size().unwrap_or((80, 24));
    // Ensure minimum width for proper rendering and add padding
    let width = width.saturating_sub(4).max(40); // Subtract 4 for terminal padding
    
    // Customize colors to match the existing UI theme using termimad's color functions
    skin.bold.set_fg(Color::White);
    skin.italic.set_fg(Color::AnsiValue(248)); // Light gray
    skin.strikeout.set_fg(Color::AnsiValue(244)); // Dimmed gray
    
    // Style headers with bright blue colors
    skin.headers[0].set_fg(Color::Rgb{r: 100, g: 200, b: 255}); // Bright blue for h1
    skin.headers[1].set_fg(Color::Rgb{r: 120, g: 200, b: 255}); // Slightly dimmer blue for h2
    skin.headers[2].set_fg(Color::Rgb{r: 140, g: 200, b: 255}); // Even dimmer for h3
    
    // Style code blocks and inline code
    skin.code_block.set_bg(Color::AnsiValue(235)); // Dark gray background
    skin.code_block.set_fg(Color::AnsiValue(252)); // Light gray text
    skin.inline_code.set_bg(Color::AnsiValue(237)); // Slightly lighter dark gray
    skin.inline_code.set_fg(Color::AnsiValue(252)); // Light gray text
    
    // Style lists with better spacing
    skin.bullet.set_fg(Color::Cyan);
    skin.paragraph.align = termimad::Alignment::Left;
    
    
    // Style quotes
    skin.quote_mark.set_fg(Color::AnsiValue(244)); // Dimmed gray
    
    // Ensure consistent paragraph formatting with no extra margins
    skin.paragraph.left_margin = 0;
    skin.paragraph.right_margin = 0;
    
    // Print the markdown content with proper formatting using dynamic width
    // The text method properly handles width constraints
    let formatted = skin.text(content, Some(width as usize));
    print!("{}", formatted);
    
    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let use_tui = args.contains(&"--tui".to_string()) || args.contains(&"-t".to_string());
    
    // Show help if requested
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("KOTA - AI Coding Assistant");
        println!();
        println!("Usage: {} [OPTIONS]", args[0]);
        println!();
        println!("Options:");
        println!("  -t, --tui       Launch with modern TUI interface");
        println!("  -h, --help      Show this help message");
        println!("  -v, --version   Show version information");
        println!();
        println!("Default: Launch in classic CLI mode");
        return Ok(());
    }
    
    // Show version if requested
    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("KOTA version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    
    let context_manager = ContextManager::new();
    let current_provider = LlmProvider::default();
    
    // Launch appropriate interface
    if use_tui {
        // Launch modern TUI
        tui::run_tui(context_manager, current_provider).await
    } else {
        // Launch classic CLI
        run_classic_cli(context_manager, current_provider).await
    }
}

async fn run_classic_cli(_context_manager: ContextManager, _current_provider: LlmProvider) -> anyhow::Result<()> {
    let header_width = 60;
    println!("{}", "═".repeat(header_width).bright_blue());
    println!("{}", "KOTA - AI Coding Assistant".bright_white().bold());
    println!("{}", "═".repeat(header_width).bright_blue());
    
    let mut context_manager = ContextManager::new();
    let mut current_provider = LlmProvider::default();
    
    // Show provider status and check API key for Gemini
    match current_provider {
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
    }
    println!();
    println!("{}", "Type your request or use /help for commands".dimmed());
    println!();

    loop {
        let user_input = input::read_line_with_shortcuts()?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() {
            continue;
        }
        
        // Command Parsing
        if trimmed_input.starts_with('/') {
            let parts: Vec<&str> = trimmed_input.splitn(2, ' ').collect();
            let command = parts[0];
            let arg = if parts.len() > 1 { parts[1] } else { "" };

            match command {
                "/quit" => {
                    println!("{}", "─".repeat(60).dimmed());
                    println!("{}", "Goodbye!".bright_white());
                    break;
                }
                "/add_file" => {
                    if arg.is_empty() {
                        println!("{} /add_file <path_to_file>", "Usage:".yellow());
                    } else if let Err(e) = context_manager.add_file(arg) {
                        println!("{} {}", "Error:".red(), e);
                    }
                }
                "/add_snippet" => {
                    if arg.is_empty() {
                        println!("Usage: /add_snippet <text_snippet>");
                    } else {
                        context_manager.add_snippet(arg.to_string());
                    }
                }
                "/show_context" => {
                    context_manager.show_context();
                }
                "/clear_context" => {
                    context_manager.clear_context();
                }
                "/run" | "/run_add" => { 
                    if arg.is_empty() {
                        println!("Usage: {} <shell_command_here>", command);
                    } else {
                        println!("Executing: {}", arg);
                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(arg)
                            .output();
                        match output {
                            Ok(out) => {
                                // Always print stdout
                                let stdout_str = String::from_utf8_lossy(&out.stdout);
                                println!("--- stdout ---\n{}\n--- end stdout ---", stdout_str.trim());
                                
                                // Print stderr if not empty
                                let stderr_str = String::from_utf8_lossy(&out.stderr);
                                if !stderr_str.trim().is_empty() {
                                    eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr_str.trim());
                                }
                                
                                // Add command output to context if /run_add was used
                                if command == "/run_add" {
                                    if !stdout_str.trim().is_empty() {
                                        context_manager.add_snippet(format!("Output of command '{}': \n{}", arg, stdout_str));
                                    } else if !stderr_str.trim().is_empty() {
                                        context_manager.add_snippet(format!("Error output of command '{}': \n{}", arg, stderr_str));
                                    }
                                }
                                
                                // Show exit status if not successful
                                if !out.status.success() {
                                    eprintln!("Command '{}' exited with status: {}", arg, out.status);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to execute command '{}': {}", arg, e);
                            }
                        }
                    }
                }
                "/git_add" => {
                    if arg.is_empty() {
                        println!("Usage: /git_add <file_path>");
                    } else {
                        println!("Executing: git add {}", arg);
                        let output = Command::new("git")
                            .arg("add")
                            .arg(arg)
                            .output();
                        
                        match output {
                            Ok(out) => {
                                // Always print stdout
                                let stdout_str = String::from_utf8_lossy(&out.stdout);
                                if !stdout_str.trim().is_empty() {
                                    println!("--- stdout ---\n{}\n--- end stdout ---", stdout_str.trim());
                                }
                                
                                // Print stderr if not empty
                                let stderr_str = String::from_utf8_lossy(&out.stderr);
                                if !stderr_str.trim().is_empty() {
                                    eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr_str.trim());
                                }
                                
                                // Show exit status if not successful
                                if out.status.success() {
                                    println!("Successfully added {}", arg);
                                } else {
                                    eprintln!("Git add failed with status: {}", out.status);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to execute git add: {}", e);
                            }
                        }
                    }
                }
                "/git_commit" => {
                    if arg.is_empty() {
                        println!("Usage: /git_commit \"<commit_message>\"");
                    } else {
                        println!("Executing: git commit -m \"{}\"", arg);
                        let output = Command::new("git")
                            .arg("commit")
                            .arg("-m")
                            .arg(arg)
                            .output();
                        
                        match output {
                            Ok(out) => {
                                // Always print stdout
                                let stdout_str = String::from_utf8_lossy(&out.stdout);
                                if !stdout_str.trim().is_empty() {
                                    println!("--- stdout ---\n{}\n--- end stdout ---", stdout_str.trim());
                                }
                                
                                // Print stderr if not empty
                                let stderr_str = String::from_utf8_lossy(&out.stderr);
                                if !stderr_str.trim().is_empty() {
                                    eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr_str.trim());
                                }
                                
                                // Show exit status if not successful
                                if out.status.success() {
                                    println!("Successfully committed changes with message: \"{}\"", arg);
                                } else {
                                    eprintln!("Git commit failed with status: {}", out.status);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to execute git commit: {}", e);
                            }
                        }
                    }
                }
                "/git_status" => {
                    println!("Executing: git status");
                    // Run basic git status
                    let output = Command::new("git")
                        .arg("status")
                        .output();
                    
                    match output {
                        Ok(out) => {
                            // Always print stdout
                            let stdout_str = String::from_utf8_lossy(&out.stdout);
                            if !stdout_str.trim().is_empty() {
                                println!("--- git status ---\n{}\n--- end git status ---", stdout_str.trim());
                            } else {
                                println!("No status information available");
                            }
                            
                            // Print stderr if not empty
                            let stderr_str = String::from_utf8_lossy(&out.stderr);
                            if !stderr_str.trim().is_empty() {
                                eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr_str.trim());
                            }
                            
                            // Show exit status if not successful
                            if !out.status.success() {
                                eprintln!("Git status failed with status: {}", out.status);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to execute git status: {}", e);
                        }
                    }
                }
                "/git_diff" => {
                    // Handle optional path argument
                    let cmd_str = if arg.is_empty() {
                        "git diff".to_string()
                    } else {
                        format!("git diff {}", arg)
                    };
                    
                    println!("Executing: {}", cmd_str);
                    
                    // Using Command with args to avoid shell escaping issues
                    let mut command = Command::new("git");
                    command.arg("diff");
                    
                    if !arg.is_empty() {
                        command.arg(arg);
                    }
                    
                    let output = command.output();
                    
                    match output {
                        Ok(out) => {
                            // Always print stdout
                            let stdout_str = String::from_utf8_lossy(&out.stdout);
                            if !stdout_str.trim().is_empty() {
                                println!("--- git diff ---\n{}\n--- end git diff ---", stdout_str.trim());
                            } else {
                                println!("No differences found");
                            }
                            
                            // Print stderr if not empty
                            let stderr_str = String::from_utf8_lossy(&out.stderr);
                            if !stderr_str.trim().is_empty() {
                                eprintln!("--- stderr ---\n{}\n--- end stderr ---", stderr_str.trim());
                            }
                            
                            // Show exit status if not successful
                            if !out.status.success() {
                                eprintln!("Git diff failed with status: {}", out.status);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to execute git diff: {}", e);
                        }
                    }
                }
                "/help" => {
                    println!();
                    println!("{}", "─".repeat(60).bright_blue());
                    println!("{}", "KOTA Commands".bright_white().bold());
                    println!("{}", "─".repeat(60).bright_blue());
                    println!();
                    
                    println!("{}", "Context Management:".bright_yellow().bold());
                    println!("  {} - Add file contents to context", "/add_file <path>".cyan());
                    println!("  {} - Add text snippet to context", "/add_snippet <text>".cyan());
                    println!("  {} - Display current context", "/show_context".cyan());
                    println!("  {} - Clear all context", "/clear_context".cyan());
                    println!();
                    
                    println!("{}", "Command Execution:".bright_yellow().bold());
                    println!("  {} - Execute shell command", "/run <command>".cyan());
                    println!("  {} - Execute command and add output to context", "/run_add <command>".cyan());
                    println!();
                    
                    println!("{}", "Git Operations:".bright_yellow().bold());
                    println!("  {} - Stage file for commit", "/git_add <file>".cyan());
                    println!("  {} - Create git commit", "/git_commit \"<message>\"".cyan());
                    println!("  {} - Show git status", "/git_status".cyan());
                    println!("  {} - Show git diff", "/git_diff [<path>]".cyan());
                    println!();
                    
                    println!("{}", "Configuration:".bright_yellow().bold());
                    println!("  {} - Switch LLM provider", "/provider <ollama|gemini>".cyan());
                    println!("  {} - Show current provider", "/provider".cyan());
                    println!();
                    
                    println!("{}", "General:".bright_yellow().bold());
                    println!("  {} - Show this help message", "/help".cyan());
                    println!("  {} - Show KOTA version", "/version".cyan());
                    println!("  {} - Exit KOTA", "/quit".cyan());
                    println!();
                    
                    println!("{}", "AI Interactions:".bright_yellow().bold());
                    println!("  {} - Ask AI to edit files or execute commands", "Type any message".cyan());
                    println!("  {} - AI can suggest file edits and shell commands", "".dimmed());
                    println!();
                    
                    println!("{}", "Important: File Access Control".bright_red().bold());
                    println!("  {} - Files must be added to context before editing", "⚠️ ".yellow());
                    println!("  {} - Use /add_file before asking AI to edit", "".dimmed());
                    println!("  {} - Edits to files not in context will be blocked", "".dimmed());
                    println!();
                }
                "/tui" => {
                    // Switch to TUI mode
                    println!("Switching to TUI mode...");
                    return tui::run_tui(context_manager, current_provider).await;
                }
                "/provider" => {
                    if arg.is_empty() {
                        match current_provider {
                            LlmProvider::Ollama => println!("Current provider: Ollama"),
                            LlmProvider::Gemini => println!("Current provider: Google Gemini"),
                        }
                        println!("Usage: /provider <ollama|gemini>");
                    } else {
                        match arg.to_lowercase().as_str() {
                            "ollama" => {
                                current_provider = LlmProvider::Ollama;
                                println!("{} {}", "Provider:".green(), "Ollama (local)".cyan());
                            }
                            "gemini" => {
                                // Check if GEMINI_API_KEY is set
                                if std::env::var("GEMINI_API_KEY").is_ok() {
                                    current_provider = LlmProvider::Gemini;
                                    println!("{} {}", "Provider:".green(), "Google Gemini (cloud)".cyan());
                                } else {
                                    println!("{} GEMINI_API_KEY environment variable", "Missing:".red());
                                    println!("{} export GEMINI_API_KEY=your_api_key", "Set with:".dimmed());
                                }
                            }
                            _ => {
                                println!("Unknown provider: {}. Use 'ollama' or 'gemini'", arg);
                            }
                        }
                    }
                }
                "/version" => {
                    // Retrieve version from Cargo.toml at compile time
                    println!("KOTA version: {}", env!("CARGO_PKG_VERSION"));
                }
                _ => {
                    println!("Unknown command: {}", command);
                }
            }
        } else {
            // Not a command, treat as a prompt for the LLM
            let current_context = context_manager.get_formatted_context();
            
            // Show thinking indicator while waiting for LLM response
            let thinking = thinking::show_llm_thinking();
            
            match llm::ask_model_with_provider(trimmed_input, &current_context, current_provider.clone()).await {
                Ok(response) => {
                    // Clear the thinking indicator
                    thinking.finish();
                    
                    // Always display the response first with markdown rendering
                    println!();
                    println!("{}", "─".repeat(60).dimmed());
                    
                    // Try to render as markdown, fall back to plain text if it fails
                    if render_markdown(&response).is_err() {
                        println!("{}", response);
                    }
                    
                    println!();
                    
                    // Check if the response contains S/R blocks
                    if sr_parser::contains_sr_blocks(&response) {
                        // Parse and handle S/R blocks
                        match sr_parser::parse_sr_blocks(&response) {
                            Ok(blocks) => {
                                if !blocks.is_empty() {
                                    if let Err(e) = editor::confirm_and_apply_blocks(blocks, trimmed_input, &context_manager).await {
                                        eprintln!("Error applying S/R blocks: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error parsing S/R blocks: {}", e);
                            }
                        }
                    }
                    
                    // Check if the response contains command blocks
                    if cmd_parser::contains_command_blocks(&response) {
                        match cmd_parser::parse_command_blocks(&response) {
                            Ok(cmd_blocks) => {
                                if !cmd_blocks.is_empty() {
                                    println!("{}", "─".repeat(60).dimmed());
                                    println!("{} {}", "Commands to execute:".bright_yellow().bold(), cmd_blocks.len());
                                    
                                    for (i, cmd_block) in cmd_blocks.iter().enumerate() {
                                        println!();
                                        println!("{} {}", format!("Command {}:", i + 1).bright_white().bold(), cmd_block.command.bright_cyan());
                                        
                                        // Ask for confirmation
                                        loop {
                                            print!("{} ", "Execute? (y/n/q):".bright_white());
                                            io::stdout().flush()?;
                                            
                                            let choice = match input::read_single_char() {
                                                Ok(c) => c.to_lowercase().to_string(),
                                                Err(_) => continue,
                                            };
                                            
                                            match choice.as_str() {
                                                "y" | "yes" => {
                                                    println!();
                                                    println!("{}", "─".repeat(30).dimmed());
                                                    match cmd_parser::execute_command(&cmd_block.command).await {
                                                        Ok((stdout, stderr, success)) => {
                                                            if !stdout.trim().is_empty() {
                                                                println!("{}", stdout.trim());
                                                                // Add stdout to context for the model to see
                                                                context_manager.add_snippet(format!("Command: {}\nOutput:\n{}", cmd_block.command, stdout.trim()));
                                                            }
                                                            if !stderr.trim().is_empty() {
                                                                println!("{}", stderr.trim().red());
                                                                // Add stderr to context if it's significant (not just warnings)
                                                                if !success || stderr.len() > 100 {
                                                                    context_manager.add_snippet(format!("Command: {}\nError output:\n{}", cmd_block.command, stderr.trim()));
                                                                }
                                                            }
                                                            println!("{}", "─".repeat(30).dimmed());
                                                            if success {
                                                                println!("{}", "Success".green());
                                                            } else {
                                                                println!("{}", "Failed".red());
                                                                // Always add failed commands to context
                                                                if stdout.trim().is_empty() && stderr.trim().is_empty() {
                                                                    context_manager.add_snippet(format!("Command failed: {}", cmd_block.command));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            println!("{}", "─".repeat(30).dimmed());
                                                            println!("{} {}", "Execution failed:".red(), e);
                                                            // Add execution error to context
                                                            context_manager.add_snippet(format!("Command execution failed: {} - Error: {}", cmd_block.command, e));
                                                        }
                                                    }
                                                    break;
                                                }
                                                "n" | "no" => {
                                                    println!("{}", "Skipped".dimmed());
                                                    break;
                                                }
                                                "q" | "quit" => {
                                                    println!("{}", "Stopped executing commands".dimmed());
                                                    return Ok(());
                                                }
                                                _ => {
                                                    println!("Please enter 'y' (yes), 'n' (no), or 'q' (quit)");
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error parsing command blocks: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    // Clear the thinking indicator
                    thinking.finish();
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
    Ok(())
}


