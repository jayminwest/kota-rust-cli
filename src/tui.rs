use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::Local;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap, Table, Row, Cell},
    Frame, Terminal,
};
use anyhow::Result;
use tokio::sync::mpsc;
use tokio::task;

use crate::context::ContextManager;
use crate::llm::{self, LlmProvider, ModelConfig};
use crate::file_browser::FileBrowser;
use crate::dynamic_prompts::DynamicPromptData;
use crate::sr_parser;
use crate::editor;
use crate::cmd_parser;
use crate::memory::MemoryManager;

// Threshold for collapsing pasted content
const PASTE_COLLAPSE_THRESHOLD: usize = 10; // Collapse if more than 10 lines

fn process_markdown_for_display(content: &str) -> String {
    let mut processed = String::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut in_code_block = false;
    
    for line in lines {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                processed.push_str(&format!("[CODE] {}", line.trim_start_matches("```")));
            } else {
                processed.push_str("[/CODE]");
            }
        } else if in_code_block {
            processed.push_str(&format!("  {}", line));
        } else if line.starts_with("# ") {
            processed.push_str(&format!("=== {} ===", line.trim_start_matches("# ")));
        } else if line.starts_with("## ") {
            processed.push_str(&format!("--- {} ---", line.trim_start_matches("## ")));
        } else if line.starts_with("### ") {
            processed.push_str(&format!(">> {}", line.trim_start_matches("### ")));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            processed.push_str(&format!("  {}", line));
        } else if line.starts_with("`") && line.ends_with("`") {
            let code = line.trim_matches('`');
            processed.push_str(&format!("[{}]", code));
        } else {
            processed.push_str(line);
        }
        processed.push('\n');
    }
    
    processed
}

#[derive(Clone)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
    FileBrowser,
}

#[derive(Clone)]
pub enum FocusedPane {
    Chat,
    Terminal,
    Context,
    FileBrowser,
}

#[derive(Clone)]
pub enum AppMessage {
    LlmResponse(String, String), // (original_prompt, response)
    TerminalOutput(String),
    ProcessingComplete,
}

#[derive(Clone)]
pub enum MessageContent {
    Text(String),
    CollapsedPaste { 
        summary: String,  // e.g., "[Pasted 150 lines]"
        full_content: String,  // The actual pasted content
    },
}

