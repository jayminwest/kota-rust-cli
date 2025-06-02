# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⚠️ CRITICAL PRIVACY AND SECURITY NOTICE

**This is a PUBLIC repository that users will clone and use with their personal and sensitive data.**

### Data Privacy Requirements

KOTA users will inevitably work with:
- **Personal code projects** containing proprietary algorithms and business logic
- **API keys and credentials** for various services  
- **Private conversations** with AI models about sensitive topics
- **Personal knowledge bases** including career, financial, and private information
- **Company codebases** and confidential project details
- **Development secrets** and configuration data

### Mandatory Privacy Protections

When developing KOTA, you MUST:

1. **Never commit sensitive data**: All personal data, API keys, conversations, and user-specific content MUST remain local-only
2. **Verify .gitignore coverage**: Always ensure sensitive directories like `knowledge-base/`, `.kota/`, and session data are properly excluded
3. **Default to private**: Any new feature that stores user data must default to local storage with explicit gitignore protection
4. **Test privacy boundaries**: Before committing changes, verify no personal data could accidentally leak into the public repository
5. **Document data flows**: Clearly document what data is stored where and how privacy is maintained

### Protected Directories

These directories MUST NEVER be committed to git:
- `knowledge-base/` - Personal knowledge base with private conversations and learning
- `.kota/` - Configuration directory that may contain API keys and personal settings
- `sessions/` - Session data with conversation history
- `context/` - Temporary context files that may contain sensitive code
- `*.env` - Environment files with API keys and secrets

### Privacy-First Development

Remember: Every KOTA user trusts this tool with their most sensitive development work. A single privacy breach could expose:
- Trade secrets and proprietary code
- Personal API keys worth thousands of dollars
- Private conversations about career and business strategies
- Confidential client information and projects

**Privacy is not optional - it's fundamental to KOTA's trustworthiness as a cognitive partner.**

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
- **Autonomous Command Execution**: KOTA can automatically execute agent, security, and memory commands without user approval, while configuration commands require user permission.

#### 4. Robustness and Resilience
- To operate effectively, especially in autonomous modes, the tool must be robust. This includes sophisticated error handling (both internal and from external services like LLMs), retry mechanisms for transient issues, and clear diagnostics.
- Timeout configurations (240 seconds for Claude, 360 for Gemini, 120 for Ollama) prevent hanging on slow responses.

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
  - **Modular architecture**: Easy swapping of LLM providers (Claude/Anthropic, Gemini, Ollama) and addition of new ones
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
- `cargo run` - Build and run the CLI application (TUI mode by default)
- `cargo test` - Run all 63+ comprehensive tests
- `cargo check` - Fast compile check without producing executables

### Development Commands
- `cargo clippy -- -D warnings` - Lint the codebase with zero-tolerance for warnings
- `cargo fmt` - Format code according to Rust style guidelines

## Architecture

KOTA is an advanced interactive Rust CLI that integrates with multiple LLM providers for AI-assisted code editing. The application offers both a Terminal User Interface (TUI) mode and a classic CLI mode, with comprehensive vim-style navigation and rich features:

### Core Components

**LLM Integration** (`src/llm.rs`): Supports multiple LLM providers:
- Anthropic Claude API (cloud): Default provider and model claude-sonnet-4-20250514 (Claude 4 Sonnet)
- Google Gemini API (cloud): Alternative model gemini-2.5-pro-preview-05-06
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

**Context Manager** (`src/context.rs`): Maintains conversation context by storing file contents and code snippets that can be referenced in LLM conversations. Features checkbox-style feedback (`Context: [x] filename`) and strict file access control.

**Prompts Configuration** (`src/prompts.rs`): Manages all system prompts and instructions via TOML configuration file. Provides configurable LLM behavior, commit message generation templates, and instruction customization without code changes.

