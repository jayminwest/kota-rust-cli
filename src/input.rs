use anyhow::Result;
use colored::*;
use reedline::{
    default_vi_insert_keybindings, default_vi_normal_keybindings, KeyCode, KeyModifiers, Prompt,
    PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode, Reedline,
    ReedlineEvent, Signal, ValidationResult, Validator, Vi,
};
use std::io::{self};
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
        let mut in_double_quote = false;
        let mut in_single_quote = false;
        let mut escape_next = false;

        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => escape_next = true,
                '"' if !in_single_quote => in_double_quote = !in_double_quote,
                '\'' if !in_double_quote => in_single_quote = !in_single_quote,
                '(' if !in_double_quote && !in_single_quote => paren_count += 1,
                ')' if !in_double_quote && !in_single_quote => paren_count -= 1,
                '{' if !in_double_quote && !in_single_quote => brace_count += 1,
                '}' if !in_double_quote && !in_single_quote => brace_count -= 1,
                '[' if !in_double_quote && !in_single_quote => bracket_count += 1,
                ']' if !in_double_quote && !in_single_quote => bracket_count -= 1,
                _ => {}
            }
        }

        // Only consider string incomplete if we have unmatched structural elements
        // Don't treat single quotes/apostrophes in normal text as incomplete
        if paren_count > 0 || brace_count > 0 || bracket_count > 0 {
            return ValidationResult::Incomplete;
        }

        // Only treat unmatched quotes as incomplete if they appear to be starting strings
        // (i.e., if there are multiple quotes suggesting an intended string)
        let double_quote_count = line.chars().filter(|&c| c == '"').count();
        let single_quote_count = line.chars().filter(|&c| c == '\'').count();

        // If we have an odd number of quotes AND it looks like a structured context
        // (like JSON or code), then consider it incomplete
        if (double_quote_count % 2 == 1
            && (line.contains('{') || line.contains('[') || line.contains(':')))
            || (single_quote_count % 2 == 1
                && (line.contains('{') || line.contains('[') || line.contains(':')))
        {
            return ValidationResult::Incomplete;
        }

        // 3. Explicit line continuation with backslash at end
        if line.trim_end().ends_with('\\') {
            return ValidationResult::Incomplete;
        }

        // 4. Common patterns that suggest multiline input
        let trimmed = line.trim();

        // Function definitions, class definitions, etc.
        if trimmed.ends_with(':')
            && (trimmed.starts_with("def ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("if ")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("while ")
                || trimmed.starts_with("with ")
                || trimmed.starts_with("try:")
                || trimmed.starts_with("except")
                || trimmed.starts_with("finally:"))
        {
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
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => "[N] ".dimmed().to_string().into(),
                PromptViMode::Insert => "[I] ".green().to_string().into(),
            },
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

    // Read input from user with proper signal handling
    let sig = line_editor.read_line(&prompt)?;
    match sig {
        Signal::Success(buffer) => Ok(buffer),
        Signal::CtrlD => {
            // Handle Ctrl+D
            println!();
            println!("{}", "Goodbye!".bright_white());
            std::process::exit(0);
        }
        Signal::CtrlC => {
            // Handle Ctrl+C
            println!();
            println!("{}", "Goodbye!".bright_white());
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_handling() {
        let validator = KotaValidator;

        // These should be complete (normal input with quotes/apostrophes)
        assert!(matches!(
            validator.validate("I don't think so"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("She said \"hello\""),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("It's working now"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Multiple 'quotes' in text"),
            ValidationResult::Complete
        ));

        // These should be incomplete (structured data with unmatched quotes)
        assert!(matches!(
            validator.validate("{\"key\": \"value"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("['item1', 'item2"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("config: {'key': value"),
            ValidationResult::Incomplete
        ));

        // These should be complete (balanced quotes in structured data)
        assert!(matches!(
            validator.validate("{\"key\": \"value\"}"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("['item1', 'item2']"),
            ValidationResult::Complete
        ));

        // These should be incomplete (unmatched brackets)
        assert!(matches!(
            validator.validate("{ unmatched"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("(unmatched"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("[unmatched"),
            ValidationResult::Incomplete
        ));
    }

    #[test]
    fn test_quote_edge_cases() {
        let validator = KotaValidator;

        // Escaped quotes should be complete
        assert!(matches!(
            validator.validate("He said \"I don't know\""),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Path: \"C:\\\\Program Files\\\\\""),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Text with \\'escaped\\' quotes"),
            ValidationResult::Complete
        ));

        // Mixed quote types
        assert!(matches!(
            validator.validate("It's a \"test\" case"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("'He said \"hello\" to me'"),
            ValidationResult::Complete
        ));

        // Complex nested scenarios
        assert!(matches!(
            validator.validate("\"She said 'I don't know' yesterday\""),
            ValidationResult::Complete
        ));

        // Empty quotes
        assert!(matches!(
            validator.validate("Empty strings: \"\" and ''"),
            ValidationResult::Complete
        ));

        // Quotes in non-structured context (should be complete)
        assert!(matches!(
            validator.validate("Random text with ' unmatched quote"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Another \" unmatched quote here"),
            ValidationResult::Complete
        ));

        // Only incomplete if structured AND unmatched
        assert!(matches!(
            validator.validate("{\"incomplete"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("['incomplete"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("key: \"incomplete"),
            ValidationResult::Incomplete
        ));
    }

    #[test]
    fn test_bracket_matching() {
        let validator = KotaValidator;

        // Nested brackets should work
        assert!(matches!(
            validator.validate("{ nested: [1, 2, { inner: true }] }"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("((nested) parentheses)"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("[[nested], [arrays]]"),
            ValidationResult::Complete
        ));

        // Unmatched nested brackets
        assert!(matches!(
            validator.validate("{ nested: [1, 2, { inner: true }"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("((nested) parentheses"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("[[nested], [arrays]"),
            ValidationResult::Incomplete
        ));

        // Mixed bracket types
        assert!(matches!(
            validator.validate("{ array: [1, 2, (func())] }"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("{ array: [1, 2, (func()]"),
            ValidationResult::Incomplete
        ));

        // Brackets inside quotes should be ignored
        assert!(matches!(
            validator.validate("\"Text with { brackets } inside\""),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("'Text with ( parens ) inside'"),
            ValidationResult::Complete
        ));
    }

    #[test]
    fn test_backticks() {
        let validator = KotaValidator;

        // Balanced backticks should be complete
        assert!(matches!(
            validator.validate("```python\nprint('hello')\n```"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Use `code` inline"),
            ValidationResult::Complete
        ));

        // Unmatched backticks should be incomplete
        assert!(matches!(
            validator.validate("```python\nprint('hello')"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("Start ```\nsome code"),
            ValidationResult::Incomplete
        ));

        // Multiple code blocks
        assert!(matches!(
            validator.validate("```js\ncode1\n```\n\n```py\ncode2\n```"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("```js\ncode1\n```\n\n```py\ncode2"),
            ValidationResult::Incomplete
        ));
    }

    #[test]
    fn test_line_continuation() {
        let validator = KotaValidator;

        // Backslash at end should be incomplete
        assert!(matches!(
            validator.validate("This line continues \\"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("Multiple \\"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("   trailing spaces   \\   "),
            ValidationResult::Incomplete
        ));

        // Backslash not at end should be complete
        assert!(matches!(
            validator.validate("This \\ is not continuation"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Path: C:\\Windows\\System32"),
            ValidationResult::Complete
        ));
    }

    #[test]
    fn test_python_blocks() {
        let validator = KotaValidator;

        // Python-style blocks should be incomplete
        assert!(matches!(
            validator.validate("def function():"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("class MyClass:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("if condition:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("for item in items:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("while True:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("with open('file'):"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("try:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("except Exception:"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("finally:"),
            ValidationResult::Incomplete
        ));

        // Colon in other contexts should be complete
        assert!(matches!(
            validator.validate("Time: 12:30 PM"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("URL: https://example.com"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Note: this is important"),
            ValidationResult::Complete
        ));
    }

    #[test]
    fn test_complex_scenarios() {
        let validator = KotaValidator;

        // Real-world complex cases
        assert!(matches!(
            validator.validate("Create a function that handles { 'user': 'John', 'age': 30 }"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("JSON: { \"users\": [\"John\", \"Jane\"] }"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("Incomplete JSON: { \"users\": [\"John\""),
            ValidationResult::Incomplete
        ));

        // Mixed everything
        assert!(matches!(
            validator.validate(
                "```python\ndef process_data():\n    return {'result': \"success\"}\n```"
            ),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("```python\ndef process_data():\n    return {'result': \"success\""),
            ValidationResult::Incomplete
        ));

        // Edge case: empty input
        assert!(matches!(validator.validate(""), ValidationResult::Complete));
        assert!(matches!(
            validator.validate("   "),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("\n\n"),
            ValidationResult::Complete
        ));
    }

    #[test]
    fn test_whitespace_handling() {
        let validator = KotaValidator;

        // Whitespace around continuation
        assert!(matches!(
            validator.validate("text \\"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("text\\"),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("text \\ "),
            ValidationResult::Incomplete
        ));

        // Whitespace in brackets
        assert!(matches!(
            validator.validate("{ }"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("( )"),
            ValidationResult::Complete
        ));
        assert!(matches!(
            validator.validate("[ ]"),
            ValidationResult::Complete
        ));

        // Whitespace in Python blocks
        assert!(matches!(
            validator.validate("  def function():  "),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validator.validate("def function():"),
            ValidationResult::Incomplete
        ));
    }
}

pub fn read_single_char() -> Result<char> {
    // For simple single character input, we'll still use the crossterm approach
    // since reedline is overkill for y/n confirmations
    use termimad::crossterm::{
        event::{self, Event, KeyEvent},
        terminal::{disable_raw_mode, enable_raw_mode},
    };
    // Use the correct KeyCode and KeyModifiers from crossterm
    use termimad::crossterm::event::{
        KeyCode as CrosstermKeyCode, KeyModifiers as CrosstermKeyModifiers,
    };

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