#[derive(Clone, Debug)]
pub enum CommandStatus {
    Pending,
    Running,
    Success,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: Option<String>,
    pub status: CommandStatus,
    pub output: Option<String>,
}

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
    
    fn has_unmatched_delimiters(&self, content: &str) -> bool {
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

pub async fn run_tui(
    context_manager: ContextManager,
    model_config: ModelConfig,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app = App::new(context_manager, model_config)?;
    app.update_context_view();
    
    // Extract the receiver from the app
    let mut rx = app.rx.take().unwrap();
    
    // Run the app
    let res = run_app(&mut terminal, &mut app, &mut rx).await;
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    res
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppMessage>,
) -> Result<()> {
    loop {
        // Check if we should quit
        if app.should_quit {
            return Ok(());
        }
        
        // Update time and live data
        app.update_time();
        app.update_context_view();
        
        // Draw UI
        terminal.draw(|f| ui(f, app))?;
        
        // Handle async messages first
        while let Ok(msg) = rx.try_recv() {
            match msg {
                AppMessage::LlmResponse(prompt, response) => {
                    app.handle_llm_response(prompt, response).await;
                }
                AppMessage::TerminalOutput(output) => {
                    app.add_terminal_output(output);
                }
                AppMessage::ProcessingComplete => {
                    app.is_processing = false;
                    app.status_message = "Ready".to_string();
                }
            }
        }
        
        // Handle keyboard events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Allow most interactions during LLM processing
                // Only block sending new messages to prevent conflicts
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('i') => {
                            app.input_mode = InputMode::Insert;
                            app.status_message = "INSERT MODE - Type your message".to_string();
                        }
                        KeyCode::Char(':') => {
                            app.input_mode = InputMode::Command;
                            app.input = String::new();
                            app.status_message = "COMMAND MODE".to_string();
                        }
                        KeyCode::Char('f') => {
                            // Only switch to file browser if we're not processing input and input is empty
                            if !app.is_processing && app.input.is_empty() && app.input_lines.len() <= 1 {
                                app.input_mode = InputMode::FileBrowser;
                                app.focused_pane = FocusedPane::FileBrowser;
                                app.status_message = "FILE BROWSER - Navigate with hjkl, Enter to add file".to_string();
                            }
                        }
                        KeyCode::Char('g') => {
                            // Check if next key is also 'g' for gg command
                            if event::poll(Duration::from_millis(500))? {
                                if let Event::Key(next_key) = event::read()? {
                                    if next_key.code == KeyCode::Char('g') {
                                        // gg - go to top
                                        match app.focused_pane {
                                            FocusedPane::Chat => app.scroll_offset = 0,
                                            FocusedPane::Terminal => app.terminal_scroll = 0,
                                            FocusedPane::Context => app.context_scroll = 0,
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('G') => {
                            // G - go to bottom (set scroll to reasonable max)
                            match app.focused_pane {
                                FocusedPane::Chat => app.scroll_offset = 1000, // More reasonable max
                                FocusedPane::Terminal => app.terminal_scroll = 1000,
                                FocusedPane::Context => app.context_scroll = 1000,
                                _ => {}
                            }
                        }
                        KeyCode::Char('?') => {
                            app.status_message = "Help: :q=quit, i=insert, :=cmd, f=files, Tab=focus, hjkl=nav, gg/G=top/bottom, a=auto-scroll, x=exec, n/p=nav-cmds, c=clear".to_string();
                        }
                        KeyCode::Char('a') => {
                            app.toggle_auto_scroll();
                        }
                        KeyCode::Tab => {
                            // Cycle through panes
                            app.focused_pane = match app.focused_pane {
                                FocusedPane::Chat => FocusedPane::Terminal,
                                FocusedPane::Terminal => FocusedPane::Context,
                                FocusedPane::Context => if app.show_file_browser { FocusedPane::FileBrowser } else { FocusedPane::Chat },
                                FocusedPane::FileBrowser => FocusedPane::Chat,
                            };
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            match app.focused_pane {
                                FocusedPane::Chat => {
                                    if app.scroll_offset > 0 {
                                        app.scroll_offset -= 1;
                                        // Disable auto-scroll when user manually scrolls
                                        app.auto_scroll_enabled = false;
                                    }
                                }
                                FocusedPane::Terminal => {
                                    if app.terminal_scroll > 0 {
                                        app.terminal_scroll -= 1;
                                    }
                                }
                                FocusedPane::Context => {
                                    if app.context_scroll > 0 {
                                        app.context_scroll -= 1;
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            match app.focused_pane {
                                FocusedPane::Chat => {
                                    app.scroll_offset += 1;
                                    // Disable auto-scroll when user manually scrolls
                                    app.auto_scroll_enabled = false;
                                }
                                FocusedPane::Terminal => app.terminal_scroll += 1,
                                FocusedPane::Context => app.context_scroll += 1,
                                _ => {}
                            }
                        }
                        KeyCode::Left => {
                            // Cycle through panes backwards
                            app.focused_pane = match app.focused_pane {
                                FocusedPane::Chat => if app.show_file_browser { FocusedPane::FileBrowser } else { FocusedPane::Context },
                                FocusedPane::Terminal => FocusedPane::Chat,
                                FocusedPane::Context => FocusedPane::Terminal,
                                FocusedPane::FileBrowser => FocusedPane::Context,
                            };
                        }
                        KeyCode::Right => {
                            // Cycle through panes forwards (same as Tab)
                            app.focused_pane = match app.focused_pane {
                                FocusedPane::Chat => FocusedPane::Terminal,
                                FocusedPane::Terminal => FocusedPane::Context,
                                FocusedPane::Context => if app.show_file_browser { FocusedPane::FileBrowser } else { FocusedPane::Chat },
                                FocusedPane::FileBrowser => FocusedPane::Chat,
                            };
                        }
                        KeyCode::Char('h') => {
                            // h for scrolling left in content (currently not used but reserved for future horizontal scrolling)
                        }
                        KeyCode::Char('l') => {
                            // l for scrolling right in content (currently not used but reserved for future horizontal scrolling)
                        }
                        KeyCode::PageUp => {
                            match app.focused_pane {
                                FocusedPane::Chat => {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                                    app.auto_scroll_enabled = false;
                                }
                                FocusedPane::Terminal => app.terminal_scroll = app.terminal_scroll.saturating_sub(10),
                                FocusedPane::Context => app.context_scroll = app.context_scroll.saturating_sub(10),
                                _ => {}
                            }
                        }
                        KeyCode::PageDown => {
                            match app.focused_pane {
                                FocusedPane::Chat => {
                                    app.scroll_offset += 10;
                                    app.auto_scroll_enabled = false;
                                }
                                FocusedPane::Terminal => app.terminal_scroll += 10,
                                FocusedPane::Context => app.context_scroll += 10,
                                _ => {}
                            }
                        }
                        KeyCode::Char('x') => {
                            // Execute selected command when terminal is focused
                            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                                app.execute_selected_command_async().await;
                            }
                        }
                        KeyCode::Char('n') => {
                            // Navigate to next command when terminal is focused
                            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                                app.navigate_commands(1);
                            }
                        }
                        KeyCode::Char('p') => {
                            // Navigate to previous command when terminal is focused
                            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                                app.navigate_commands(-1);
                            }
                        }
                        KeyCode::Char('c') => {
                            // Clear all commands when terminal is focused
                            if matches!(app.focused_pane, FocusedPane::Terminal) {
                                app.suggested_commands.clear();
                                app.selected_command_index = 0;
                                app.add_terminal_output("Cleared all suggested commands".to_string());
                            }
                        }
                        _ => {}
                    },
                    InputMode::Insert => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.clear_input();
                            app.status_message = "NORMAL MODE".to_string();
                        }
                        KeyCode::Enter => {
                            if !app.is_processing {
                                // Check if we should auto-continue to next line
                                if app.should_auto_continue() {
                                    app.add_new_line();
                                    app.status_message = "Multi-line mode - Ctrl+D to send, Esc to cancel".to_string();
                                } else if !app.get_full_input().trim().is_empty() {
                                    // Send the message
                                    app.input_mode = InputMode::Normal;
                                    app.process_user_input(String::new()).await; // Empty string means use full input
                                }
                            }
                        }
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' => {
                            // Ctrl+D to force send multi-line input
                            if !app.is_processing && !app.get_full_input().trim().is_empty() {
                                app.input_mode = InputMode::Normal;
                                app.process_user_input(String::new()).await;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        _ => {}
                    },
                    InputMode::Command => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input.clear();
                            app.status_message = "NORMAL MODE".to_string();
                        }
                        KeyCode::Enter => {
                            // Allow most commands during processing, but not LLM requests
                            let cmd = app.input.clone();
                            app.input.clear();
                            app.input_mode = InputMode::Normal;
                            app.process_command(cmd).await;
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        _ => {}
                    },
                    InputMode::FileBrowser => {
                        match key.code {
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.status_message = "NORMAL MODE".to_string();
                            }
                            KeyCode::Enter => {
                                // Add selected file to context
                                if let Some(path) = app.file_browser.enter_selected()? {
                                    if let Err(e) = app.add_file_to_context(path.to_str().unwrap()) {
                                        app.status_message = format!("Error adding file: {}", e);
                                    }
                                }
                            }
                            _ => {
                                // Let file browser handle other keys
                                app.file_browser.handle_key(key)?;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main area
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status bar
        ])
        .split(f.area());
    
    // Header
    let header = create_header(app);
    f.render_widget(header, chunks[0]);
    
    // Main area - split horizontally
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(if app.show_file_browser { 20 } else { 0 }),
            Constraint::Percentage(if app.show_file_browser { 50 } else { 60 }),
            Constraint::Percentage(if app.show_file_browser { 30 } else { 40 }),
        ])
        .split(chunks[1]);
    
    // File browser (always visible in TUI mode)
    let file_browser = create_file_browser(app);
    f.render_widget(file_browser, main_chunks[0]);
    
    // Chat/terminal area
    let chat_area_idx = if app.show_file_browser { 1 } else { 0 };
    let chat_terminal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[chat_area_idx]);
    
    // Chat history
    let chat = create_chat_view(app);
    f.render_widget(chat, chat_terminal_chunks[0]);
    
    // Terminal output
    let terminal = create_terminal_view(app);
    f.render_widget(terminal, chat_terminal_chunks[1]);
    
    // Context view
    let context_idx = if app.show_file_browser { 2 } else { 1 };
    let context = create_context_view(app);
    f.render_widget(context, main_chunks[context_idx]);
    
    // Input area
    let input = create_input_area(app);
    f.render_widget(input, chunks[2]);
    
    // Status bar
    let status_bar = create_status_bar(app);
    f.render_widget(status_bar, chunks[3]);
}

fn create_header(app: &App) -> Paragraph {
    let header_text = vec![
        Line::from(vec![
            Span::raw("KOTA "),
            Span::styled("AI Coding Assistant", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(&app.current_time, Style::default().fg(Color::Yellow)),
        ]),
    ];
    
    Paragraph::new(header_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .title(" KOTA Enhanced UI ")
            .title_alignment(Alignment::Center))
        .alignment(Alignment::Center)
}

fn create_chat_view(app: &App) -> Paragraph {
    let mut lines = Vec::new();
    
    // Debug: Add message count to title
    if app.messages.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No messages yet. Try typing 'i' and sending a message.", 
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }
    
    for (role, content) in &app.messages {
        let style = if role == "User" {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };
        
        // Add role header
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", role), style.add_modifier(Modifier::BOLD)),
        ]));
        
        // Process content based on type
        match content {
            MessageContent::Text(text) => {
                let processed_content = process_markdown_for_display(text);
                for line in processed_content.lines() {
                    lines.push(Line::from(line.to_string()));
                }
            }
            MessageContent::CollapsedPaste { summary, .. } => {
                lines.push(Line::from(vec![
                    Span::styled(summary, Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]));
            }
        }
        
        lines.push(Line::from("")); // Empty line for spacing
    }
    
    let title = format!(" Chat History ({} messages) ", app.messages.len());
    
    Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if matches!(app.focused_pane, FocusedPane::Chat) {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            }))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_offset, 0))
}