**Terminal User Interface** (`src/tui/`): Advanced modular TUI featuring:
- **Multi-pane layout**: Chat history, terminal output, context view, and file browser
- **Vim-style navigation**: Full hjkl navigation with arrow key alternatives
- **Auto-scrolling chat**: Automatic scrolling with manual override and 'a' key toggle
- **Enhanced command display**: Clear command suggestions with status indicators
- **Individual command execution**: Navigate commands with 'n'/'p', execute with 'x'
- **Multi-line input support**: Smart detection of code blocks, brackets, and continuations
- **Professional design**: Clean text-based indicators and strict content boundaries
- **Markdown rendering**: Enhanced display of headers, code blocks, and formatting
- **Real-time updates**: Live data display (time, git branch, file counts, scroll mode)
- **Responsive layout**: Automatic content scaling and truncation for stable display

**File Browser** (`src/file_browser.rs`): Interactive file navigation with sudo support, permissions display, and context integration.

**Dynamic Prompts** (`src/dynamic_prompts.rs`): Live system data injection including current time, working directory, git branch, and context information.

**Memory Manager** (`src/memory.rs`): Persistent knowledge base system that automatically captures and organizes conversation context:
- **Automatic storage**: Conversations saved with timestamps in structured knowledge-base/
- **Domain organization**: Content organized by topic areas (personal, projects, systems, etc.)
- **Privacy protection**: Local-only storage with .gitignore protection
- **Smart retrieval**: Commands for memory access (/memory, /search, /learn)
- **Conversation summarization**: Automatic context summarization and storage

**Multi-Agent Framework** (`src/agents/`): Complete agent system with working implementations:
- **AgentManager**: Central coordination for all agents with task delegation and communication
- **CodeAgent**: Specialized in code analysis, generation, and editing
- **PlanningAgent**: Expert in task breakdown, project planning, and resource allocation
- **ResearchAgent**: Focused on information gathering, web search, and data analysis
- **Autonomous execution**: Agents can execute commands without user approval
- **Shared context**: All agents access the same context and knowledge base

**Security Framework** (`src/security/`): Comprehensive macOS-focused security system:
- **Sandbox**: macOS Seatbelt sandboxing with multiple profiles (minimal, development, read-only)
- **Policy Engine**: Regex-based command filtering with customizable allow/deny rules
- **Approval System**: Interactive user prompts with risk assessment
- **Secure Executor**: Three-layer security (sandbox + policy + approval)
- **Configuration-driven**: TOML-based security policies and settings

**Configuration System** (`src/config/`): TOML-based configuration management:
- **Persistent settings**: Save and load configuration from files
- **Runtime customization**: Change behavior without recompilation
- **Security integration**: Configure approval modes and sandbox profiles
- **LLM settings**: Provider selection and model configuration

### Application Flow

#### TUI Mode (Default)
1. **Interactive Interface**: Multi-pane TUI with chat, terminal, context, and file browser
2. **Vim Navigation**: Use hjkl for scrolling, Tab/arrow keys for pane switching
3. **Message Input**: Enter insert mode ('i'), type message, send with Enter
4. **Command Suggestions**: LLM responses with command blocks show in terminal pane
5. **Command Execution**: Focus terminal ('Tab' to cycle), press 'x' to execute suggested commands
6. **File Operations**: Use file browser ('f' key) to add files to context
7. **Context Management**: Real-time context display with file tracking
8. **Visual Feedback**: Professional design with text indicators and content boundaries

#### Classic Mode
1. User enters commands (starting with `/`) or natural language prompts
2. For commands: Direct execution (file operations, git commands, shell commands)  
3. For prompts: Send to LLM with accumulated context + S/R + command execution instructions
4. Parse LLM response for S/R blocks and command blocks
5. Present S/R blocks for user confirmation and apply approved file changes
6. Present command blocks for user confirmation and execute approved commands

#### Both Modes Feature
- **Markdown Rendering**: Enhanced display of LLM responses with headers, code blocks, and formatting
- **Context Integration**: Automatic context awareness and file access control
- **Auto-commit**: Automatic git commits with LLM-generated messages after successful S/R applications
- **Provider Switching**: Easy switching between Claude, Gemini, and Ollama providers
- **Autonomous Commands**: AI can automatically execute agent, security, and memory commands

