#![allow(dead_code)]

use std::ops::ControlFlow;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::tui::{App, AppMessage, InputMode, FocusedPane};

/// Handles all async messages from the channel
pub async fn handle_app_messages(
    app: &mut App, 
    rx: &mut mpsc::UnboundedReceiver<AppMessage>
) -> Result<()> {
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
    Ok(())
}

/// Main keyboard event dispatcher
pub async fn handle_keyboard_event(app: &mut App, key: KeyEvent) -> Result<ControlFlow<()>> {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key).await,
        InputMode::Insert => handle_insert_mode_key(app, key).await,
        InputMode::Command => handle_command_mode_key(app, key).await,
        InputMode::FileBrowser => handle_file_browser_mode_key(app, key).await,
    }
}

/// Handles keyboard events in normal mode
async fn handle_normal_mode_key(app: &mut App, key: KeyEvent) -> Result<ControlFlow<()>> {
    match key.code {
        KeyCode::Char('q') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(ControlFlow::Break(()));
            }
        }
        KeyCode::Char('i') => {
            app.input_mode = InputMode::Insert;
        }
        KeyCode::Char('f') => {
            app.input_mode = InputMode::FileBrowser;
            app.focused_pane = FocusedPane::FileBrowser;
        }
        KeyCode::Char('a') => {
            app.auto_scroll_enabled = !app.auto_scroll_enabled;
        }
        KeyCode::Char('?') => {
            // Show help - for now just update status message
            app.status_message = "Help: i=insert, f=file browser, a=auto-scroll, q=quit, Tab=switch panes".to_string();
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Redraw header - for now just update time
            app.update_time();
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(ControlFlow::Break(()));
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(ControlFlow::Break(()));
        }
        KeyCode::Tab => {
            handle_pane_focus_change(app, true);
        }
        KeyCode::BackTab => {
            handle_pane_focus_change(app, false);
        }
        // Navigation keys
        KeyCode::Char('h') | KeyCode::Left => {
            handle_navigation_keys(app, KeyCode::Left);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            handle_navigation_keys(app, KeyCode::Down);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            handle_navigation_keys(app, KeyCode::Up);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            handle_navigation_keys(app, KeyCode::Right);
        }
        // Page navigation
        KeyCode::PageUp => {
            handle_page_navigation(app, true);
        }
        KeyCode::PageDown => {
            handle_page_navigation(app, false);
        }
        // Handle 'gg' command sequence - simplified for now
        KeyCode::Char('g') => {
            // For now, just go to top immediately
            handle_go_to_top(app);
        }
        // Terminal-specific commands when terminal is focused
        KeyCode::Char(c) if matches!(app.focused_pane, FocusedPane::Terminal) => {
            handle_terminal_commands(app, c).await?;
        }
        _ => {
            // Reset last_key for any other key
            // Reset any tracking state if needed
        }
    }
    Ok(ControlFlow::Continue(()))
}

/// Handles keyboard events in insert mode
async fn handle_insert_mode_key(app: &mut App, key: KeyEvent) -> Result<ControlFlow<()>> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Force send 
            if !app.input.trim().is_empty() {
                let input_text = app.input.clone();
                app.input.clear();
                app.input_mode = InputMode::Normal;
                app.process_user_input(input_text).await;
            }
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Enter => {
            // Send the message - simplified for now
            if !app.input.trim().is_empty() {
                let input_text = app.input.clone();
                app.input.clear();
                app.input_mode = InputMode::Normal;
                
                // Process the input
                app.process_user_input(input_text).await;
            }
        }
        _ => {}
    }
    Ok(ControlFlow::Continue(()))
}

/// Handles keyboard events in command mode  
async fn handle_command_mode_key(app: &mut App, key: KeyEvent) -> Result<ControlFlow<()>> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Enter => {
            // Process command - simplified for now
            let command = app.input.clone();
            app.input.clear();
            app.input_mode = InputMode::Normal;
            app.status_message = format!("Command executed: {}", command);
        }
        _ => {}
    }
    Ok(ControlFlow::Continue(()))
}

/// Handles keyboard events in file browser mode
async fn handle_file_browser_mode_key(app: &mut App, key: KeyEvent) -> Result<ControlFlow<()>> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.focused_pane = FocusedPane::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.file_browser.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.file_browser.move_down();
        }
        KeyCode::Enter => {
            if let Some(selected) = app.file_browser.get_selected() {
                let path_str = selected.path.to_string_lossy().to_string();
                if let Err(e) = app.add_file_to_context(&path_str) {
                    app.add_terminal_output(format!("Error adding file: {}", e));
                } else {
                    app.add_terminal_output(format!("Added to context: {}", path_str));
                }
                app.input_mode = InputMode::Normal;
                app.focused_pane = FocusedPane::Chat;
            }
        }
        _ => {}
    }
    Ok(ControlFlow::Continue(()))
}

