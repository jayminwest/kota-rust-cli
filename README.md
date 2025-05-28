# KOTA Rust CLI

<p align="center">
  <img src="kota_image.png" alt="KOTA Logo" width="400">
</p>

KOTA is an interactive Rust CLI that integrates with local Ollama LLM instances for AI-assisted code editing and development tasks. It operates as a REPL shell with intelligent file editing, command execution, and context management capabilities.

## Philosophy: A Cognitive Partner in Code

KOTA is designed to be more than just a command-line tool – it's a cognitive partner that augments your capabilities in software development and complex knowledge work. Built with the vision of becoming an increasingly autonomous AI agent, KOTA embodies several key principles:

### 🚀 Self-Improving System
KOTA has the unique ability to modify its own source code. When guided by users, it can iteratively enhance its capabilities, add new features, and evolve over time. This self-modification loop (supported by the `run_kota.sh` wrapper) represents a step toward truly adaptive software.

### 🧠 Deep Contextual Understanding
The tool excels at managing and utilizing context – from your project files to its own source code and interaction history. Through its context management system, KOTA maintains awareness of what it's working on and can make informed, relevant suggestions.

### 🤖 Intelligent Autonomy
While remaining user-directed, KOTA strives for increasing autonomy. It can break down complex tasks, execute multi-step operations, and leverage command outputs to make follow-up decisions – all with appropriate user oversight.

### 🛡️ Safety & Control
Despite its autonomous capabilities, KOTA prioritizes user control and safety:
- **File Access Control**: Can only edit files explicitly added to its context
- **Confirmation Required**: All file changes and commands require user approval
- **Transparent Operations**: Shows exactly what it's doing and why

### 🔮 Future Vision
KOTA represents an exploration of distributed cognition – where human and AI capabilities merge to create something more powerful than either alone. It's a step toward a future where AI partners can handle significant portions of complex work, allowing humans to focus on higher-level thinking and creativity.

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

### Self-Modification Workflow

KOTA can modify its own source code to add new features or improve itself:

```bash
# Use the wrapper script for automatic rebuild/restart
./run_kota.sh

# Inside KOTA, ask it to improve itself
You: Add a new command /version that shows the current version of KOTA

# KOTA will:
# 1. Request to read its own source: /add_file src/main.rs
# 2. Generate S/R blocks to add the feature
# 3. Apply changes and create a commit
# 4. Exit with code 123 to trigger rebuild

# The wrapper script automatically rebuilds and restarts KOTA with new features!
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