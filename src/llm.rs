use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use std::time::Duration;

// Structs for Ollama's /api/chat endpoint (non-streaming)
#[derive(Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    // Add other fields if needed like done, total_duration, etc.
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

const OLLAMA_API_URL: &str = "http://localhost:11434/api/chat";
const DEFAULT_MODEL: &str = "qwen3:8b";

pub async fn ask_model(user_prompt: &str, context_str: &str) -> anyhow::Result<String> {
    // Create a client with timeout settings
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(120))  // 2 minute timeout for the entire request
        .connect_timeout(Duration::from_secs(10))  // 10 second timeout for establishing connection
        .build()
        .context("Failed to create HTTP client")?;

    let mut messages = Vec::new();

    // Add S/R and command execution instructions as a system message
    let system_instructions = r#"
You are KOTA, a helpful coding assistant. You can suggest file edits and run commands.

For file edits, use this Search/Replace block format:

path/to/file.ext
<<<<<<< SEARCH
content to be searched and replaced
=======
new content to replace the searched content
>>>>>>> REPLACE

For commands, use code blocks with bash/sh/command:

```bash
ls -la
cd src
```

Rules for file edits:
- Use exact indentation and whitespace in the SEARCH block
- Only replace the first occurrence of the search content
- Multiple S/R blocks can be used in a single response
- File paths should be relative to the project root
- Always provide enough context in the SEARCH block to uniquely identify the location

Rules for commands:
- Use ```bash, ```sh, or ```command for command blocks
- Commands in a single block will be chained with &&
- The user will be prompted before each command execution
- Use commands for tasks like building, testing, file operations, etc.

Example file edit:
src/main.rs
<<<<<<< SEARCH
fn old_function() {
    println!("Hello, old world!");
}
=======
fn new_function() {
    println!("Hello, new world!");
}
>>>>>>> REPLACE

Example commands:
```bash
cargo build
cargo test
```
"#;

    messages.push(OllamaChatMessage {
        role: "system".to_string(),
        content: system_instructions.to_string(),
    });

    // Add context as a system message if it's not empty
    if !context_str.is_empty() {
        messages.push(OllamaChatMessage {
            role: "system".to_string(),
            content: context_str.to_string(),
        });
    }

    // Add the user's prompt
    messages.push(OllamaChatMessage {
        role: "user".to_string(),
        content: user_prompt.to_string(),
    });

    let request_payload = OllamaChatRequest {
        model: DEFAULT_MODEL.to_string(),
        messages,
        stream: false,
    };

    let response = client
        .post(OLLAMA_API_URL)
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| {
            // Provide more specific error messages for common connection issues
            if e.is_connect() {
                anyhow::anyhow!("Failed to connect to Ollama API. Please check if Ollama is running (brew services start ollama)")
            } else if e.is_timeout() {
                anyhow::anyhow!("Request to Ollama API timed out. The model might be too large or the server is under heavy load")
            } else {
                anyhow::anyhow!("Failed to send request to Ollama API: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        
        // Add more context to specific error codes
        let error_msg = match status.as_u16() {
            404 => format!("Model not found. Status {}: {}", status, error_text),
            400 => format!("Bad request to Ollama API. Status {}: {}", status, error_text),
            412 => format!("Ollama version incompatibility. Please update Ollama (brew upgrade ollama). Status {}: {}", status, error_text),
            500 => format!("Ollama server error. Status {}: {}", status, error_text),
            _ => format!("Ollama API request failed with status {}: {}", status, error_text),
        };
        
        return Err(anyhow::anyhow!(error_msg));
    }

    let ollama_response = response
        .json::<OllamaChatResponse>()
        .await
        .context("failed to parse JSON response from Ollama API")?;

    Ok(ollama_response.message.content)
}

pub async fn generate_commit_message(original_prompt: &str, git_diff: &str) -> anyhow::Result<String> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(60))  // 1 minute timeout for commit message generation
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let commit_instructions = r#"
You are a git commit message generator. Generate a concise, descriptive commit message based on the user's original request and the git diff showing what actually changed.

Rules:
- Use conventional commit format: type(scope): description
- Keep it under 72 characters
- Use present tense ("add" not "added")
- Common types: feat, fix, refactor, docs, style, test, chore
- Be specific but concise
- Focus on the semantic change, not the implementation details
- Don't mention "LLM" or "AI" in the message

Examples:
- feat(auth): add user login validation
- fix(parser): handle edge case in S/R blocks
- refactor(api): simplify error handling logic
- docs(readme): update installation instructions
"#;

    let prompt = format!(
        "Original user request: \"{}\"\n\nGit diff of changes:\n{}\n\nGenerate only the commit message, nothing else:",
        original_prompt, git_diff
    );

    let messages = vec![
        OllamaChatMessage {
            role: "system".to_string(),
            content: commit_instructions.to_string(),
        },
        OllamaChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let request_payload = OllamaChatRequest {
        model: DEFAULT_MODEL.to_string(),
        messages,
        stream: false,
    };

    let response = client
        .post(OLLAMA_API_URL)
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                anyhow::anyhow!("Failed to connect to Ollama API for commit message generation")
            } else if e.is_timeout() {
                anyhow::anyhow!("Commit message generation timed out")
            } else {
                anyhow::anyhow!("Failed to generate commit message: {}", e)
            }
        })?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to generate commit message: HTTP {}", response.status()));
    }

    let ollama_response = response
        .json::<OllamaChatResponse>()
        .await
        .context("Failed to parse commit message response")?;

    // Clean up the response (remove any extra whitespace/newlines)
    let commit_message = ollama_response.message.content.trim().to_string();
    
    Ok(commit_message)
}
