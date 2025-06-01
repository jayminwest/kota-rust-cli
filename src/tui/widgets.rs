use ratatui::{
    layout::{Alignment, Constraint},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap, Table, Row, Cell},
};

use crate::file_browser::FileBrowser;
use super::app::App;
use super::types::{MessageContent, CommandStatus, InputMode, FocusedPane};

pub fn process_markdown_for_display(content: &str) -> String {
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

pub fn create_header(app: &App) -> Paragraph {
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

pub fn create_chat_view(app: &App) -> Paragraph {
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

pub fn create_terminal_view(app: &App) -> Paragraph {
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

pub fn create_context_view(app: &App) -> Paragraph {
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

pub fn create_file_browser(app: &App) -> Table {
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

pub fn create_input_area(app: &App) -> Paragraph {
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

pub fn create_status_bar(app: &App) -> Paragraph {
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