use std::sync::{Arc, Mutex};
use anyhow::Result;
use chrono::Local;
use tokio::sync::mpsc;
use tokio::task;

use crate::context::ContextManager;
use crate::llm::{self, LlmProvider, ModelConfig};
use crate::file_browser::FileBrowser;
use crate::dynamic_prompts::DynamicPromptData;
use crate::memory::MemoryManager;
use crate::sr_parser;
use crate::editor;
use crate::cmd_parser;

use super::types::{InputMode, FocusedPane, AppMessage, MessageContent, CommandStatus, CommandSuggestion};

// Threshold for collapsing pasted content
const PASTE_COLLAPSE_THRESHOLD: usize = 10;

pub struct App {
    // UI state
    pub input: String,
    pub input_lines: Vec<String>, // For multi-line input
    pub current_line: usize,      // Current line cursor position
    pub input_mode: InputMode,
    pub messages: Vec<(String, MessageContent)>, // (role, content)
    pub context_view: String,
    pub status_message: String,
    pub current_time: String,
    pub scroll_offset: u16,
    pub auto_scroll_enabled: bool,
    pub focused_pane: FocusedPane,
    
    // Core components
    pub context_manager: Arc<Mutex<ContextManager>>,
    pub model_config: ModelConfig,
    pub memory_manager: MemoryManager,
    
    // Terminal output buffer
    pub terminal_output: Vec<String>,
    pub terminal_scroll: u16,
    pub suggested_commands: Vec<CommandSuggestion>,
    pub selected_command_index: usize,
    
    // File browser
    pub file_browser: FileBrowser,
    pub show_file_browser: bool,
    
    // Live data
    pub live_data: DynamicPromptData,
    
    // Message channel
    pub tx: mpsc::UnboundedSender<AppMessage>,
    pub rx: Option<mpsc::UnboundedReceiver<AppMessage>>,
    
    // Processing state
    pub is_processing: bool,
    
    // Context scroll
    pub context_scroll: u16,
    
    // Application state
    pub should_quit: bool,
}

