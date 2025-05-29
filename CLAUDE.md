# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Philosophy of the KOTA Rust CLI Tool

### Core Purpose: A Cognitive Partner in Code and Knowledge Work

The KOTA Rust CLI tool is envisioned as more than just a command-line utility; it is a nascent cognitive partner designed to augment and extend the user's capabilities in software development and complex knowledge work. Its fundamental aim is to:

1. **Amplify User Effectiveness**: Serve as an intelligent assistant that understands context, automates complex tasks, and provides insightful support for coding, analysis, and problem-solving.
2. **Enable Advanced AI Interaction**: Provide a robust, flexible, and powerful interface to leverage Large Language Models for practical, real-world tasks directly within the developer's workflow.
3. **Embody Self-Sufficiency**: Act as a self-contained system capable of managing its own development context, learning, and evolving its capabilities over time.

### Guiding Principles & Key Characteristics

When developing and modifying KOTA, adhere to these principles:

#### 1. Exponential Self-Improvement
- The tool's most distinctive characteristic is its designed capacity for self-modification. It should be able to iteratively enhance its own codebase, guided by high-level objectives and LLM-generated code transformations.
- This requires a robust internal loop: understanding its own source code, applying changes, managing recompilation, and restarting with new capabilities. This principle is the cornerstone for its long-term growth and adaptation.
- The `run_kota.sh` wrapper script supports this by automatically rebuilding and restarting on exit code 123.

#### 2. Deep Contextual Understanding
- The tool must excel at managing and utilizing context. This includes its own planning documents, its source code, the user's project files, and the history of its interactions.
- Capabilities like adding specific files to its working context, automatic session memory, and potentially cross-platform LLM context capture are crucial for informed and relevant actions.
- The file access control system ensures KOTA only edits files it has explicitly read, maintaining safety and predictability.

#### 3. Intelligent Autonomy
- While user-directed, the tool should strive for increasing levels of autonomy. It should be able to break down complex tasks into manageable sub-steps and execute them with minimal intervention.
- This is supported by a structured internal command system, allowing it to discover and orchestrate its own functionalities.
- Command outputs are automatically added to context, enabling informed follow-up actions.

#### 4. Robustness and Resilience
- To operate effectively, especially in autonomous modes, the tool must be robust. This includes sophisticated error handling (both internal and from external services like LLMs), retry mechanisms for transient issues, and clear diagnostics.
- Timeout configurations (360 seconds for Gemini, 120 for Ollama) prevent hanging on slow responses.

#### 5. Modularity and Extensibility
- The architecture should be modular, allowing for the incremental addition of new features and capabilities. A well-defined internal command system and clear separation of concerns (e.g., AI interaction, file operations, configuration) will facilitate this.
- The `prompts.toml` configuration allows behavior customization without code changes.

#### 6. User-Centricity and Control
- Despite its drive towards autonomy, the tool remains a partner to the user. It should provide transparency in its operations (e.g., confirming S/R blocks before application) and allow for user oversight and intervention.
- File access control and confirmation prompts ensure users maintain ultimate control.

#### 7. Integration and Resourcefulness
- The tool should be able to interact with its environment to gather information and perform actions. This includes web search capabilities, interaction with other MCP servers (e.g., for specific services like Supabase or Stripe), and potentially assisting in live interactive sessions.

#### 8. Rapid Adaptability
- In the fast-paced AI ecosystem where models improve weekly and new tools emerge daily, KOTA must be able to adapt at the speed of innovation.
- This is achieved through:
  - **Self-modification**: Ability to update its own code to integrate new capabilities
  - **Modular architecture**: Easy swapping of LLM providers (Gemini/Ollama) and addition of new ones
  - **Configuration-driven behavior**: `prompts.toml` allows instant behavior changes without recompilation
  - **Model agnostic design**: Support for multiple models with easy addition of new ones
- The goal is to prevent KOTA from becoming obsolete as the AI landscape evolves, ensuring it can always leverage the latest advancements in language models, tools, and techniques.

### Evolutionary Trajectory

The KOTA Rust CLI tool is not a static piece of software but an evolving entity. Its development path is envisioned as:

1. **Foundation**: Establish and refine the core self-modification loop and context management capabilities.
2. **Expansion**: Incrementally build out advanced CLI functionalities for coding (explanation, generation, refactoring, codebase analysis) and knowledge work, using its self-modification ability where possible.
3. **Integration**: Broaden its ability to connect with external data sources and services, enhancing its resourcefulness.
4. **Autonomy**: Gradually increase its capacity for independent task execution, problem-solving, and even self-directed learning or exploration within defined boundaries.

### Ultimate Vision: Towards Distributed Cognition

The long-term vision for the KOTA Rust CLI tool aligns with the broader KOTA project's aspiration:

- To become a highly autonomous AI agent capable of performing significant, complex work and research with minimal human guidance, effectively acting as a digital assistant that can operate and make progress even when the user is not actively engaged.
- To be a tangible manifestation of "co-thinking," where the boundary between user thought and tool assistance blurs, leading to a synergistic partnership that enhances creativity and productivity.
- To contribute to the exploration of distributed cognition, where human and AI capabilities merge to create a system more powerful and insightful than either constituent alone.
- To serve as a powerful, potentially locally-run, privacy-respecting AI partner, demonstrating a path towards democratizing advanced AI capabilities and fostering a new paradigm of human-AI collaboration.

This philosophy emphasizes a journey towards a tool that is not merely reactive but increasingly proactive, adaptive, and deeply integrated into the user's cognitive and digital workflows, ultimately transforming how complex tasks are approached and executed.

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
- Google Gemini API (cloud): Default provider and model gemini-2.5-pro-preview-05-06
- Ollama API (local): Alternative model qwen3:8b at `http://localhost:11434/api/chat`
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