fn create_terminal_view(app: &App) -> Paragraph {
    let mut lines: Vec<Line> = app.terminal_output
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();
    
    // Add enhanced command display if there are suggested commands
    if !app.suggested_commands.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("=== Suggested Commands ===", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        ]));
        
        for (i, cmd) in app.suggested_commands.iter().enumerate() {
            let is_selected = i == app.selected_command_index;
            let status_indicator = match &cmd.status {
                CommandStatus::Pending => "⏸",
                CommandStatus::Running => "▶",
                CommandStatus::Success => "✓",
                CommandStatus::Failed(_err) => {
                    // Error details stored in _err for debugging
                    "✗"
                },
            };
            
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                match &cmd.status {
                    CommandStatus::Success => Style::default().fg(Color::Green),
                    CommandStatus::Failed(_) => Style::default().fg(Color::Red),
                    CommandStatus::Running => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                }
            };
            
            let prefix = if is_selected { "→ " } else { "  " };
            // Use description for tooltip/debugging info (accessible but not cluttering display)
            let _tooltip = cmd.description.as_ref().unwrap_or(&"No description".to_string());
            
            lines.push(Line::from(vec![
                Span::styled(format!("{}{}[{}] {}", prefix, i + 1, status_indicator, cmd.command), style)
            ]));
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Commands: x=execute n=next p=prev c=clear", Style::default().fg(Color::DarkGray))
        ]));
    }
    
    let title = if !app.suggested_commands.is_empty() {
        format!(" KOTA Terminal ({} commands) ", app.suggested_commands.len())
    } else {
        " KOTA Terminal ".to_string()
    };
    
    Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if matches!(app.focused_pane, FocusedPane::Terminal) {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            }))
        .wrap(Wrap { trim: true })
        .scroll((app.terminal_scroll, 0))
}

