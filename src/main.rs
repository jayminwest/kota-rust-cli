use std::io::{self, Write};
use std::process::Command;

mod llm;
mod context;

use context::ContextManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("KOTA - Type '/quit' to exit.");
    println!("Commands: /add_file <path>, /add_snippet <text>, /show_context, /clear_context, /run <command>, /run_add <command>, /git_add <file_path>, /git_commit \"<message>\", /git_status, /git_diff [<path>]");
    
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