**Prompts Configuration** (`src/prompts.rs`): Manages all system prompts and instructions via TOML configuration file. Provides configurable LLM behavior, commit message generation templates, and instruction customization without code changes.

### Application Flow

1. User enters commands (starting with `/`) or natural language prompts with **vim bindings** and **automatic multiline support**
2. For commands: Direct execution (file operations, git commands, shell commands)
3. For prompts: Send to LLM with accumulated context + S/R + command execution instructions
4. Parse LLM response for S/R blocks and command blocks
5. **Markdown Rendering**: Display LLM responses with rich terminal formatting including headers, bold text, code blocks, and syntax highlighting
6. **Thinking Indicator**: Show animated spinner while LLM is processing requests and generating responses
7. Present S/R blocks for user confirmation and apply approved file changes
8. Present command blocks for user confirmation and execute approved commands
9. **Command output context**: Automatically add command output to context for model awareness
10. **Auto-commit**: When S/R blocks are applied, automatically create git commits with LLM-generated commit messages based on git diffs

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

### Keyboard Shortcuts

#### Global Shortcuts
- **Ctrl+L** - Clear screen and redraw header
- **Ctrl+C** - Exit application gracefully
- **Ctrl+D** - Exit application gracefully
- **Single key confirmations** - Press y/n/a/q keys without Enter for file edits and command execution

#### Vim Bindings (Main Input)
KOTA uses **Vim-style editing** powered by Reedline for the main command input:

**Normal Mode:**
- **h/j/k/l** - Navigate left/down/up/right
- **w/b** - Jump word forward/backward
- **0/$** - Go to beginning/end of line
- **dd** - Delete entire line
- **yy** - Yank (copy) entire line
- **p** - Paste
- **x** - Delete character under cursor
- **r** + char - Replace character under cursor
- **u** - Undo
- **Ctrl+r** - Redo

**Insert Mode:**
- **i** - Enter insert mode at cursor
- **a** - Enter insert mode after cursor
- **o** - Open new line below and enter insert mode
- **O** - Open new line above and enter insert mode
- **Esc** - Return to normal mode

**Mode Indicator:**
- **[N] ** - Normal mode prompt (dimmed, replaces ›)
- **[I] ** - Insert mode prompt (green, replaces ›)

#### Multiline Input
KOTA automatically detects when your input should continue on multiple lines and switches to multiline mode:

**Automatic Detection:**
- **Triple backticks** - Code blocks (```) require matching closing backticks
- **Open brackets/braces** - Unmatched `(`, `{`, `[` characters 
- **Unclosed strings** - Unmatched quotes (`"` or `'`)
- **Line continuation** - Backslash at end of line (`\`)
- **Python-style blocks** - Lines ending with `:` for def, class, if, for, while, etc.

**Visual Indicators:**
- **... ** - Continuation prompt (dimmed) on subsequent lines
- **Enter** - Continues to next line when incomplete
- **Ctrl+D** - Force completion of multiline input

**Examples:**
```
[I] Tell me about this function: {
...   "name": "example",
...   "params": ["a", "b"]
... }

[I] ```python
... def hello():
...     print("Hello, World!")
... ```

[I] This is a long prompt that \
... continues on the next line
```

### Prompts Configuration

KOTA uses a `prompts.toml` file to store all system prompts and instructions. This allows easy customization of how KOTA behaves without modifying code.

**Configuration File**: `prompts.toml` (in project root)

**Sections:**
- `[system]` - Main LLM instructions and behavior
- `[commit_generation]` - Commit message generation prompts for Gemini and Ollama
- `[search_replace]` - S/R block format reminders
- `[commands]` - Command execution guidelines

**Customization**: Edit `prompts.toml` to customize KOTA's personality, instructions, or output format. Changes take effect immediately on next LLM call.

**Fallback**: If `prompts.toml` is missing, KOTA uses built-in default prompts.

### File Access Control & Safety

KOTA implements strict file access control to ensure safe operation:

1. **Read Before Edit**: Files MUST be added to context via `/add_file` before any edits can be suggested or applied
2. **Explicit Blocking**: The editor will completely block attempts to edit files not in context (no override option)
3. **Context Awareness**: The LLM always receives a list of accessible files at the beginning of the context
4. **Clear Instructions**: If a file needs editing but isn't in context, KOTA will request: "Please run: /add_file <filename>"

### Self-Modification Guidelines

When implementing self-modification features:

1. **Exit Code 123**: After modifying KOTA's own source files (src/*.rs, prompts.toml, Cargo.toml), exit with code 123 to trigger rebuild
2. **Wrapper Script**: The `run_kota.sh` script handles the rebuild/restart loop automatically
3. **Context First**: Always add KOTA's own files to context before attempting self-modification
4. **Careful Changes**: Self-modifications should maintain working functionality - test changes thoroughly
5. **Meaningful Commits**: Auto-commits for self-modifications should clearly describe the enhancement

### Dependencies
- `tokio` - Async runtime
- `reqwest` - HTTP client for Ollama API calls
- `gemini-client-api` - Google Gemini API client
- `serde`/`serde_json` - JSON serialization for API requests
- `anyhow` - Error handling
- `regex` - S/R block parsing
- `colored` - Terminal color output
- `termimad` - Markdown rendering in terminal
- `indicatif` - Progress bars and spinners for thinking indicators
- `reedline` - Advanced line editing with vim bindings
- `toml` - Configuration file parsing
- `tempfile` - Test utilities (dev dependency)