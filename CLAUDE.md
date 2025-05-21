# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Building and Running
- `cargo build` - Compile the project
- `cargo run` - Build and run the CLI application
- `cargo test` - Run all tests
- `cargo check` - Fast compile check without producing executables

### Development Commands
- `cargo clippy` - Lint the codebase for common issues
- `cargo fmt` - Format code according to Rust style guidelines

## Architecture

KOTA is an interactive Rust CLI that integrates with local Ollama LLM instances for AI-assisted code editing. The application operates as a REPL shell with several key architectural components:

### Core Components

**LLM Integration** (`src/llm.rs`): Supports multiple LLM providers:
- Ollama API (local): Default model qwen3:8b at `http://localhost:11434/api/chat`
- Google Gemini API (cloud): Default model gemini-2.5-pro-preview-05-06
Uses non-streaming APIs with proper error handling for connection issues and timeouts. Includes functionality to generate conventional commit messages using Gemini Flash (gemini-2.5-flash-preview-05-20) with fallback to Ollama, based on git diffs and user prompts.

**Search/Replace Parser** (`src/sr_parser.rs`): Parses structured S/R blocks from LLM responses using regex-based parsing. Expected format:
```
file/path
<<<<<<< SEARCH
content to replace
=======
replacement content
>>>>>>> REPLACE
```

**Command Parser** (`src/cmd_parser.rs`): Parses command blocks from LLM responses and executes them with user confirmation. Command output is automatically added to context so the model can see results and make follow-up decisions. Expected format:
```bash
command1
command2
```

**File Editor** (`src/editor.rs`): Handles interactive confirmation and application of parsed S/R blocks. Provides user prompts for each file change with options to apply individually, apply all, or quit. After successful application, automatically creates git commits with LLM-generated commit messages. Warns when trying to edit files not added to context.

**Context Manager** (`src/context.rs`): Maintains conversation context by storing file contents and code snippets that can be referenced in LLM conversations.

### Application Flow

1. User enters commands (starting with `/`) or natural language prompts
2. For commands: Direct execution (file operations, git commands, shell commands)
3. For prompts: Send to LLM with accumulated context + S/R + command execution instructions
4. Parse LLM response for S/R blocks and command blocks
5. Present S/R blocks for user confirmation and apply approved file changes
6. Present command blocks for user confirmation and execute approved commands
7. **Command output context**: Automatically add command output to context for model awareness
8. **Auto-commit**: When S/R blocks are applied, automatically create git commits with LLM-generated commit messages based on git diffs

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
- `/quit` - Exit application

### Dependencies
- `tokio` - Async runtime
- `reqwest` - HTTP client for Ollama API calls
- `gemini-client-api` - Google Gemini API client
- `serde`/`serde_json` - JSON serialization for API requests
- `anyhow` - Error handling
- `regex` - S/R block parsing
- `tempfile` - Test utilities (dev dependency)