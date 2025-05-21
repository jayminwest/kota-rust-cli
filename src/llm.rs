use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use std::time::Duration;
use gemini_client_api::gemini::{
    ask::Gemini,
    types::sessions::Session,
};
use crate::prompts::PromptsConfig;

#[derive(Debug, Clone)]
pub enum LlmProvider {
    Ollama,
    Gemini,
}

impl Default for LlmProvider {
    fn default() -> Self {
        LlmProvider::Gemini
    }
}

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
const DEFAULT_OLLAMA_MODEL: &str = "qwen3:8b";
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-pro-preview-05-06";
const GEMINI_COMMIT_MODEL: &str = "gemini-2.5-flash-preview-05-20";

pub async fn ask_model(user_prompt: &str, context_str: &str) -> anyhow::Result<String> {
    ask_model_with_provider(user_prompt, context_str, LlmProvider::default()).await
}

pub async fn ask_model_with_provider(user_prompt: &str, context_str: &str, provider: LlmProvider) -> anyhow::Result<String> {
    let prompts_config = PromptsConfig::load().unwrap_or_default();
    match provider {
        LlmProvider::Ollama => ask_ollama_model(user_prompt, context_str, &prompts_config).await,
        LlmProvider::Gemini => ask_gemini_model(user_prompt, context_str, &prompts_config).await,
    }
}

async fn ask_gemini_model(user_prompt: &str, context_str: &str, prompts_config: &PromptsConfig) -> anyhow::Result<String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable not found. Please set it to use Gemini."))?;
    
    let ai = Gemini::new(api_key, DEFAULT_GEMINI_MODEL, None);
    let mut session = Session::new(10); // Keep last 10 messages for context
    
    // Prepare the full prompt with system instructions and context
    let system_instructions = prompts_config.get_system_instructions();
    let full_prompt = if context_str.is_empty() {
        format!("{}\n\nUser: {}", system_instructions, user_prompt)
    } else {
        format!("{}\n\n{}\n\nUser: {}", system_instructions, context_str, user_prompt)
    };
    
    let response = ai.ask(session.ask_string(&full_prompt)).await
        .map_err(|e| anyhow::anyhow!("Gemini API error: {}", e))?;
    
    Ok(response.get_text(""))
}

async fn ask_ollama_model(user_prompt: &str, context_str: &str, prompts_config: &PromptsConfig) -> anyhow::Result<String> {
    // Create a client with timeout settings
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(120))  // 2 minute timeout for the entire request
        .connect_timeout(Duration::from_secs(10))  // 10 second timeout for establishing connection
        .build()
        .context("Failed to create HTTP client")?;

    let mut messages = Vec::new();

    // Add S/R and command execution instructions as a system message
    let system_instructions = prompts_config.get_system_instructions();

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
        model: DEFAULT_OLLAMA_MODEL.to_string(),
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
    let prompts_config = PromptsConfig::load().unwrap_or_default();
    
    // Try Gemini first, fallback to Ollama if API key not available
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        match generate_commit_message_gemini(original_prompt, git_diff, &api_key, &prompts_config).await {
            Ok(message) => return Ok(message),
            Err(e) => {
                eprintln!("Warning: Gemini commit generation failed: {}. Falling back to Ollama...", e);
            }
        }
    }
    
    // Fallback to Ollama
    generate_commit_message_ollama(original_prompt, git_diff, &prompts_config).await
}

async fn generate_commit_message_gemini(original_prompt: &str, git_diff: &str, api_key: &str, prompts_config: &PromptsConfig) -> anyhow::Result<String> {
    let ai = Gemini::new(api_key.to_string(), GEMINI_COMMIT_MODEL, None);
    let mut session = Session::new(2); // Simple session for commit messages
    
    let prompt = prompts_config.get_gemini_commit_prompt(original_prompt, git_diff);
    
    let response = ai.ask(session.ask_string(&prompt)).await
        .map_err(|e| anyhow::anyhow!("Gemini commit generation error: {}", e))?;
    
    // Clean up the response (remove any extra whitespace/newlines)
    let commit_message = response.get_text("").trim().to_string();
    
    Ok(commit_message)
}

async fn generate_commit_message_ollama(original_prompt: &str, git_diff: &str, prompts_config: &PromptsConfig) -> anyhow::Result<String> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(60))  // 1 minute timeout for commit message generation
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let prompt = prompts_config.get_ollama_commit_prompt(original_prompt, git_diff);

    let messages = vec![
        OllamaChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let request_payload = OllamaChatRequest {
        model: DEFAULT_OLLAMA_MODEL.to_string(),
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
