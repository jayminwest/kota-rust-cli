use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use std::time::Duration;
use gemini_client_api::gemini::{
    ask::Gemini,
    types::sessions::Session,
};
use crate::prompts::PromptsConfig;
use tokio::time::timeout;

#[derive(Debug, Clone)]
#[derive(Default)]
pub enum LlmProvider {
    Ollama,
    Gemini,
    #[default]
    Anthropic,
}

#[derive(Debug, Clone, Default)]
pub struct ModelConfig {
    pub provider: LlmProvider,
    pub model_name: Option<String>,
}

impl ModelConfig {

    pub fn get_model_name(&self) -> String {
        match &self.model_name {
            Some(name) => name.clone(),
            None => match self.provider {
                LlmProvider::Ollama => DEFAULT_OLLAMA_MODEL.to_string(),
                LlmProvider::Gemini => DEFAULT_GEMINI_MODEL.to_string(),
                LlmProvider::Anthropic => DEFAULT_ANTHROPIC_MODEL.to_string(),
            }
        }
    }

    pub fn display_name(&self) -> String {
        let model = self.get_model_name();
        match self.provider {
            LlmProvider::Ollama => format!("Ollama/{}", model),
            LlmProvider::Gemini => format!("Gemini/{}", model),
            LlmProvider::Anthropic => format!("Claude/{}", model),
        }
    }
}


// Structs for Ollama's /api/chat endpoint (non-streaming)
#[derive(Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

// Structs for Anthropic's messages API
#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
    #[serde(rename = "type")]
    content_type: String,
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
    content: String,
}

const OLLAMA_API_URL: &str = "http://localhost:11434/api/chat";
const DEFAULT_OLLAMA_MODEL: &str = "qwen3:8b";
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-pro-preview-05-06";
const GEMINI_COMMIT_MODEL: &str = "gemini-2.5-flash-preview-05-20";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-20250514";
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

// Timeout configuration
// Ollama: 120 seconds for main requests, 60 seconds for commits
// Gemini: 360 seconds for main requests (3x Ollama), 180 seconds for commits
// Anthropic: 240 seconds for main requests (2x Ollama), 120 seconds for commits
const GEMINI_TIMEOUT_SECS: u64 = 360;
const ANTHROPIC_TIMEOUT_SECS: u64 = 240;



pub async fn ask_model_with_config(user_prompt: &str, context_str: &str, config: &ModelConfig) -> anyhow::Result<String> {
    let prompts_config = PromptsConfig::load().unwrap_or_default();
    let model_name = config.get_model_name();
    
    match config.provider {
        LlmProvider::Ollama => ask_ollama_model(user_prompt, context_str, &prompts_config, &model_name).await,
        LlmProvider::Gemini => ask_gemini_model(user_prompt, context_str, &prompts_config, &model_name).await,
        LlmProvider::Anthropic => ask_anthropic_model(user_prompt, context_str, &prompts_config, &model_name).await,
    }
}

async fn ask_gemini_model(user_prompt: &str, context_str: &str, prompts_config: &PromptsConfig, model_name: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable not found. Please set it to use Gemini."))?;
    
    let ai = Gemini::new(api_key, model_name, None);
    let mut session = Session::new(10); // Keep last 10 messages for context
    
    // Prepare the full prompt with system instructions and context
    let system_instructions = prompts_config.get_system_instructions();
    let full_prompt = if context_str.is_empty() {
        format!("{}\n\nUser: {}", system_instructions, user_prompt)
    } else {
        format!("{}\n\n{}\n\nUser: {}", system_instructions, context_str, user_prompt)
    };
    
    // Wrap the API call with a timeout
    let response = timeout(
        Duration::from_secs(GEMINI_TIMEOUT_SECS),
        ai.ask(session.ask_string(&full_prompt))
    )
    .await
    .map_err(|_| anyhow::anyhow!("Gemini API request timed out after {} seconds", GEMINI_TIMEOUT_SECS))?
    .map_err(|e| anyhow::anyhow!("Gemini API error: {}", e))?;
    
    Ok(response.get_text(""))
}

