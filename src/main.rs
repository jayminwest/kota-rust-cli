use std::env;
use termimad::MadSkin;

mod llm;
mod context;
mod sr_parser;
mod editor;
mod cmd_parser;
mod input;
mod thinking;
mod prompts;
mod tui;
mod dynamic_prompts;
mod file_browser;
mod memory;
mod commands;
mod cli;
mod agents;
mod security;
mod config;

use context::ContextManager;
use llm::ModelConfig;

fn render_markdown(content: &str) -> anyhow::Result<()> {
    // Create a markdown renderer with customized skin
    let mut skin = MadSkin::default();
    
    // Set consistent spacing and wrapping
    skin.paragraph.align = termimad::Alignment::Left;
    
    // Import the correct Color type from crossterm
    use termimad::crossterm::style::Color;
    use termimad::crossterm::terminal;
    
    // Get terminal dimensions
    let (width, _height) = terminal::size().unwrap_or((80, 24));
    // Ensure minimum width for proper rendering and add padding
    let width = width.saturating_sub(4).max(40); // Subtract 4 for terminal padding
    
    // Customize colors to match the existing UI theme using termimad's color functions
    skin.bold.set_fg(Color::White);
    skin.italic.set_fg(Color::AnsiValue(248)); // Light gray
    skin.strikeout.set_fg(Color::AnsiValue(244)); // Dimmed gray
    
    // Style headers with bright blue colors
    skin.headers[0].set_fg(Color::Rgb{r: 100, g: 200, b: 255}); // Bright blue for h1
    skin.headers[1].set_fg(Color::Rgb{r: 120, g: 200, b: 255}); // Slightly dimmer blue for h2
    skin.headers[2].set_fg(Color::Rgb{r: 140, g: 200, b: 255}); // Even dimmer for h3
    
    // Style code blocks and inline code
    skin.code_block.set_bg(Color::AnsiValue(235)); // Dark gray background
    skin.code_block.set_fg(Color::AnsiValue(252)); // Light gray text
    skin.inline_code.set_bg(Color::AnsiValue(237)); // Slightly lighter dark gray
    skin.inline_code.set_fg(Color::AnsiValue(252)); // Light gray text
    
    // Style lists with better spacing
    skin.bullet.set_fg(Color::Cyan);
    skin.paragraph.align = termimad::Alignment::Left;
    
    
    // Style quotes
    skin.quote_mark.set_fg(Color::AnsiValue(244)); // Dimmed gray
    
    // Ensure consistent paragraph formatting with no extra margins
    skin.paragraph.left_margin = 0;
    skin.paragraph.right_margin = 0;
    
    // Print the markdown content with proper formatting using dynamic width
    // The text method properly handles width constraints
    let formatted = skin.text(content, Some(width as usize));
    print!("{}", formatted);
    
    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let use_tui = args.contains(&"--tui".to_string()) || args.contains(&"-t".to_string());
    
    // Show help if requested
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("KOTA - AI Coding Assistant");
        println!();
        println!("Usage: {} [OPTIONS]", args[0]);
        println!();
        println!("Options:");
        println!("  -t, --tui       Launch with modern TUI interface");
        println!("  -h, --help      Show this help message");
        println!("  -v, --version   Show version information");
        println!();
        println!("Default: Launch in classic CLI mode");
        return Ok(());
    }
    
    // Show version if requested
    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("KOTA version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    
    let context_manager = ContextManager::new();
    let model_config = ModelConfig::default();
    
    // Launch appropriate interface
    if use_tui {
        // Launch modern TUI
        tui::run_tui(context_manager, model_config).await
    } else {
        // Launch classic CLI
        cli::run_classic_cli(context_manager, model_config).await
    }
}