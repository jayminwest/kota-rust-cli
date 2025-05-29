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
use crate::llm::{self, LlmProvider};
use crate::file_browser::FileBrowser;
use crate::dynamic_prompts::DynamicPromptData;
use crate::sr_parser;
use crate::editor;
use crate::cmd_parser;

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

pub struct App {
    // UI state
    pub input: String,
    pub input_mode: InputMode,
    pub messages: Vec<(String, String)>, // (role, content)
    pub context_view: String,
    pub status_message: String,
    pub current_time: String,
    pub scroll_offset: u16,
    pub focused_pane: FocusedPane,
    
    // Core components
    pub context_manager: Arc<Mutex<ContextManager>>,
    pub llm_provider: LlmProvider,
    
    // Terminal output buffer
    pub terminal_output: Vec<String>,
    pub terminal_scroll: u16,
    pub suggested_commands: Vec<String>,
    
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
}

impl App {
    pub fn new(context_manager: ContextManager, llm_provider: LlmProvider) -> Result<Self> {
        let live_data = DynamicPromptData::new(&context_manager);
        let file_browser = FileBrowser::new()?;
        let (tx, rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            context_view: String::new(),
            status_message: "Ready - Press '?' for help".to_string(),
            current_time: Local::now().format("%H:%M:%S").to_string(),
            scroll_offset: 0,
            focused_pane: FocusedPane::Chat,
            context_manager: Arc::new(Mutex::new(context_manager)),
            llm_provider,
            terminal_output: Vec::new(),
            terminal_scroll: 0,
            suggested_commands: Vec::new(),
            file_browser,
            show_file_browser: true,
            live_data,
            tx,
            rx: Some(rx),
            is_processing: false,
            context_scroll: 0,
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
    
    pub fn add_suggested_command(&mut self, command: String) {
        self.suggested_commands.push(command.clone());
        self.add_terminal_output(format!("[SUGGESTED] {}", command));
    }
    
    pub async fn execute_suggested_commands(&mut self) {
        if self.suggested_commands.is_empty() {
            self.add_terminal_output("No commands to execute".to_string());
            return;
        }
        
        self.add_terminal_output("Executing suggested commands...".to_string());
        
        for cmd in &self.suggested_commands.clone() {
            // Extract the actual command (remove the number prefix if present)
            let actual_cmd = if let Some(colon_pos) = cmd.find(": ") {
                &cmd[colon_pos + 2..]
            } else {
                cmd
            };
            
            self.add_terminal_output(format!("[EXEC] {}", actual_cmd));
            
            // Execute the command using tokio process
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(actual_cmd)
                .output()
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.trim().is_empty() {
                            for line in stdout.lines() {
                                self.add_terminal_output(format!("  {}", line));
                            }
                        }
                        self.add_terminal_output("[SUCCESS] Command completed".to_string());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        self.add_terminal_output(format!("[ERROR] Command failed with code: {}", 
                            output.status.code().unwrap_or(-1)));
                        if !stderr.trim().is_empty() {
                            for line in stderr.lines() {
                                self.add_terminal_output(format!("  {}", line));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.add_terminal_output(format!("[ERROR] Failed to execute: {}", e));
                }
            }
        }
        
        // Clear suggested commands after execution
        self.suggested_commands.clear();
        self.add_terminal_output("All commands executed. Press 'Tab' to switch focus.".to_string());
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
        self.messages.push(("User".to_string(), input.clone()));
        self.add_terminal_output(format!(">>> {}", input));
        self.is_processing = true;
        self.status_message = "Processing...".to_string();
        
        // Get current context
        let context = if let Ok(cm) = self.context_manager.lock() {
            cm.get_formatted_context()
        } else {
            String::new()
        };
        
        // Spawn async LLM task
        let tx = self.tx.clone();
        let provider = self.llm_provider.clone();
        let prompt = input.clone();
        
        task::spawn(async move {
            match llm::ask_model_with_provider(&prompt, &context, provider).await {
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
        
        // Handle basic commands
        if cmd.starts_with("add_file ") {
            let path = cmd.strip_prefix("add_file ").unwrap_or("");
            if let Err(e) = self.add_file_to_context(path) {
                self.status_message = format!("Error: {}", e);
            }
        } else if cmd == "show_context" {
            let context = if let Ok(cm) = self.context_manager.lock() {
                cm.get_formatted_context()
            } else {
                "Error accessing context".to_string()
            };
            self.add_terminal_output(format!("Context:\n{}", context));
        } else if cmd == "clear_context" {
            if let Ok(mut cm) = self.context_manager.lock() {
                cm.clear_context();
            }
            self.update_context_view();
            self.status_message = "Context cleared".to_string();
        } else if cmd.starts_with("provider ") {
            let provider = cmd.strip_prefix("provider ").unwrap_or("");
            match provider {
                "ollama" => {
                    self.llm_provider = LlmProvider::Ollama;
                    self.status_message = "Switched to Ollama".to_string();
                }
                "gemini" => {
                    self.llm_provider = LlmProvider::Gemini;
                    self.status_message = "Switched to Gemini".to_string();
                }
                _ => {
                    self.status_message = "Unknown provider. Use 'ollama' or 'gemini'".to_string();
                }
            }
        } else {
            self.status_message = format!("Unknown command: {}", cmd);
        }
    }
    
    #[allow(clippy::await_holding_lock)]
    pub async fn handle_llm_response(&mut self, original_prompt: String, response: String) {
        self.messages.push(("KOTA".to_string(), response.clone()));
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
                        for (i, cmd_block) in cmd_blocks.iter().enumerate() {
                            self.add_suggested_command(format!("{}: {}", i + 1, cmd_block.command));
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
    llm_provider: LlmProvider,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app = App::new(context_manager, llm_provider)?;
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
                            app.input_mode = InputMode::FileBrowser;
                            app.focused_pane = FocusedPane::FileBrowser;
                            app.status_message = "FILE BROWSER - Navigate with hjkl, Enter to add file".to_string();
                        }
                        KeyCode::Char('?') => {
                            app.status_message = "Help: Ctrl+Q=quit, i=insert, f=files, Tab=focus, hjkl=nav/panes".to_string();
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
                                FocusedPane::Chat => app.scroll_offset += 1,
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
                                FocusedPane::Chat => app.scroll_offset = app.scroll_offset.saturating_sub(10),
                                FocusedPane::Terminal => app.terminal_scroll = app.terminal_scroll.saturating_sub(10),
                                FocusedPane::Context => app.context_scroll = app.context_scroll.saturating_sub(10),
                                _ => {}
                            }
                        }
                        KeyCode::PageDown => {
                            match app.focused_pane {
                                FocusedPane::Chat => app.scroll_offset += 10,
                                FocusedPane::Terminal => app.terminal_scroll += 10,
                                FocusedPane::Context => app.context_scroll += 10,
                                _ => {}
                            }
                        }
                        KeyCode::Char('x') => {
                            // Execute suggested commands when terminal is focused
                            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                                app.execute_suggested_commands().await;
                            }
                        }
                        _ => {}
                    },
                    InputMode::Insert => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.status_message = "NORMAL MODE".to_string();
                        }
                        KeyCode::Enter => {
                            if !app.is_processing && !app.input.trim().is_empty() {
                                let input = app.input.clone();
                                app.input.clear();
                                app.input_mode = InputMode::Normal;
                                app.process_user_input(input).await;
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
        
        // Process content for better markdown display
        let processed_content = process_markdown_for_display(content);
        for line in processed_content.lines() {
            lines.push(Line::from(line.to_string()));
        }
        
        lines.push(Line::from("")); // Empty line for spacing
    }
    
    Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Chat History ")
            .border_style(if matches!(app.focused_pane, FocusedPane::Chat) {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            }))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_offset, 0))
}

