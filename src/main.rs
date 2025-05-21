use std::io::{self, Write};
use std::process::Command;

mod llm;
mod context;

use context::ContextManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("KOTA - Type '/quit' to exit.");
    println!("Commands: /add_file <path>, /add_snippet <text>, /show_context, /clear_context, /run <command>, /run_add <command>");
    
    let mut context_manager = ContextManager::new();

    loop {
        println!("You: ");
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;

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
                    println!("exiting KOTA");
                    break;
                }
                "/add_file" => {
                    if arg.is_empty() {
                        println!("Usage: /add_file <path_to_file>");
                    } else {
                        if let Err(e) = context_manager.add_file(arg) {
                            eprintln!("Error adding file: {}", e);
                        }
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
                _ => {
                    println!("Unknown command: {}", command);
                }
            }
        } else {
            // Not a command, treat as a prompt for the LLM
            let current_context = context_manager.get_formatted_context();
            match llm::ask_model(trimmed_input, &current_context).await {
                Ok(response) => {
                    println!("KOTA: {}", response);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
    Ok(())
}