impl App {
    pub fn new(context_manager: ContextManager, model_config: ModelConfig) -> Result<Self> {
        let live_data = DynamicPromptData::new(&context_manager);
        let file_browser = FileBrowser::new()?;
        let memory_manager = MemoryManager::new()?;
        let (tx, rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            input: String::new(),
            input_lines: vec![String::new()],
            current_line: 0,
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            context_view: String::new(),
            status_message: "Ready - Press '?' for help".to_string(),
            current_time: Local::now().format("%H:%M:%S").to_string(),
            scroll_offset: 0,
            auto_scroll_enabled: true,
            focused_pane: FocusedPane::Chat,
            context_manager: Arc::new(Mutex::new(context_manager)),
            model_config,
            memory_manager,
            terminal_output: Vec::new(),
            terminal_scroll: 0,
            suggested_commands: Vec::new(),
            selected_command_index: 0,
            file_browser,
            show_file_browser: true,
            live_data,
            tx,
            rx: Some(rx),
            is_processing: false,
            context_scroll: 0,
            should_quit: false,
        })
    }
    
    pub fn update_time(&mut self) {
        self.current_time = Local::now().format("%H:%M:%S").to_string();
    }
    
    pub fn update_context_view(&mut self) {
        if let Ok(cm) = self.context_manager.lock() {
            self.context_view = cm.get_formatted_context();
            // Update live data
            self.live_data = DynamicPromptData::new(&cm);
        }
    }
    
    pub fn add_terminal_output(&mut self, output: String) {
        self.terminal_output.push(output);
        // Keep only last 1000 lines
        if self.terminal_output.len() > 1000 {
            self.terminal_output.remove(0);
        }
    }
    
    pub fn auto_scroll_to_bottom(&mut self) {
        if self.auto_scroll_enabled {
            // For now, just ensure we can see the content by resetting scroll to 0
            // This will show messages from the beginning
            // TODO: Implement proper bottom-scrolling when we have more messages than fit on screen
            self.scroll_offset = 0;
        }
    }
    
    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll_enabled = !self.auto_scroll_enabled;
        self.status_message = format!("Auto-scroll: {}", 
            if self.auto_scroll_enabled { "ON" } else { "OFF" });
    }
    
    pub fn get_full_input(&self) -> String {
        if self.input_lines.len() == 1 {
            self.input.clone()
        } else {
            let mut lines = self.input_lines.clone();
            // Update the current line with any ongoing input
            if !self.input.is_empty() && self.current_line < lines.len() {
                lines[self.current_line] = self.input.clone();
            }
            lines.join("\n")
        }
    }
    
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.input_lines = vec![String::new()];
        self.current_line = 0;
    }
    
    pub fn is_multi_line_input(&self) -> bool {
        self.input_lines.len() > 1 || self.input.contains('\n')
    }
    
    pub fn add_new_line(&mut self) {
        // Convert single line to multi-line if needed
        if self.input_lines.len() == 1 && !self.input.is_empty() {
            self.input_lines[0] = self.input.clone();
        } else if !self.input.is_empty() {
            // Add current input to the current line
            if self.current_line < self.input_lines.len() {
                self.input_lines[self.current_line] = self.input.clone();
            }
        }
        
        self.input_lines.push(String::new());
        self.current_line = self.input_lines.len() - 1;
        self.input.clear(); // Clear the working input
    }
    
    pub fn should_auto_continue(&self) -> bool {
        let empty_string = String::new();
        let content = if self.input_lines.len() == 1 {
            &self.input
        } else {
            self.input_lines.last().unwrap_or(&empty_string)
        };
        
        // Check for multi-line triggers (similar to input.rs logic)
        content.ends_with('\\') ||           // Line continuation
        content.ends_with(':') ||            // Python-style blocks
        content.ends_with('{') ||            // Open brace
        content.starts_with("```") ||        // Code blocks
        self.has_unmatched_delimiters(content)
    }
    
    pub fn has_unmatched_delimiters(&self, content: &str) -> bool {
        let mut parens = 0;
        let mut brackets = 0;
        let mut braces = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for ch in content.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' => escape_next = true,
                '"' | '\'' if !in_string => in_string = true,
                '"' | '\'' if in_string => in_string = false,
                '(' if !in_string => parens += 1,
                ')' if !in_string => parens -= 1,
                '[' if !in_string => brackets += 1,
                ']' if !in_string => brackets -= 1,
                '{' if !in_string => braces += 1,
                '}' if !in_string => braces -= 1,
                _ => {}
            }
        }
        
        parens > 0 || brackets > 0 || braces > 0 || in_string
    }
    
    pub fn add_suggested_command(&mut self, command: String) {
        let suggestion = CommandSuggestion {
            command: command.clone(),
            description: Some(format!("Execute: {}", command)),
            status: CommandStatus::Pending,
            output: None,
        };
        self.suggested_commands.push(suggestion);
        self.add_terminal_output(format!("[SUGGESTED] {}", command));
    }
    
    pub fn navigate_commands(&mut self, direction: i32) {
        if self.suggested_commands.is_empty() {
            return;
        }
        
        let len = self.suggested_commands.len();
        match direction.cmp(&0) {
            std::cmp::Ordering::Greater => {
                self.selected_command_index = (self.selected_command_index + 1) % len;
            }
            std::cmp::Ordering::Less => {
                self.selected_command_index = if self.selected_command_index == 0 {
                    len - 1
                } else {
                    self.selected_command_index - 1
                };
            }
            std::cmp::Ordering::Equal => {
                // No change for zero direction
            }
        }
    }
    
    pub fn execute_selected_command(&mut self) -> Option<String> {
        if self.selected_command_index < self.suggested_commands.len() {
            let command = self.suggested_commands[self.selected_command_index].command.clone();
            self.suggested_commands[self.selected_command_index].status = CommandStatus::Running;
            Some(command)
        } else {
            None
        }
    }
    
    pub async fn execute_selected_command_async(&mut self) {
        if let Some(command) = self.execute_selected_command() {
            self.add_terminal_output(format!("[EXEC] {}", command));
            
            // Execute the command using tokio process
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    if output.status.success() {
                        if !stdout.trim().is_empty() {
                            for line in stdout.lines() {
                                self.add_terminal_output(format!("  {}", line));
                            }
                        }
                        self.add_terminal_output("[SUCCESS] Command completed".to_string());
                        
                        // Update command status
                        if self.selected_command_index < self.suggested_commands.len() {
                            self.suggested_commands[self.selected_command_index].status = CommandStatus::Success;
                            self.suggested_commands[self.selected_command_index].output = Some(stdout.to_string());
                        }
                    } else {
                        self.add_terminal_output(format!("[ERROR] Command failed with code: {}", 
                            output.status.code().unwrap_or(-1)));
                        if !stderr.trim().is_empty() {
                            for line in stderr.lines() {
                                self.add_terminal_output(format!("  {}", line));
                            }
                        }
                        
                        // Update command status and show error details
                        if self.selected_command_index < self.suggested_commands.len() {
                            let error_msg = stderr.to_string();
                            self.suggested_commands[self.selected_command_index].status = CommandStatus::Failed(error_msg.clone());
                            // Log the error for debugging
                            self.add_terminal_output(format!("[DEBUG] Error details: {}", error_msg));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Execution error: {}", e);
                    self.add_terminal_output(format!("[ERROR] Failed to execute: {}", e));
                    if self.selected_command_index < self.suggested_commands.len() {
                        self.suggested_commands[self.selected_command_index].status = CommandStatus::Failed(error_msg);
                    }
                }
            }
        } else {
            self.add_terminal_output("No command selected".to_string());
        }
    }
    
    pub fn add_file_to_context(&mut self, path: &str) -> Result<()> {
        if let Ok(mut cm) = self.context_manager.lock() {
            cm.add_file(path)?;
        }
        self.update_context_view();
        self.status_message = format!("Added {} to context", path);
        Ok(())
    }
    
    pub async fn process_user_input(&mut self, input: String) {
        // Use the full input (could be multi-line)
        let full_input = if input.is_empty() {
            self.get_full_input()
        } else {
            input
        };
        
        // Check if this is a command (starts with / or :)
        let trimmed = full_input.trim();
        if trimmed.starts_with('/') || trimmed.starts_with(':') {
            // Remove the prefix and process as command
            let cmd = if trimmed.starts_with('/') {
                trimmed.strip_prefix('/').unwrap_or(trimmed)
            } else {
                trimmed.strip_prefix(':').unwrap_or(trimmed)
            };
            self.process_command(cmd.to_string()).await;
            return;
        }
        
        // Check if this is a large paste
        let line_count = full_input.lines().count();
        let message_content = if line_count > PASTE_COLLAPSE_THRESHOLD {
            MessageContent::CollapsedPaste {
                summary: format!("[Pasted {} lines]", line_count),
                full_content: full_input.clone(),
            }
        } else {
            MessageContent::Text(full_input.clone())
        };
        
        self.messages.push(("User".to_string(), message_content.clone()));
        
        // Auto-scroll to bottom when new message is added
        self.auto_scroll_to_bottom();
        
        // Display in terminal
        match &message_content {
            MessageContent::Text(text) => {
                self.add_terminal_output(format!(">>> {}", text));
            }
            MessageContent::CollapsedPaste { summary, .. } => {
                self.add_terminal_output(format!(">>> {}", summary));
            }
        }
        
        self.is_processing = true;
        self.status_message = "Processing LLM request... (UI remains interactive)".to_string();
        
        // Get current context
        let context = if let Ok(cm) = self.context_manager.lock() {
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        // Extract the actual content for LLM
        let actual_content = match &message_content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::CollapsedPaste { full_content, .. } => full_content.clone(),
        };
        
        // Store conversation in memory
        if let Err(e) = self.memory_manager.store_conversation_summary(&format!("User: {}", full_input)) {
            eprintln!("Warning: Failed to store user message in memory: {}", e);
        }
        
        // Clear the input after processing
        self.clear_input();
        
        // Spawn async LLM task
        let tx = self.tx.clone();
        let model_config = self.model_config.clone();
        let prompt = actual_content;
        
        task::spawn(async move {
            match llm::ask_model_with_config(&prompt, &context, &model_config).await {
                Ok(response) => {
                    let _ = tx.send(AppMessage::LlmResponse(prompt, response));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::TerminalOutput(format!("Error: {}", e)));
                }
            }
            let _ = tx.send(AppMessage::ProcessingComplete);
        });
    }
    
    pub async fn process_command(&mut self, cmd: String) {
        self.status_message = format!("Executing command: {}", cmd);
        
        // Handle vim-style commands first
        match cmd.trim() {
            "q" | "quit" => {
                // Signal to exit the application
                self.should_quit = true;
                return;
            }
            "w" | "write" => {
                // Save current context to a file
                if let Ok(cm) = self.context_manager.lock() {
                    let context = cm.get_formatted_context();
                    match std::fs::write("kota_context.txt", context) {
                        Ok(_) => self.status_message = "Context saved to kota_context.txt".to_string(),
                        Err(e) => self.status_message = format!("Error saving context: {}", e),
                    }
                } else {
                    self.status_message = "Error accessing context".to_string();
                }
                return;
            }
            "wq" => {
                // Save and quit
                if let Ok(cm) = self.context_manager.lock() {
                    let context = cm.get_formatted_context();
                    let _ = std::fs::write("kota_context.txt", context);
                }
                self.should_quit = true;
                return;
            }
            "h" | "help" => {
                self.add_terminal_output("Vim Commands:".to_string());
                self.add_terminal_output("  :q, :quit         - Exit KOTA".to_string());
                self.add_terminal_output("  :w, :write        - Save context to file".to_string());
                self.add_terminal_output("  :wq               - Save and quit".to_string());
                self.add_terminal_output("  :e <file>         - Edit/add file to context".to_string());
                self.add_terminal_output("  :h, :help         - Show this help".to_string());
                self.add_terminal_output("".to_string());
                self.add_terminal_output("Navigation:".to_string());
                self.add_terminal_output("  Normal mode: hjkl, Tab, i, f, :, ?".to_string());
                self.add_terminal_output("  Insert mode: Esc to return to Normal".to_string());
                self.add_terminal_output("".to_string());
                self.add_terminal_output("File Commands:".to_string());
                self.add_terminal_output("  :e <file>         - Edit/add file to context".to_string());
                self.add_terminal_output("  :add <file>       - Add file to context (alias for :e)".to_string());
                self.add_terminal_output("  :context          - Display current context".to_string());
                self.add_terminal_output("  :clear            - Clear all context".to_string());
                self.add_terminal_output("  :provider <name>  - Switch LLM provider".to_string());
                self.add_terminal_output("  :model <name>     - Set model".to_string());
                self.add_terminal_output("".to_string());
                self.add_terminal_output("Memory Commands:".to_string());
                self.add_terminal_output("  :memory           - Show recent memories".to_string());
                self.add_terminal_output("  :search <query>   - Search knowledge base".to_string());
                self.add_terminal_output("  :learn <topic>: <content> - Store learning".to_string());
                return;
            }
            _ => {} // Continue to handle other commands
        }
        
        // Handle vim-style edit command
        if cmd.starts_with("e ") {
            let path = cmd.strip_prefix("e ").unwrap_or("");
            if let Err(e) = self.add_file_to_context(path) {
                self.status_message = format!("Error: {}", e);
            }
            return;
        }
        
        // Handle file commands
        if cmd.starts_with("add ") {
            let path = cmd.strip_prefix("add ").unwrap_or("");
            if let Err(e) = self.add_file_to_context(path) {
                self.status_message = format!("Error: {}", e);
            }
        } else if cmd.starts_with("add_file ") {
            // Legacy support for old command format
            let path = cmd.strip_prefix("add_file ").unwrap_or("");
            if let Err(e) = self.add_file_to_context(path) {
                self.status_message = format!("Error: {}", e);
            }
        } else if cmd == "context" || cmd == "show_context" {
            let context = if let Ok(cm) = self.context_manager.lock() {
                cm.get_formatted_context()
            } else {
                "Error accessing context".to_string()
            };
            self.add_terminal_output(format!("Context:\n{}", context));
        } else if cmd == "clear" || cmd == "clear_context" {
            if let Ok(mut cm) = self.context_manager.lock() {
                cm.clear_context();
            }
            self.update_context_view();
            self.status_message = "Context cleared".to_string();
        } else if cmd.starts_with("provider ") {
            let provider = cmd.strip_prefix("provider ").unwrap_or("");
            match provider {
                "ollama" => {
                    self.model_config.provider = LlmProvider::Ollama;
                    self.status_message = "Switched to Ollama".to_string();
                }
                "gemini" => {
                    self.model_config.provider = LlmProvider::Gemini;
                    self.status_message = "Switched to Gemini".to_string();
                }
                "anthropic" => {
                    self.model_config.provider = LlmProvider::Anthropic;
                    self.status_message = "Switched to Anthropic Claude".to_string();
                }
                _ => {
                    self.status_message = "Unknown provider. Use 'ollama', 'gemini', or 'anthropic'".to_string();
                }
            }
        } else if cmd.starts_with("model ") {
            let model = cmd.strip_prefix("model ").unwrap_or("");
            if model.is_empty() {
                self.status_message = format!("Current model: {}", self.model_config.display_name());
            } else {
                self.model_config.model_name = Some(model.to_string());
                self.status_message = format!("Model set to: {}", self.model_config.display_name());
            }
        } else if cmd == "memory" || cmd == "memories" {
            match self.memory_manager.get_recent_memories(5) {
                Ok(memories) => {
                    self.add_terminal_output("=== Recent Memories ===".to_string());
                    let is_empty = memories.is_empty();
                    for memory in memories {
                        self.add_terminal_output(memory);
                    }
                    if is_empty {
                        self.add_terminal_output("No memories found".to_string());
                    }
                }
                Err(e) => {
                    self.status_message = format!("Error accessing memories: {}", e);
                }
            }
        } else if cmd.starts_with("search ") {
            let query = cmd.strip_prefix("search ").unwrap_or("");
            if !query.is_empty() {
                match self.memory_manager.search_knowledge(query) {
                    Ok(results) => {
                        self.add_terminal_output(format!("=== Search Results for '{}' ===", query));
                        let is_empty = results.is_empty();
                        for result in results {
                            self.add_terminal_output(result);
                        }
                        if is_empty {
                            self.add_terminal_output("No results found".to_string());
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Error searching: {}", e);
                    }
                }
            } else {
                self.status_message = "Usage: search <query>".to_string();
            }
        } else if cmd.starts_with("learn ") {
            let content = cmd.strip_prefix("learn ").unwrap_or("");
            if !content.is_empty() {
                // Extract topic and content (simple parsing)
                let parts: Vec<&str> = content.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let topic = parts[0].trim();
                    let learning_content = parts[1].trim();
                    match self.memory_manager.store_learning(topic, learning_content) {
                        Ok(_) => {
                            self.status_message = format!("Stored learning about: {}", topic);
                        }
                        Err(e) => {
                            self.status_message = format!("Error storing learning: {}", e);
                        }
                    }
                } else {
                    self.status_message = "Usage: learn <topic>: <content>".to_string();
                }
            } else {
                self.status_message = "Usage: learn <topic>: <content>".to_string();
            }
        } else {
            self.status_message = format!("Unknown command: {}", cmd);
        }
    }
    
    #[allow(clippy::await_holding_lock)]
    pub async fn handle_llm_response(&mut self, original_prompt: String, response: String) {
        // Always show KOTA responses in full - don't collapse them
        let message_content = MessageContent::Text(response.clone());
        
        self.messages.push(("KOTA".to_string(), message_content));
        
        // Store KOTA response in memory
        if let Err(e) = self.memory_manager.store_conversation_summary(&format!("KOTA: {}", &response[..500.min(response.len())])) {
            eprintln!("Warning: Failed to store KOTA response in memory: {}", e);
        }
        
        // Auto-scroll to bottom when KOTA responds
        self.auto_scroll_to_bottom();
        
        self.add_terminal_output(format!("KOTA: {}", &response[..response.len().min(100)]));
        
        // Check for S/R blocks
        if sr_parser::contains_sr_blocks(&response) {
            match sr_parser::parse_sr_blocks(&response) {
                Ok(blocks) => {
                    if !blocks.is_empty() {
                        self.add_terminal_output(format!("Found {} S/R blocks - applying changes...", blocks.len()));
                        
                        // Apply blocks (simplified for TUI)
                        let apply_result = {
                            if let Ok(cm) = self.context_manager.lock() {
                                editor::confirm_and_apply_blocks(blocks, &original_prompt, &cm).await
                            } else {
                                Err(anyhow::anyhow!("Could not access context manager"))
                            }
                        };
                        
                        match apply_result {
                            Ok(_) => {
                                self.add_terminal_output("Changes applied successfully".to_string());
                                self.update_context_view();
                            }
                            Err(e) => {
                                self.add_terminal_output(format!("Error applying changes: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.add_terminal_output(format!("Error parsing S/R blocks: {}", e));
                }
            }
        }
        
        // Check for command blocks
        if cmd_parser::contains_command_blocks(&response) {
            match cmd_parser::parse_command_blocks(&response) {
                Ok(cmd_blocks) => {
                    if !cmd_blocks.is_empty() {
                        self.add_terminal_output(format!("Found {} suggested command(s):", cmd_blocks.len()));
                        
                        // Show suggested commands in terminal
                        for cmd_block in cmd_blocks.iter() {
                            self.add_suggested_command(cmd_block.command.clone());
                        }
                        
                        self.add_terminal_output("Press 'x' in terminal mode to execute commands".to_string());
                    }
                }
                Err(e) => {
                    self.add_terminal_output(format!("Error parsing command blocks: {}", e));
                }
            }
        }
    }
}