use futures_util::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::watch;
use crate::prompt::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    pub temperature: f32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub max_tokens: usize,
    #[serde(default)]
    pub stop: Vec<String>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 800,
            stop: Vec::new(),
        }
    }
}

/// Normalizes the completion endpoint URL to ensure /chat/completions is present.
pub fn normalize_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

/// Normalizes the base endpoint URL for /models
pub fn normalize_models_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if let Some(base) = trimmed.strip_suffix("/chat/completions") {
        format!("{base}/models")
    } else if trimmed.ends_with("/models") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/models")
    }
}

/// Fetches available model IDs from an OpenAI-compatible /v1/models endpoint
pub async fn fetch_models(endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    let models_url = normalize_models_endpoint(endpoint);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.get(&models_url);
    if !api_key.is_empty() && api_key != "none" {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await.map_err(|e| format!("Failed to connect to {}: {}", models_url, e))?;
    if !response.status().is_success() {
        return Err(format!("Model endpoint returned HTTP status {}", response.status()));
    }

    let json_val: Value = response.json().await.map_err(|e| format!("Invalid JSON from model endpoint: {}", e))?;
    
    let mut model_ids = Vec::new();
    if let Some(data_array) = json_val.get("data").and_then(|v| v.as_array()) {
        for item in data_array {
            if let Some(id_str) = item.get("id").and_then(|id| id.as_str()) {
                model_ids.push(id_str.to_string());
            }
        }
    } else if let Some(models_array) = json_val.get("models").and_then(|v| v.as_array()) {
        // Ollama /api/tags or similar structure fallback
        for item in models_array {
            if let Some(name_str) = item.get("name").and_then(|n| n.as_str()) {
                model_ids.push(name_str.to_string());
            }
        }
    }

    model_ids.sort();
    Ok(model_ids)
}

/// Streams text tokens from an OpenAI-compatible chat completion endpoint.
/// Can be cleanly aborted mid-stream by signaling through `cancel_rx`.
pub async fn stream_chat_completion<F>(
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    params: &GenerationParams,
    mut cancel_rx: Option<watch::Receiver<bool>>,
    mut on_token: F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    let target_url = normalize_endpoint(endpoint);
    let client = Client::new();

    let mut body = json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "temperature": params.temperature,
        "top_p": params.top_p,
        "frequency_penalty": params.frequency_penalty,
        "presence_penalty": params.presence_penalty,
        "max_tokens": params.max_tokens,
    });

    if !params.stop.is_empty() {
        body["stop"] = json!(params.stop);
    }

    let mut request = client.post(&target_url).json(&body);
    if !api_key.is_empty() && api_key != "none" {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let mut es = EventSource::new(request).map_err(|e| format!("Failed to initiate SSE connection: {}", e))?;
    let mut accumulated = String::new();

    loop {
        // Check for cancellation
        if let Some(rx) = &mut cancel_rx {
            if *rx.borrow() {
                es.close();
                break;
            }
        }

        tokio::select! {
            _ = async {
                if let Some(rx) = &mut cancel_rx {
                    let _ = rx.changed().await;
                } else {
                    futures_util::future::pending::<()>().await;
                }
            } => {
                if let Some(rx) = &cancel_rx {
                    if *rx.borrow() {
                        es.close();
                        break;
                    }
                }
            }
            event_option = es.next() => {
                match event_option {
                    Some(Ok(Event::Open)) => continue,
                    Some(Ok(Event::Message(message))) => {
                        if message.data.trim() == "[DONE]" {
                            break;
                        }

                        if let Ok(json) = serde_json::from_str::<Value>(&message.data) {
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                accumulated.push_str(content);
                                on_token(content.to_string());
                            }
                        }
                    }
                    Some(Err(err)) => {
                        es.close();
                        // If we already received tokens, treat connection termination as natural finish
                        if accumulated.is_empty() {
                            return Err(format!("Stream error from {}: {}", target_url, err));
                        }
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }

    Ok(accumulated)
}