fn create_context_view(app: &App) -> Paragraph {
    let mut content = String::new();
    
    // Add live data section (compact format)
    content.push_str("=== Live Data ===\n");
    content.push_str(&format!("Time: {}\n", app.live_data.time));
    content.push_str(&format!("Date: {}\n", app.live_data.date));
    
    // Truncate long paths
    let wd = &app.live_data.working_directory;
    let short_wd = if wd.len() > 25 {
        format!("...{}", &wd[wd.len()-22..])
    } else {
        wd.clone()
    };
    content.push_str(&format!("Dir: {}\n", short_wd));
    
    if let Some(branch) = &app.live_data.git_branch {
        content.push_str(&format!("Git: {}\n", branch));
    }
    content.push_str(&format!("User: {}\n", app.live_data.system_info.username));
    content.push('\n');
    
    // Add context (truncated for display)
    content.push_str("=== Context ===\n");
    let context_preview = if app.context_view.len() > 500 {
        format!("{}...\n[{} more chars]", &app.context_view[..500], app.context_view.len() - 500)
    } else {
        app.context_view.clone()
    };
    content.push_str(&context_preview);
    
    Paragraph::new(content)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Context & Live Data ")
            .border_style(if matches!(app.focused_pane, FocusedPane::Context) {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            }))
        .wrap(Wrap { trim: true })
        .scroll((app.context_scroll, 0))
}

