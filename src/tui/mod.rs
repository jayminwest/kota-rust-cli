// Re-export main TUI components
pub mod app;
pub mod rendering;
pub mod types;
pub mod widgets;

#[cfg(test)]
mod tests;

pub use rendering::run_tui;