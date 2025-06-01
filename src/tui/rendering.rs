use std::io;
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use crate::context::ContextManager;
use crate::llm::ModelConfig;

use super::app::App;
use super::types::{AppMessage, InputMode, FocusedPane};
use super::widgets;

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
    let header = widgets::create_header(app);
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
    let file_browser = widgets::create_file_browser(app);
    f.render_widget(file_browser, main_chunks[0]);
    
    // Chat/terminal area
    let chat_area_idx = if app.show_file_browser { 1 } else { 0 };
    let chat_terminal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[chat_area_idx]);
    
    // Chat history
    let chat = widgets::create_chat_view(app);
    f.render_widget(chat, chat_terminal_chunks[0]);
    
    // Terminal output
    let terminal = widgets::create_terminal_view(app);
    f.render_widget(terminal, chat_terminal_chunks[1]);
    
    // Context view
    let context_idx = if app.show_file_browser { 2 } else { 1 };
    let context = widgets::create_context_view(app);
    f.render_widget(context, main_chunks[context_idx]);
    
    // Input area
    let input = widgets::create_input_area(app);
    f.render_widget(input, chunks[2]);
    
    // Status bar
    let status_bar = widgets::create_status_bar(app);
    f.render_widget(status_bar, chunks[3]);
}