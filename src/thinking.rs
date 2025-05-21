use indicatif::{ProgressBar, ProgressStyle};
use colored::*;
use std::time::Duration;

pub struct ThinkingIndicator {
    spinner: ProgressBar,
}

impl ThinkingIndicator {
    pub fn new(message: &str) -> Self {
        let spinner = ProgressBar::new_spinner();
        
        // Set up the spinner style with custom characters and colors
        let style = ProgressStyle::with_template("{spinner:.bright_cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
            ]);
        
        spinner.set_style(style);
        spinner.set_message(message.dimmed().to_string());
        
        // Set the spinner to tick every 80ms for smooth animation
        spinner.enable_steady_tick(Duration::from_millis(80));
        
        Self { spinner }
    }
    
    pub fn update_message(&self, message: &str) {
        self.spinner.set_message(message.dimmed().to_string());
    }
    
    pub fn finish(&self) {
        self.spinner.finish_and_clear();
    }
    
    pub fn finish_with_message(&self, message: &str) {
        self.spinner.finish_with_message(message.to_string());
    }
}

impl Drop for ThinkingIndicator {
    fn drop(&mut self) {
        self.spinner.finish_and_clear();
    }
}

// Convenience functions for common use cases
pub fn show_thinking(message: &str) -> ThinkingIndicator {
    ThinkingIndicator::new(message)
}

pub fn show_llm_thinking() -> ThinkingIndicator {
    ThinkingIndicator::new("Thinking...")
}

pub fn show_generating_commit() -> ThinkingIndicator {
    ThinkingIndicator::new("Generating commit message...")
}