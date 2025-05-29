# KOTA Rust CLI

<p align="center">
  <img src="kota_image.png" alt="KOTA Logo" width="400">
</p>

KOTA is an advanced interactive Rust CLI that provides both a sophisticated Terminal User Interface (TUI) and a classic command-line interface for AI-assisted code editing and development tasks. It integrates with multiple LLM providers and features comprehensive vim-style navigation, intelligent file editing, command execution, and context management capabilities.

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

### ⚡ Rapid Adaptability
In the fast-evolving AI landscape, KOTA is designed to adapt at the speed of innovation. Through its self-modification capabilities and modular architecture, it can quickly integrate new models, adopt emerging tools, and evolve its capabilities as the AI ecosystem advances. This ensures KOTA remains cutting-edge without requiring complete rewrites or architectural overhauls.

### 🔮 Future Vision
KOTA represents an exploration of distributed cognition – where human and AI capabilities merge to create something more powerful than either alone. It's a step toward a future where AI partners can handle significant portions of complex work, allowing humans to focus on higher-level thinking and creativity.

## Features

### 🖥️ Advanced Terminal User Interface (TUI)
- **Multi-pane layout**: Chat history, terminal output, context view, and file browser
- **Vim-style navigation**: Full hjkl navigation with arrow key alternatives
- **Command execution**: Terminal pane displays suggested commands, execute with 'x' key
- **Interactive file browser**: Navigate directories, add files to context
- **Real-time updates**: Live display of time, git branch, file counts
- **Emoji-free design**: Clean text-based indicators for compatibility

### 🤖 AI-Powered Development
- **AI-Powered Code Editing**: Uses Search/Replace blocks to suggest and apply precise code changes
- **Command Suggestions**: LLMs suggest shell commands displayed in terminal pane for execution
- **Context Management**: Maintain conversation context by adding files and code snippets
- **Auto-Commit**: Automatically creates git commits with AI-generated commit messages
- **Multiple LLM Providers**: Works with both Google Gemini (cloud, default) and Ollama (local)

### 🔧 Developer Experience
- **Vim Bindings**: Full vim-style editing and navigation throughout the interface
- **Markdown Rendering**: Enhanced display of headers, code blocks, and formatting
- **File Safety**: Strict access control - can only edit files explicitly added to context
- **Checkbox Feedback**: Clear visual indicators for context operations (`Context: [x] filename`)
- **Zero Warnings**: Passes strictest Rust linting with 45+ comprehensive tests

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

### Start KOTA (TUI Mode - Default)
```bash
cargo run
```

The TUI provides an interactive multi-pane interface:

#### TUI Navigation
- **i** - Enter insert mode to type messages
- **Esc** - Return to normal mode
- **f** - Enter file browser mode
- **Tab** - Cycle through panes (Chat → Terminal → Context → File Browser)
- **hjkl / ↑↓←→** - Navigate and scroll within panes
- **x** - Execute suggested commands (when terminal pane focused)
- **?** - Show help and keyboard shortcuts
- **Ctrl+Q** - Quit application

#### TUI Workflow
1. **Browse files**: Press 'f' to open file browser, navigate with hjkl, Enter to add files
2. **Chat with AI**: Press 'i' to enter insert mode, type your message, press Enter
3. **Execute commands**: AI suggestions appear in terminal pane, press Tab to focus, 'x' to execute
4. **Review changes**: File edits are applied with confirmation, auto-commits created

### Available Commands (Both TUI and CLI)

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

### Example TUI Workflow

1. **Start KOTA**: `cargo run` (launches TUI by default)
2. **Add files to context**: Press 'f', navigate to files, press Enter to add
3. **Send message**: Press 'i', type "Add better error handling", press Enter
4. **Execute commands**: AI suggestions appear in terminal, press Tab to focus terminal, 'x' to execute
5. **Review changes**: File edits are applied automatically after confirmation

### Example CLI Workflow

```bash
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

### Core Components
- **Terminal User Interface**: Advanced multi-pane TUI with vim navigation
- **LLM Integration**: Supports Google Gemini (cloud) and Ollama (local)
- **Search/Replace Parser**: Parses structured file edit suggestions
- **Command Parser**: Parses and executes shell commands with confirmation
- **File Editor**: Handles interactive file modifications with safety checks
- **Context Manager**: Maintains conversation context with strict access control
- **File Browser**: Interactive navigation with sudo support

### Quality Standards
- **Zero Clippy Warnings**: Passes `cargo clippy -- -D warnings` with no issues
- **Comprehensive Testing**: 45+ tests covering all core functionality
- **Dead Code Elimination**: No unused code, methods, or dependencies
- **Memory Safety**: Safe async patterns with proper mutex handling
- **Error Handling**: Robust error handling with `anyhow` throughout

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code (strict mode)
cargo clippy -- -D warnings
```

## Configuration

### Google Gemini (Cloud) - Default
Set your API key as an environment variable:
```bash
export GEMINI_API_KEY=your_api_key_here
```

### Ollama (Local)
Ensure Ollama is running with your preferred model:
```bash
ollama serve
```

### Provider Switching
Switch providers in both TUI and CLI:
```bash
/provider gemini    # Use Google Gemini (default)
/provider ollama    # Use local Ollama
```

### Prompts Configuration
Customize KOTA's behavior by editing `prompts.toml`:
- System prompts and instructions
- Commit message generation templates
- Command execution guidelines
- Search/Replace format specifications

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Ensure `cargo clippy -- -D warnings` passes
6. Submit a pull request

## License

This project is open source. See LICENSE file for details.