use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemConfig {
    pub instructions: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommitGenerationConfig {
    pub gemini_prompt: String,
    pub ollama_prompt: String,
    pub anthropic_prompt: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchReplaceConfig {
    pub format_reminder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandsConfig {
    pub safety_note: String,
    pub execution_reminder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PromptsConfig {
    pub system: SystemConfig,
    pub commit_generation: CommitGenerationConfig,
    pub search_replace: SearchReplaceConfig,
    pub commands: CommandsConfig,
}

impl PromptsConfig {
    pub fn load() -> Result<Self> {
        // Try to load from current directory first, then from executable directory
        let config_paths = [
            "prompts.toml",
            "./prompts.toml",
            "../prompts.toml", // In case running from target/debug
        ];

        for path in &config_paths {
            if let Ok(content) = fs::read_to_string(path) {
                return toml::from_str(&content)
                    .with_context(|| format!("Failed to parse prompts.toml from {}", path));
            }
        }

        // If no config file found, return default configuration
        Ok(Self::default())
    }

    pub fn get_system_instructions(&self) -> &str {
        &self.system.instructions
    }

    pub fn get_gemini_commit_prompt(&self, original_prompt: &str, git_diff: &str) -> String {
        self.commit_generation
            .gemini_prompt
            .replace("{original_prompt}", original_prompt)
            .replace("{git_diff}", git_diff)
    }

    pub fn get_ollama_commit_prompt(&self, original_prompt: &str, git_diff: &str) -> String {
        self.commit_generation
            .ollama_prompt
            .replace("{original_prompt}", original_prompt)
            .replace("{git_diff}", git_diff)
    }

    pub fn get_anthropic_commit_prompt(&self, original_prompt: &str, git_diff: &str) -> String {
        self.commit_generation
            .anthropic_prompt
            .replace("{original_prompt}", original_prompt)
            .replace("{git_diff}", git_diff)
    }
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig {
                instructions: r#"You are KOTA, a helpful coding assistant. You help users with coding tasks by providing search/replace blocks for file edits and command blocks for terminal commands.

When the user asks you to edit files or run commands, you can use two special formats:

## Search/Replace Block Format
```
filename.ext
<<<<<<< SEARCH
exact content to find
=======
new content to replace it with
>>>>>>> REPLACE
```

## Command Block Format
```bash
command to run
```

## Rules:
1. For file edits: Only edit files that are in the user's context. Use EXACT text in SEARCH blocks.
2. For commands: Suggest commands that help accomplish the user's goals. They will be executed with user confirmation.
3. Always explain what you're doing and why.
4. Be concise but thorough in your explanations.
5. If you're not sure about something, ask for clarification.

## Examples:

### File Edit Example:
src/main.rs
<<<<<<< SEARCH
fn main() {
    println!("Hello, world!");
}
=======
fn main() {
    println!("Hello, KOTA!");
}
>>>>>>> REPLACE

### Command Example:
```bash
cargo test
```

Remember: Search blocks must match EXACTLY, and commands will be confirmed before execution."#.to_string(),
            },
            commit_generation: CommitGenerationConfig {
                gemini_prompt: r#"Please generate a concise commit message for the following changes:

Original user request: {original_prompt}

Git diff:
{git_diff}

Requirements:
- Use conventional commit format (type: description)
- Keep under 72 characters
- Use present tense
- Be specific about what changed
- Choose appropriate type: feat, fix, docs, style, refactor, test, chore

Examples:
- feat: add user authentication system
- fix: resolve memory leak in parser
- docs: update installation instructions
- refactor: simplify database connection logic

Return only the commit message, no explanation."#.to_string(),
                ollama_prompt: r#"Generate a commit message for these changes:

User request: {original_prompt}

Changes:
{git_diff}

Format: type: brief description
- Use feat/fix/docs/style/refactor/test/chore
- Under 72 characters
- Present tense
- Be specific

Examples:
feat: add user login
fix: memory leak in parser
docs: update README

Commit message:"#.to_string(),
                anthropic_prompt: r#"Generate a conventional commit message for the following changes.

Original user request: {original_prompt}

Git diff:
{git_diff}

Requirements:
- Use conventional commit format (type: description)
- Keep under 72 characters
- Use present tense
- Be specific and clear about what changed
- Choose appropriate type: feat, fix, docs, style, refactor, test, chore

Examples:
- feat: add user authentication system
- fix: resolve memory leak in parser
- docs: update installation instructions
- refactor: simplify database connection logic

Return only the commit message, nothing else."#.to_string(),
            },
            search_replace: SearchReplaceConfig {
                format_reminder: r#"Remember: Search/Replace blocks must use this exact format:

filename.ext
<<<<<<< SEARCH
exact content to find
=======
new content to replace it with
>>>>>>> REPLACE

The SEARCH content must match the file EXACTLY (including whitespace)."#.to_string(),
            },
            commands: CommandsConfig {
                safety_note: "Commands will be presented to the user for confirmation before execution. Suggest helpful commands that accomplish the user's goals.".to_string(),
                execution_reminder: "Remember: Commands are executed with user confirmation and their output is added to the conversation context for follow-up actions.".to_string(),
            },
        }
    }
}