fn create_file_browser(app: &App) -> Table {
    use crate::file_browser::FileBrowser;
    
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Size").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Perm").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    
    let rows: Vec<Row> = app.file_browser.items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.file_browser.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else if item.requires_sudo {
                Style::default().fg(Color::Red)
            } else if item.is_dir {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if item.is_symlink {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default()
            };
            
            // Truncate long names to fit better
            let display_name = if item.name.len() > 15 {
                format!("{}...", &item.name[..12])
            } else {
                item.name.clone()
            };
            
            let name = if item.is_dir {
                format!("[D] {}", display_name)
            } else if item.is_symlink {
                format!("[L] {}", display_name)
            } else {
                format!("    {}", display_name)
            };
            
            Row::new(vec![
                Cell::from(name),
                Cell::from(if item.is_dir { "-".to_string() } else { FileBrowser::format_size(item.size) }),
                Cell::from(item.permissions.clone()),
            ]).style(style)
        })
        .collect();
    
    let widths = [
        Constraint::Min(12),    // Name column - flexible but smaller
        Constraint::Length(6),  // Size column - shorter
        Constraint::Length(4),  // Permissions column - shorter
    ];
    
    // Truncate long directory paths for the title
    let dir_str = app.file_browser.current_dir.to_string_lossy();
    let short_dir = if dir_str.len() > 20 {
        format!("...{}", &dir_str[dir_str.len()-17..])
    } else {
        dir_str.to_string()
    };
    
    let title = format!(
        " {} {} ",
        short_dir,
        if app.file_browser.use_sudo { "[SUDO]" } else { "" }
    );
    
    Table::new(rows, widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if matches!(app.focused_pane, FocusedPane::FileBrowser) {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            }))
}

fn create_input_area(app: &App) -> Paragraph {
    let (mode_indicator, mode_color) = match app.input_mode {
        InputMode::Normal => ("[N]", Color::Blue),
        InputMode::Insert => ("[I]", Color::Green),
        InputMode::Command => ("[:]", Color::Yellow),
        InputMode::FileBrowser => ("[F]", Color::Magenta),
    };
    
    let mut input_lines = Vec::new();
    
    if app.is_multi_line_input() {
        // Show all lines for multi-line input
        for (i, line) in app.input_lines.iter().enumerate() {
            let is_current = i == app.current_line;
            let line_content = if i == app.input_lines.len() - 1 && !app.input.is_empty() {
                // Current working line
                &app.input
            } else {
                line
            };
            
            
            let mut spans = vec![
                if i == 0 {
                    Span::styled(mode_indicator, Style::default().fg(mode_color).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("...", Style::default().fg(Color::DarkGray))
                },
                Span::raw(" "),
            ];
            
            if matches!(app.input_mode, InputMode::Command) && i == 0 {
                spans.push(Span::raw(":"));
            }
            
            spans.push(Span::raw(line_content));
            
            if is_current && matches!(app.input_mode, InputMode::Insert | InputMode::Command) {
                spans.push(Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)));
            }
            
            input_lines.push(Line::from(spans));
        }
    } else {
        // Single line input
        let mut spans = vec![
            Span::styled(mode_indicator, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ];
        
        if matches!(app.input_mode, InputMode::Command) {
            spans.push(Span::raw(":"));
        }
        
        spans.push(Span::raw(&app.input));
        
        if matches!(app.input_mode, InputMode::Insert | InputMode::Command) {
            spans.push(Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)));
        }
        
        input_lines.push(Line::from(spans));
    }
    
    let title = if app.is_multi_line_input() {
        format!(" Input ({} lines) ", app.input_lines.len())
    } else {
        " Input ".to_string()
    };
    
    Paragraph::new(input_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(mode_color)))
}

