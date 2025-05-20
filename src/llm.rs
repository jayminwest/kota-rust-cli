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
        .timeout(Duration::from_secs(30))  // 30 second timeout for the entire request
        .connect_timeout(Duration::from_secs(5))  // 5 second timeout for establishing connection
        .build()
        .context("Failed to create HTTP client")?;

    let mut messages = Vec::new();

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