### Available Commands

#### TUI Commands
- **i** - Enter insert mode to type messages
- **Esc** - Return to normal mode
- **f** - Enter file browser mode
- **a** - Toggle auto-scroll mode in chat (AUTO/MANUAL indicator in status)
- **Tab** - Cycle through panes (Chat → Terminal → Context → File Browser)
- **hjkl / ↑↓←→** - Navigate and scroll within panes
- **n/p** - Navigate through command suggestions (when terminal focused)
- **x** - Execute selected command or all commands
- **?** - Show help and keyboard shortcuts
- **Ctrl+Q** - Quit application

#### CLI Commands (Both Modes)

**Context Management (Autonomous)**
- `/add_file <path>` - Add file contents to context
- `/add_snippet <text>` - Add text snippet to context  
- `/show_context` - Display current context
- `/clear_context` - Clear all context

**Command Execution (Require Approval)**
- `/run <command>` - Execute shell command
- `/run_add <command>` - Execute shell command and add output to context

**Git Operations (Require Approval)**
- `/git_add <file>` - Stage file for commit
- `/git_commit "<message>"` - Create git commit
- `/git_status` - Show git status
- `/git_diff [<path>]` - Show git diff

**Agent Commands (Autonomous)**
- `/agents` - List all available agents
- `/agent <name>` - Get details about specific agent
- `/delegate <task>` - Delegate task to best agent
- `/ask_agent <question>` - Ask question to agent

**Security Commands (Autonomous)**
- `/security [status]` - Show security system status
- `/sandbox [profile]` - Configure sandbox profiles
- `/approval [mode]` - Configure approval settings

**Configuration Commands (Require Approval)**
- `/provider <anthropic|gemini|ollama>` - Switch between LLM providers
- `/config [show|save|load|reset]` - Manage configuration
- `/help` - Show all available commands
- `/quit` - Exit application

#### Memory & Knowledge Base Commands (Autonomous)
- `/memory` - Show recent conversation summaries
- `/search <query>` - Search knowledge base for specific topics
- `/learn <topic>: <content>` - Add specific learning to knowledge base

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

### Prompts Configuration

KOTA uses a `prompts.toml` file to store all system prompts and instructions. This allows easy customization of how KOTA behaves without modifying code.

**Configuration File**: `prompts.toml` (in project root)

**Sections:**
- `[system]` - Main LLM instructions and behavior, including autonomous command access
- `[commit_generation]` - Commit message generation prompts for different providers
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

### Multi-Agent Architecture Foundation

KOTA includes a complete multi-agent system with working implementations:

**Agent System**: Structured agent management with specialized capabilities:
- **AgentManager**: Central coordination for task delegation and agent communication
- **CodeAgent**: Code analysis, generation, and editing capabilities
- **PlanningAgent**: Task breakdown, project planning, and resource allocation
- **ResearchAgent**: Information gathering, web search, and data analysis

**Agent Features**:
- **Autonomous Execution**: Agents can execute commands without user approval
- **Task Delegation**: Intelligent routing of tasks to the most suitable agent
- **Shared Context**: All agents access the same context and knowledge base
- **Concurrent Operations**: Multiple agents can work simultaneously
- **Extensible Design**: Easy to add new specialized agents

**Command Integration**: Agent commands are fully integrated into both TUI and CLI modes with autonomous execution capability.

### Knowledge Base & Memory

KOTA automatically builds and maintains a persistent knowledge base:

**Automatic Capture**: All conversations are automatically summarized and stored with timestamps

**Domain Organization**: Knowledge organized by subject areas following KOTA principles:
- `personal/` - Identity, career, finance, journaling
- `projects/` - Active and historical project documentation  
- `systems/` - Tools, workflows, and technical knowledge
- `core/` - Conversation management, partnerships, MCP integration

**Privacy Protection**: Local-only storage with .gitignore ensuring personal content stays private

**Smart Retrieval**: Natural language commands for accessing stored knowledge