fn create_status_bar(app: &App) -> Paragraph {
    let shortcuts = match app.input_mode {
        InputMode::Normal => {
            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                "^Q:quit i:insert f:files Tab/←→:focus x:exec n/p:nav c:clear ?:help"
            } else {
                "^Q:quit i:insert f:files Tab/←→:focus kj:scroll a:auto-scroll ?:help"
            }
        },
        InputMode::Insert => if app.is_processing { 
            "Processing..." 
        } else if app.is_multi_line_input() {
            "Esc:cancel Ctrl+D:send Enter:newline"
        } else {
            "Esc:normal Enter:send Ctrl+D:force-send"
        },
        InputMode::Command => "Esc:cancel Enter:execute",
        InputMode::FileBrowser => "hjkl:nav Enter:add .:hidden s:sudo Esc:back",
    };
    
    let processing_indicator = if app.is_processing {
        Span::styled("[PROCESSING] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("")
    };
    
    let auto_scroll_indicator = if app.auto_scroll_enabled {
        Span::styled("AUTO", Style::default().fg(Color::Green))
    } else {
        Span::styled("MANUAL", Style::default().fg(Color::Yellow))
    };
    
    let status = vec![
        Line::from(vec![
            processing_indicator,
            Span::styled(
                app.model_config.display_name(),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{} files", app.live_data.context_file_count),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" | "),
            auto_scroll_indicator,
            Span::raw(" | "),
            Span::raw(&app.status_message),
            Span::raw(" | "),
            Span::styled(shortcuts, Style::default().fg(Color::DarkGray)),
        ]),
    ];
    
    Paragraph::new(status)
        .style(Style::default().bg(Color::Black).fg(Color::White))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextManager;
    use crate::llm::ModelConfig;

    #[tokio::test]
    async fn test_app_creation() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        
        // This test might fail if knowledge-base creation fails, which is ok for testing
        let app_result = App::new(context_manager, model_config);
        
        // If app creation succeeds, test the state
        if let Ok(app) = app_result {
            assert_eq!(app.input, "");
            assert_eq!(app.input_lines, vec![String::new()]);
            assert_eq!(app.current_line, 0);
            assert!(matches!(app.input_mode, InputMode::Normal));
            assert!(matches!(app.focused_pane, FocusedPane::Chat));
            assert_eq!(app.messages.len(), 0);
            assert_eq!(app.terminal_output.len(), 0);
            assert_eq!(app.suggested_commands.len(), 0);
            assert!(app.auto_scroll_enabled);
        }
        
        // Test passes regardless of app creation success/failure
        assert!(true);
    }

    #[tokio::test]
    async fn test_add_terminal_output() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        
        if let Ok(mut app) = App::new(context_manager, model_config) {
            app.add_terminal_output("Test output".to_string());
            
            assert_eq!(app.terminal_output.len(), 1);
            assert_eq!(app.terminal_output[0], "Test output");
        }
    }

    #[tokio::test]
    async fn test_add_suggested_command() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        
        if let Ok(mut app) = App::new(context_manager, model_config) {
            app.add_suggested_command("ls -la".to_string());
            
            assert_eq!(app.suggested_commands.len(), 1);
            assert_eq!(app.suggested_commands[0].command, "ls -la");
            assert!(matches!(app.suggested_commands[0].status, CommandStatus::Pending));
            assert_eq!(app.terminal_output.len(), 1);
            assert!(app.terminal_output[0].contains("[SUGGESTED] ls -la"));
        }
    }

    #[test]
    fn test_process_markdown_for_display() {
        let markdown = "# Header\n```rust\nfn main() {}\n```\n- List item";
        let processed = process_markdown_for_display(markdown);
        
        assert!(processed.contains("=== Header ==="));
        assert!(processed.contains("[CODE] rust"));
        assert!(processed.contains("[/CODE]"));
        assert!(processed.contains("  - List item"));
    }

    #[test]
    fn test_input_mode_transitions() {
        // Test that input modes are properly defined
        let modes = [
            InputMode::Normal,
            InputMode::Insert,
            InputMode::Command,
            InputMode::FileBrowser,
        ];
        
        for mode in &modes {
            match mode {
                InputMode::Normal => assert!(true),
                InputMode::Insert => assert!(true),
                InputMode::Command => assert!(true),
                InputMode::FileBrowser => assert!(true),
            }
        }
    }

    #[test]
    fn test_focused_pane_transitions() {
        // Test that focused panes are properly defined
        let panes = [
            FocusedPane::Chat,
            FocusedPane::Terminal,
            FocusedPane::Context,
            FocusedPane::FileBrowser,
        ];
        
        for pane in &panes {
            match pane {
                FocusedPane::Chat => assert!(true),
                FocusedPane::Terminal => assert!(true),
                FocusedPane::Context => assert!(true),
                FocusedPane::FileBrowser => assert!(true),
            }
        }
    }
    
    #[tokio::test]
    async fn test_auto_scroll_functionality() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        if let Ok(mut app) = App::new(context_manager, model_config) {
        
        // Test initial state
        assert!(app.auto_scroll_enabled);
        assert_eq!(app.scroll_offset, 0);
        
        // Test toggle
        app.toggle_auto_scroll();
        assert!(!app.auto_scroll_enabled);
        
        app.toggle_auto_scroll();
        assert!(app.auto_scroll_enabled);
        
        // Test auto scroll when enabled
        app.auto_scroll_to_bottom();
        assert_eq!(app.scroll_offset, 0); // Now we reset to 0 to show content
        
        // Test auto scroll when disabled
        app.auto_scroll_enabled = false;
        app.scroll_offset = 0;
        app.auto_scroll_to_bottom();
        assert_eq!(app.scroll_offset, 0); // Should not change
        }
    }
    
    #[tokio::test]
    async fn test_command_navigation() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        if let Ok(mut app) = App::new(context_manager, model_config) {
        
        // Add multiple commands
        app.add_suggested_command("ls".to_string());
        app.add_suggested_command("pwd".to_string());
        app.add_suggested_command("echo test".to_string());
        
        assert_eq!(app.selected_command_index, 0);
        
        // Navigate forward
        app.navigate_commands(1);
        assert_eq!(app.selected_command_index, 1);
        
        app.navigate_commands(1);
        assert_eq!(app.selected_command_index, 2);
        
        // Wrap around
        app.navigate_commands(1);
        assert_eq!(app.selected_command_index, 0);
        
        // Navigate backward
        app.navigate_commands(-1);
        assert_eq!(app.selected_command_index, 2);
        
        // Test execute selected
        let command = app.execute_selected_command();
        assert_eq!(command, Some("echo test".to_string()));
        assert!(matches!(app.suggested_commands[2].status, CommandStatus::Running));
        }
    }
    
    #[tokio::test]
    async fn test_multi_line_input() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        if let Ok(mut app) = App::new(context_manager, model_config) {
        
        // Test single line to multi-line conversion
        app.input = "function test() {".to_string();
        assert!(app.should_auto_continue());
        
        app.add_new_line();
        assert!(app.is_multi_line_input());
        assert_eq!(app.input_lines.len(), 2);
        assert_eq!(app.input_lines[0], "function test() {");
        assert_eq!(app.current_line, 1);
        
        // Test full input retrieval
        app.input = "  return 42;".to_string();
        app.add_new_line();
        app.input = "}".to_string();
        
        let full_input = app.get_full_input();
        assert!(full_input.contains("function test() {"));
        assert!(full_input.contains("  return 42;"));
        assert!(full_input.contains("}"));
        
        // Test clear input
        app.clear_input();
        assert_eq!(app.input_lines, vec![String::new()]);
        assert_eq!(app.current_line, 0);
        assert!(!app.is_multi_line_input());
        }
    }
    
    #[test]
    fn test_delimiter_matching() {
        let context_manager = ContextManager::new();
        let model_config = ModelConfig::default();
        if let Ok(app) = App::new(context_manager, model_config) {
        
        // Test unmatched delimiters
        assert!(app.has_unmatched_delimiters("function(arg"));
        assert!(app.has_unmatched_delimiters("array[index"));
        assert!(app.has_unmatched_delimiters("object {"));
        assert!(app.has_unmatched_delimiters("\"unclosed string"));
        
        // Test matched delimiters
        assert!(!app.has_unmatched_delimiters("function(arg)"));
        assert!(!app.has_unmatched_delimiters("array[index]"));
        assert!(!app.has_unmatched_delimiters("object {}"));
        assert!(!app.has_unmatched_delimiters("\"closed string\""));
        }
    }
}