/// Handles pane focus changes (Tab/Shift+Tab)
fn handle_pane_focus_change(app: &mut App, forward: bool) {
    app.focused_pane = if forward {
        match app.focused_pane {
            FocusedPane::Chat => FocusedPane::Terminal,
            FocusedPane::Terminal => FocusedPane::Context,
            FocusedPane::Context => FocusedPane::FileBrowser,
            FocusedPane::FileBrowser => FocusedPane::Chat,
        }
    } else {
        match app.focused_pane {
            FocusedPane::Chat => FocusedPane::FileBrowser,
            FocusedPane::Terminal => FocusedPane::Chat,
            FocusedPane::Context => FocusedPane::Terminal,
            FocusedPane::FileBrowser => FocusedPane::Context,
        }
    };
}

/// Handles vim-style navigation keys
fn handle_navigation_keys(app: &mut App, key_code: KeyCode) {
    match app.focused_pane {
        FocusedPane::Chat => {
            handle_chat_navigation(app, key_code);
        }
        FocusedPane::Terminal => {
            handle_terminal_navigation(app, key_code);
        }
        FocusedPane::Context => {
            handle_context_navigation(app, key_code);
        }
        FocusedPane::FileBrowser => {
            handle_file_browser_navigation(app, key_code);
        }
    }
}

/// Handles navigation within the chat pane
fn handle_chat_navigation(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up | KeyCode::Left => {
            if app.scroll_offset > 0 {
                app.scroll_offset -= 1;
                app.auto_scroll_enabled = false;
            }
        }
        KeyCode::Down | KeyCode::Right => {
            let max_scroll = app.messages.len().saturating_sub(1);
            if app.scroll_offset < max_scroll as u16 {
                app.scroll_offset += 1;
            }
        }
        _ => {}
    }
}

/// Handles navigation within the terminal pane
fn handle_terminal_navigation(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up | KeyCode::Left => {
            if app.terminal_scroll > 0 {
                app.terminal_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Right => {
            let max_scroll = app.terminal_output.len().saturating_sub(1);
            if (app.terminal_scroll as usize) < max_scroll {
                app.terminal_scroll += 1;
            }
        }
        _ => {}
    }
}

/// Handles navigation within the context pane
fn handle_context_navigation(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up | KeyCode::Left => {
            if app.context_scroll > 0 {
                app.context_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Right => {
            // Context scrolling logic would go here
        }
        _ => {}
    }
}

/// Handles navigation within the file browser pane
fn handle_file_browser_navigation(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up | KeyCode::Left => {
            app.file_browser.move_up();
        }
        KeyCode::Down | KeyCode::Right => {
            app.file_browser.move_down();
        }
        _ => {}
    }
}

/// Handles page-based navigation (PageUp/PageDown)
fn handle_page_navigation(app: &mut App, page_up: bool) {
    match app.focused_pane {
        FocusedPane::Chat => {
            let scroll_amount = 10; // Scroll by 10 messages
            if page_up {
                app.scroll_offset = app.scroll_offset.saturating_sub(scroll_amount as u16);
                app.auto_scroll_enabled = false;
            } else {
                let max_scroll = app.messages.len().saturating_sub(1);
                app.scroll_offset = (app.scroll_offset + scroll_amount as u16).min(max_scroll as u16);
            }
        }
        FocusedPane::Terminal => {
            let scroll_amount = 10;
            if page_up {
                app.terminal_scroll = app.terminal_scroll.saturating_sub(scroll_amount);
            } else {
                let max_scroll = app.terminal_output.len().saturating_sub(1) as u16;
                app.terminal_scroll = (app.terminal_scroll + scroll_amount).min(max_scroll);
            }
        }
        _ => {}
    }
}

/// Handles the 'gg' command (go to top)
fn handle_go_to_top(app: &mut App) {
    match app.focused_pane {
        FocusedPane::Chat => {
            app.scroll_offset = 0;
            app.auto_scroll_enabled = false;
        }
        FocusedPane::Terminal => {
            app.terminal_scroll = 0;
        }
        _ => {}
    }
}

/// Handles terminal-specific commands (when terminal pane is focused)
async fn handle_terminal_commands(app: &mut App, key: char) -> Result<()> {
    match key {
        'n' => {
            app.navigate_commands(1);
        }
        'p' => {
            app.navigate_commands(-1);
        }
        'x' => {
            app.execute_selected_command_async().await;
        }
        _ => {}
    }
    Ok(())
}