**Context Integration**: Memory automatically informs current conversations with relevant past context

### Quality Standards

KOTA maintains the highest code quality standards with comprehensive testing and linting:

- **Zero Clippy Warnings**: Passes `cargo clippy -- -D warnings` with no issues
- **Comprehensive Testing**: 63+ tests covering all core functionality including TUI, agents, and security
- **Dead Code Elimination**: No unused code (framework modules use `#[allow(dead_code)]` for future features)
- **Memory Safety**: Safe async patterns with proper mutex handling
- **Error Handling**: Robust error handling with `anyhow` throughout
- **Professional Display**: Strict content boundaries prevent UI corruption
- **Privacy Protection**: All user data remains local-only with proper gitignore coverage
- **Security by Design**: Default configurations prioritize user privacy and data protection

### Dependencies
- `tokio` - Async runtime with process support
- `reqwest` - HTTP client for API calls
- `gemini-client-api` - Google Gemini API client
- `serde`/`serde_json` - JSON serialization for API requests
- `anyhow` - Comprehensive error handling
- `regex` - S/R block parsing
- `colored` - Terminal color output
- `termimad` - Markdown rendering in terminal
- `indicatif` - Progress bars and spinners for thinking indicators
- `reedline` - Advanced line editing with vim bindings
- `toml` - Configuration file parsing
- `ratatui` - Terminal user interface framework
- `crossterm` - Cross-platform terminal manipulation
- `chrono` - Date and time handling
- `unicode-width`, `textwrap` - Text formatting
- `hostname`, `whoami` - System information
- `tempfile` - Test utilities (dev dependency)

### Recent Architectural Changes

#### Default LLM Provider Update (January 2025)
- Changed default provider from Gemini to Anthropic Claude
- Updated configuration to use claude-sonnet-4-20250514 (Claude 4 Sonnet) as default model
- Maintained backward compatibility with Gemini and Ollama providers
- Added PartialEq derive to LlmProvider for testing support

#### Complete Framework Implementation (January 2025)
- **Multi-Agent System**: Fully implemented and integrated agent framework with CodeAgent, PlanningAgent, ResearchAgent, and AgentManager
- **Security Framework**: Complete macOS security system with sandboxing, policy engine, and approval system
- **Configuration System**: TOML-based configuration management with persistence
- **Autonomous Command Execution**: AI can automatically execute agent, security, and memory commands while requiring approval for configuration changes

#### TUI Display Improvements (January 2025)
- **Professional Layout**: Implemented strict content boundaries and responsive design
- **Aggressive Content Limiting**: Multiple layers of truncation to prevent text bleeding between panes
- **Layout Constraints**: Fixed pane widths with conservative space allocation
- **Content Scaling**: Automatic truncation and overflow indicators for stable display

#### Code Quality Improvements (January 2025)
- Achieved zero tolerance policy: no warnings or errors in compilation or tests
- Increased test coverage from 52 to 63+ tests
- Added `#[allow(dead_code)]` to framework modules for future features
- Fixed all clippy warnings and implemented CommandRegistry thread safety

#### TUI Modularization (December 2024)
- Broke down monolithic tui.rs into modular components
- Created tui/ subdirectory with app.rs, rendering.rs, types.rs, widgets.rs, and tests.rs
- Improved maintainability and testability of TUI code

## Important Instruction Reminders

**Zero Tolerance Policy**: KOTA maintains a zero-tolerance policy for ALL warnings and dead code. Every line of code must be used and clean. Framework modules use `#[allow(dead_code)]` for future features.

**File Safety**: Always prefer editing existing files to creating new ones. Never create documentation files unless explicitly requested.

**Quality Standards**: All changes must pass `cargo clippy -- -D warnings` and `cargo test` with zero issues.

**Architectural Integrity**: Maintain the modular architecture and ensure all new features integrate properly with existing systems.

**Self-Modification**: Remember that KOTA can modify its own code - use exit code 123 after self-modifications to trigger rebuild via `run_kota.sh`.