async fn ask_anthropic_model(user_prompt: &str, context_str: &str, prompts_config: &PromptsConfig, model_name: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable not found. Please set it to use Anthropic Claude."))?;
    
    // Create a client with timeout settings
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(ANTHROPIC_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;
    
    let mut messages = Vec::new();
    
    // Add system message with instructions and context
    let system_instructions = prompts_config.get_system_instructions();
    let system_content = if context_str.is_empty() {
        system_instructions.to_string()
    } else {
        format!("{}\n\n{}", system_instructions, context_str)
    };
    
    // For Anthropic, we need to structure messages differently
    // The system prompt goes in the system parameter of the API call
    messages.push(AnthropicMessage {
        role: "user".to_string(),
        content: user_prompt.to_string(),
    });
    
    // Note: We're using serde_json::json! here because Anthropic API requires
    // the "system" field which is not part of our AnthropicRequest struct
    let request_payload = serde_json::json!({
        "model": model_name,
        "messages": messages,
        "max_tokens": 4096,
        "system": system_content,
    });
    
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                anyhow::anyhow!("Failed to connect to Anthropic API. Please check your internet connection.")
            } else if e.is_timeout() {
                anyhow::anyhow!("Request to Anthropic API timed out after {} seconds", ANTHROPIC_TIMEOUT_SECS)
            } else {
                anyhow::anyhow!("Failed to send request to Anthropic API: {}", e)
            }
        })?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        
        let error_msg = match status.as_u16() {
            401 => format!("Authentication failed. Please check your ANTHROPIC_API_KEY. Status {}: {}", status, error_text),
            403 => format!("Access forbidden. Your API key may not have access to this model. Status {}: {}", status, error_text),
            404 => format!("Model not found. Status {}: {}", status, error_text),
            429 => format!("Rate limit exceeded. Please wait before trying again. Status {}: {}", status, error_text),
            500 => format!("Anthropic server error. Status {}: {}", status, error_text),
            _ => format!("Anthropic API request failed with status {}: {}", status, error_text),
        };
        
        return Err(anyhow::anyhow!(error_msg));
    }
    
    let anthropic_response: AnthropicResponse = response
        .json()
        .await
        .context("Failed to parse JSON response from Anthropic API")?;
    
    // Extract text from the first content block
    let text = anthropic_response
        .content
        .into_iter()
        .find(|c| c.content_type == "text")
        .map(|c| c.text)
        .unwrap_or_else(|| "No text response from Anthropic".to_string());
    
    Ok(text)
}

async fn ask_ollama_model(user_prompt: &str, context_str: &str, prompts_config: &PromptsConfig, model_name: &str) -> anyhow::Result<String> {
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
        model: model_name.to_string(),
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
    
    // Try Anthropic first if API key is available
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        match generate_commit_message_anthropic(original_prompt, git_diff, &api_key, &prompts_config).await {
            Ok(message) => return Ok(message),
            Err(e) => {
                eprintln!("Warning: Anthropic commit generation failed: {}. Trying other providers...", e);
            }
        }
    }
    
    // Try Gemini next, fallback to Ollama if API key not available
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
    
    // Wrap the API call with a timeout (use half the main timeout for commit messages)
    let response = timeout(
        Duration::from_secs(GEMINI_TIMEOUT_SECS / 2),
        ai.ask(session.ask_string(&prompt))
    )
    .await
    .map_err(|_| anyhow::anyhow!("Gemini commit generation timed out after {} seconds", GEMINI_TIMEOUT_SECS / 2))?
    .map_err(|e| anyhow::anyhow!("Gemini commit generation error: {}", e))?;
    
    // Clean up the response (remove any extra whitespace/newlines)
    let commit_message = response.get_text("").trim().to_string();
    
    Ok(commit_message)
}

async fn generate_commit_message_anthropic(original_prompt: &str, git_diff: &str, api_key: &str, prompts_config: &PromptsConfig) -> anyhow::Result<String> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(ANTHROPIC_TIMEOUT_SECS / 2))  // Half timeout for commit messages
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;
    
    let prompt = prompts_config.get_anthropic_commit_prompt(original_prompt, git_diff);
    
    let messages = vec![
        AnthropicMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];
    
    let request_payload = serde_json::json!({
        "model": DEFAULT_ANTHROPIC_MODEL,
        "messages": messages,
        "max_tokens": 1024,
    });
    
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("Anthropic commit generation timed out after {} seconds", ANTHROPIC_TIMEOUT_SECS / 2)
            } else {
                anyhow::anyhow!("Failed to generate commit message via Anthropic: {}", e)
            }
        })?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to generate commit message: HTTP {}", response.status()));
    }
    
    let anthropic_response: AnthropicResponse = response
        .json()
        .await
        .context("Failed to parse commit message response from Anthropic")?;
    
    // Extract text from the first content block
    let commit_message = anthropic_response
        .content
        .into_iter()
        .find(|c| c.content_type == "text")
        .map(|c| c.text.trim().to_string())
        .unwrap_or_else(|| "No commit message generated".to_string());
    
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
