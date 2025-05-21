# KOTA Rust CLI

KOTA is an interactive Rust CLI that integrates with local Ollama LLM instances for AI-assisted code editing and development tasks. It operates as a REPL shell with intelligent file editing, command execution, and context management capabilities.

## Features

- **AI-Powered Code Editing**: Uses Search/Replace blocks to suggest and apply precise code changes
- **Command Execution**: LLMs can suggest and execute shell commands with user confirmation, and see their output for follow-up actions  
- **Context Management**: Maintain conversation context by adding files and code snippets
- **File Safety**: Warns when editing files not explicitly added to context
- **Auto-Commit**: Automatically creates git commits with AI-generated commit messages (uses Gemini Flash for fast generation)
- **Multiple LLM Providers**: Works with both Google Gemini (cloud, default) and Ollama (local)
  - Google Gemini: Cloud-based models (default: gemini-2.5-pro-preview-05-06)
  - Ollama: Local models (qwen3:8b)

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)

### For Google Gemini (Cloud LLM) - Default
- Google Gemini API key from [Google AI Studio](https://ai.google.dev/)
  ```bash
  export GEMINI_API_KEY=your_api_key_here
  ```

### For Ollama (Local LLM) - Alternative
- [Ollama](https://ollama.ai/) running locally with a compatible model
  ```bash
  brew install ollama
  ollama pull qwen3:8b
  ollama serve
  ```

## Installation

```bash
git clone <repository-url>
cd kota-rust-cli
cargo build --release
```

## Usage

Start KOTA:
```bash
cargo run
```

### Available Commands

- `/add_file <path>` - Add file contents to context
- `/add_snippet <text>` - Add text snippet to context  
- `/show_context` - Display current context
- `/clear_context` - Clear all context
- `/run <command>` - Execute shell command
- `/run_add <command>` - Execute shell command and add output to context
- `/git_add <file>` - Stage file for commit
- `/git_commit "<message>"` - Create git commit
- `/git_status` - Show git status
- `/git_diff [<path>]` - Show git diff
- `/provider <ollama|gemini>` - Switch between LLM providers
- `/help` - Show all available commands
- `/quit` - Exit application

### AI Interactions

Simply type natural language prompts to interact with the AI:

```
You: Add error handling to the main function
```

The AI can respond with:
1. **File edits** using Search/Replace blocks
2. **Commands** to build, test, or manage the project
3. **Explanations** and guidance

### Example Workflow

```bash
# Switch LLM provider if needed (Gemini is default)
/provider ollama  # or /provider gemini

# Add files to context
/add_file src/main.rs
/add_file Cargo.toml

# Ask AI to make changes
You: Add better error handling and logging

# AI suggests file changes and commands
KOTA: I'll add error handling and logging to your application.

src/main.rs
<<<<<<< SEARCH
fn main() {
    println!("Hello, world!");
}
=======
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("Starting application");
    
    println!("Hello, world!");
    
    Ok(())
}
>>>>>>> REPLACE

```bash
cargo add env_logger log
cargo build
```

# Review and approve changes
Apply this change? (y/n/a/q) [yes/no/apply_all/quit]: y
Execute this command? (y/n/q) [yes/no/quit]: y
```

## Architecture

- **LLM Integration**: Communicates with Ollama API for AI responses
- **Search/Replace Parser**: Parses structured file edit suggestions
- **Command Parser**: Parses and executes shell commands with confirmation
- **File Editor**: Handles interactive file modifications with safety checks
- **Context Manager**: Maintains conversation context and tracks added files

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Configuration

### Ollama (Local)
KOTA connects to Ollama at `http://localhost:11434/api/chat` by default. Ensure Ollama is running with your preferred model.

### Google Gemini (Cloud)
Set your API key as an environment variable:
```bash
export GEMINI_API_KEY=your_api_key_here
```

Switch providers in the CLI:
```bash
/provider gemini    # Use Google Gemini (default)
/provider ollama    # Use local Ollama
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is open source. See LICENSE file for details.