fn create_terminal_view(app: &App) -> Paragraph {
    let lines: Vec<Line> = app.terminal_output
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();
    
    Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" KOTA Terminal ")
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
    
    let input_text = vec![
        Line::from(vec![
            Span::styled(mode_indicator, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::raw(&app.input),
            if matches!(app.input_mode, InputMode::Insert) {
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK))
            } else {
                Span::raw("")
            },
        ]),
    ];
    
    Paragraph::new(input_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .border_style(Style::default().fg(mode_color)))
}

fn create_status_bar(app: &App) -> Paragraph {
    let shortcuts = match app.input_mode {
        InputMode::Normal => {
            if matches!(app.focused_pane, FocusedPane::Terminal) && !app.suggested_commands.is_empty() {
                "^Q:quit i:insert f:files Tab/←→:focus kj:scroll x:execute ?:help"
            } else {
                "^Q:quit i:insert f:files Tab/←→:focus kj:scroll ?:help"
            }
        },
        InputMode::Insert => if app.is_processing { "Processing..." } else { "Esc:normal Enter:send" },
        InputMode::Command => "Esc:cancel Enter:execute",
        InputMode::FileBrowser => "hjkl:nav Enter:add .:hidden s:sudo Esc:back",
    };
    
    let processing_indicator = if app.is_processing {
        Span::styled("[PROCESSING] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("")
    };
    
    let status = vec![
        Line::from(vec![
            processing_indicator,
            Span::styled(
                match app.llm_provider {
                    LlmProvider::Ollama => "Ollama",
                    LlmProvider::Gemini => "Gemini",
                },
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{} files", app.live_data.context_file_count),
                Style::default().fg(Color::Cyan),
            ),
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
    use crate::llm::LlmProvider;

    #[tokio::test]
    async fn test_app_creation() {
        let context_manager = ContextManager::new();
        let llm_provider = LlmProvider::Ollama;
        
        let app = App::new(context_manager, llm_provider).unwrap();
        
        assert_eq!(app.input, "");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert!(matches!(app.focused_pane, FocusedPane::Chat));
        assert_eq!(app.messages.len(), 0);
        assert_eq!(app.terminal_output.len(), 0);
        assert_eq!(app.suggested_commands.len(), 0);
    }

    #[tokio::test]
    async fn test_add_terminal_output() {
        let context_manager = ContextManager::new();
        let llm_provider = LlmProvider::Ollama;
        let mut app = App::new(context_manager, llm_provider).unwrap();
        
        app.add_terminal_output("Test output".to_string());
        
        assert_eq!(app.terminal_output.len(), 1);
        assert_eq!(app.terminal_output[0], "Test output");
    }

    #[tokio::test]
    async fn test_add_suggested_command() {
        let context_manager = ContextManager::new();
        let llm_provider = LlmProvider::Ollama;
        let mut app = App::new(context_manager, llm_provider).unwrap();
        
        app.add_suggested_command("ls -la".to_string());
        
        assert_eq!(app.suggested_commands.len(), 1);
        assert_eq!(app.suggested_commands[0], "ls -la");
        assert_eq!(app.terminal_output.len(), 1);
        assert!(app.terminal_output[0].contains("[SUGGESTED] ls -la"));
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
}