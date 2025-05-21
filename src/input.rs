use std::io::{self};
use anyhow::Result;
use colored::*;
use reedline::{Reedline, Signal, Vi, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode, 
               default_vi_normal_keybindings, default_vi_insert_keybindings, ReedlineEvent, KeyCode, KeyModifiers,
               Validator, ValidationResult};
use termimad::crossterm::{
    execute,
    terminal::{Clear, ClearType},
};

// Custom validator for determining when input should continue on multiple lines
pub struct KotaValidator;

impl Validator for KotaValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        // Check for various multiline indicators
        
        // 1. Triple backticks for code blocks (must have matching closing backticks)
        let backtick_count = line.matches("```").count();
        if backtick_count % 2 == 1 {
            return ValidationResult::Incomplete;
        }
        
        // 2. Check for open brackets/braces/parentheses
        let mut paren_count = 0;
        let mut brace_count = 0;
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' => escape_next = true,
                '"' | '\'' if !escape_next => in_string = !in_string,
                '(' if !in_string => paren_count += 1,
                ')' if !in_string => paren_count -= 1,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => brace_count -= 1,
                '[' if !in_string => bracket_count += 1,
                ']' if !in_string => bracket_count -= 1,
                _ => {}
            }
        }
        
        if paren_count > 0 || brace_count > 0 || bracket_count > 0 || in_string {
            return ValidationResult::Incomplete;
        }
        
        // 3. Explicit line continuation with backslash at end
        if line.trim_end().ends_with('\\') {
            return ValidationResult::Incomplete;
        }
        
        // 4. Common patterns that suggest multiline input
        let trimmed = line.trim();
        
        // Function definitions, class definitions, etc.
        if trimmed.ends_with(':') && (
            trimmed.starts_with("def ") ||
            trimmed.starts_with("class ") ||
            trimmed.starts_with("if ") ||
            trimmed.starts_with("for ") ||
            trimmed.starts_with("while ") ||
            trimmed.starts_with("with ") ||
            trimmed.starts_with("try:") ||
            trimmed.starts_with("except") ||
            trimmed.starts_with("finally:")
        ) {
            return ValidationResult::Incomplete;
        }
        
        // Default to complete
        ValidationResult::Complete
    }
}

// Custom prompt that shows vim mode
pub struct KotaPrompt;

impl Prompt for KotaPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_right(&self) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> std::borrow::Cow<str> {
        match edit_mode {
            PromptEditMode::Default => "› ".bright_green().bold().to_string().into(),
            PromptEditMode::Emacs => "› ".bright_green().bold().to_string().into(),
            PromptEditMode::Vi(vi_mode) => {
                match vi_mode {
                    PromptViMode::Normal => "[N] ".dimmed().to_string().into(),
                    PromptViMode::Insert => "[I] ".green().to_string().into(),
                }
            }
            PromptEditMode::Custom(_) => "› ".bright_green().bold().to_string().into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<str> {
        "... ".dimmed().to_string().into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> std::borrow::Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        format!("({}reverse-search: {}) ", prefix, history_search.term).into()
    }
}

pub fn read_line_with_shortcuts() -> Result<String> {
    // Create Vi mode with custom keybindings
    let mut normal_keybindings = default_vi_normal_keybindings();
    let mut insert_keybindings = default_vi_insert_keybindings();
    
    // Add Ctrl+L to clear screen in both normal and insert modes
    normal_keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('l'),
        ReedlineEvent::ClearScreen,
    );
    insert_keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('l'),
        ReedlineEvent::ClearScreen,
    );
    
    let vi_mode = Vi::new(insert_keybindings, normal_keybindings);
    
    // Create a Reedline instance with Vi mode and multiline validator
    let mut line_editor = Reedline::create()
        .with_edit_mode(Box::new(vi_mode))
        .with_validator(Box::new(KotaValidator));
    
    let prompt = KotaPrompt;
    
    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                return Ok(buffer);
            }
            Ok(Signal::CtrlD) => {
                // Handle Ctrl+D
                println!();
                println!("{}", "Goodbye!".bright_white());
                std::process::exit(0);
            }
            Ok(Signal::CtrlC) => {
                // Handle Ctrl+C
                println!();
                println!("{}", "Goodbye!".bright_white());
                std::process::exit(0);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error reading input: {}", e));
            }
        }
    }
}

pub fn read_single_char() -> Result<char> {
    // For simple single character input, we'll still use the crossterm approach
    // since reedline is overkill for y/n confirmations
    use termimad::crossterm::{
        event::{self, Event, KeyEvent},
        terminal::{enable_raw_mode, disable_raw_mode},
    };
    // Use the correct KeyCode and KeyModifiers from crossterm
    use termimad::crossterm::event::{KeyCode as CrosstermKeyCode, KeyModifiers as CrosstermKeyModifiers};
    
    enable_raw_mode()?;
    
    loop {
        match event::read()? {
            Event::Key(KeyEvent {
                code: CrosstermKeyCode::Char('l'),
                modifiers: CrosstermKeyModifiers::CONTROL,
                ..
            }) => {
                // Handle Ctrl+L - clear screen
                execute!(io::stdout(), Clear(ClearType::All))?;
                execute!(io::stdout(), termimad::crossterm::cursor::MoveTo(0, 0))?;
                
                // Redraw the header  
                println!("{}", "═".repeat(60).bright_blue());
                println!("{}", "KOTA - AI Coding Assistant".bright_white().bold());
                println!("{}", "═".repeat(60).bright_blue());
                println!();
            }
            Event::Key(KeyEvent {
                code: CrosstermKeyCode::Char('c'),
                modifiers: CrosstermKeyModifiers::CONTROL,
                ..
            }) => {
                // Handle Ctrl+C - exit gracefully
                disable_raw_mode()?;
                println!();
                println!("{}", "Goodbye!".bright_white());
                std::process::exit(0);
            }
            Event::Key(KeyEvent {
                code: CrosstermKeyCode::Char(c),
                modifiers: CrosstermKeyModifiers::NONE | CrosstermKeyModifiers::SHIFT,
                ..
            }) => {
                disable_raw_mode()?;
                println!("{}", c);
                return Ok(c);
            }
            _ => {
                // Ignore other events
            }
        }
    }
}