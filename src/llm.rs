use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Context;

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

const OLLAMA_API_URL: &str = "http://localhost:8000/api/chat";
const DEFAULT_MODEL: &str = "qwen3:30b";

pub async fn ask_model(prompt: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let messages = vec![OllamaChatMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    }];

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
        .context("Failed to send request to Ollama API")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unkonwn error".to_string());
        return Err(anyhow::anyhow!(
                "Ollama API request failed with status {}: {}",
                status, 
                error_text
        ));
    }

    let ollama_response = response
        .json::<OllamaChatResponse>()
        .await
        .context("failed to parse JSON response from Ollama API")?;

    Ok(ollama_response.message.content)
}
