use crate::prompt::ChatMessage;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::watch;

fn default_temperature() -> f32 {
    0.7
}

fn default_top_p() -> f32 {
    0.9
}

fn default_max_tokens() -> usize {
    800
}

fn default_min_p() -> f32 {
    0.0
}

fn default_top_k() -> u32 {
    40
}

fn default_repetition_penalty() -> f32 {
    1.05
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationParams {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default)]
    pub frequency_penalty: f32,
    #[serde(default)]
    pub presence_penalty: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub stop: Vec<String>,
    #[serde(default = "default_min_p")]
    pub min_p: f32,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_repetition_penalty")]
    pub repetition_penalty: f32,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            top_p: default_top_p(),
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: default_max_tokens(),
            stop: Vec::new(),
            min_p: default_min_p(),
            top_k: default_top_k(),
            repetition_penalty: default_repetition_penalty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiType {
    #[default]
    OpenAi,
    Anthropic,
    KoboldCpp,
}

impl ApiType {
    pub fn from_str_or_detect(api_type: Option<&str>, endpoint: &str) -> Self {
        if let Some(t) = api_type {
            match t.to_lowercase().trim() {
                "anthropic" | "claude" => return ApiType::Anthropic,
                "kobold" | "koboldcpp" => return ApiType::KoboldCpp,
                "openai" | "openrouter" | "ollama" | "lmstudio" | "vllm" => return ApiType::OpenAi,
                _ => {}
            }
        }
        Self::detect(endpoint)
    }

    pub fn detect(endpoint: &str) -> Self {
        let lower = endpoint.to_lowercase();
        if lower.contains("anthropic.com") || lower.contains("/v1/messages") {
            ApiType::Anthropic
        } else if lower.contains(":5001")
            || lower.contains("/api/v1/generate")
            || lower.contains("/api/extra")
        {
            ApiType::KoboldCpp
        } else {
            ApiType::OpenAi
        }
    }
}

pub fn normalize_endpoint(endpoint: &str) -> String {
    normalize_endpoint_for_type(endpoint, ApiType::detect(endpoint))
}

pub fn normalize_endpoint_for_type(endpoint: &str, api_type: ApiType) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    match api_type {
        ApiType::Anthropic => {
            if trimmed.ends_with("/v1/messages") || trimmed.ends_with("/messages") {
                trimmed.to_string()
            } else if trimmed.ends_with("/v1") {
                format!("{trimmed}/messages")
            } else {
                format!("{trimmed}/v1/messages")
            }
        }
        ApiType::KoboldCpp => {
            if trimmed.ends_with("/api/extra/generate/stream") {
                trimmed.to_string()
            } else if let Some(base) = trimmed.strip_suffix("/api/v1/generate") {
                format!("{base}/api/extra/generate/stream")
            } else if let Some(base) = trimmed.strip_suffix("/v1") {
                format!("{base}/api/extra/generate/stream")
            } else {
                format!("{trimmed}/api/extra/generate/stream")
            }
        }
        ApiType::OpenAi => {
            if trimmed.ends_with("/chat/completions") {
                trimmed.to_string()
            } else {
                format!("{trimmed}/chat/completions")
            }
        }
    }
}

pub fn normalize_models_endpoint(endpoint: &str) -> String {
    normalize_models_endpoint_for_type(endpoint, ApiType::detect(endpoint))
}

pub fn normalize_models_endpoint_for_type(endpoint: &str, api_type: ApiType) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    match api_type {
        ApiType::Anthropic => {
            if trimmed.ends_with("/v1/models") {
                trimmed.to_string()
            } else if let Some(base) = trimmed.strip_suffix("/v1/messages") {
                format!("{base}/v1/models")
            } else if let Some(base) = trimmed.strip_suffix("/messages") {
                format!("{base}/models")
            } else if trimmed.ends_with("/v1") {
                format!("{trimmed}/models")
            } else {
                format!("{trimmed}/v1/models")
            }
        }
        ApiType::KoboldCpp => {
            if trimmed.ends_with("/api/v1/model") {
                trimmed.to_string()
            } else if let Some(base) = trimmed.strip_suffix("/api/extra/generate/stream") {
                format!("{base}/api/v1/model")
            } else if let Some(base) = trimmed.strip_suffix("/api/v1/generate") {
                format!("{base}/api/v1/model")
            } else if let Some(base) = trimmed.strip_suffix("/v1") {
                format!("{base}/api/v1/model")
            } else {
                format!("{trimmed}/api/v1/model")
            }
        }
        ApiType::OpenAi => {
            if let Some(base) = trimmed.strip_suffix("/chat/completions") {
                format!("{base}/models")
            } else if trimmed.ends_with("/models") {
                trimmed.to_string()
            } else {
                format!("{trimmed}/models")
            }
        }
    }
}

pub async fn fetch_models(endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    fetch_models_with_type(endpoint, api_key, None).await
}

pub async fn fetch_models_with_type(
    endpoint: &str,
    api_key: &str,
    api_type: Option<&str>,
) -> Result<Vec<String>, String> {
    let resolved_type = ApiType::from_str_or_detect(api_type, endpoint);
    let models_url = normalize_models_endpoint_for_type(endpoint, resolved_type);
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.get(&models_url);

    match resolved_type {
        ApiType::Anthropic => {
            if !api_key.is_empty() && api_key != "none" {
                request = request
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01");
            }
        }
        ApiType::KoboldCpp | ApiType::OpenAi => {
            if !api_key.is_empty() && api_key != "none" {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }
        }
    }

    let response = match request.send().await {
        Ok(res) => res,
        Err(e) => {
            // For Anthropic, if endpoint fails (e.g. models API unavailable without auth or 404), return fallback models
            if resolved_type == ApiType::Anthropic {
                return Ok(vec![
                    "claude-3-7-sonnet-latest".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-5-sonnet-latest".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
                    "claude-3-5-haiku-latest".to_string(),
                    "claude-3-opus-20240229".to_string(),
                ]);
            }
            return Err(format!("Failed to connect to {models_url}: {e}"));
        }
    };

    if !response.status().is_success() {
        if resolved_type == ApiType::Anthropic {
            return Ok(vec![
                "claude-3-7-sonnet-latest".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-sonnet-latest".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                "claude-3-5-haiku-latest".to_string(),
                "claude-3-opus-20240229".to_string(),
            ]);
        }
        return Err(format!(
            "Model endpoint returned HTTP status {}",
            response.status()
        ));
    }

    let json_val: Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid JSON from model endpoint: {e}"))?;

    let mut model_ids = Vec::new();

    // KoboldCpp returns {"result": "model_name"}
    if let Some(res_str) = json_val.get("result").and_then(|v| v.as_str())
        && !res_str.is_empty()
    {
        model_ids.push(res_str.to_string());
    }

    // OpenAI/Anthropic format {"data": [{"id": "..."}, ...]}
    if let Some(data_array) = json_val.get("data").and_then(|v| v.as_array()) {
        for item in data_array {
            if let Some(id_str) = item.get("id").and_then(|id| id.as_str()) {
                model_ids.push(id_str.to_string());
            }
        }
    } else if let Some(models_array) = json_val.get("models").and_then(|v| v.as_array()) {
        // Ollama format {"models": [{"name": "..."}, ...]}
        for item in models_array {
            if let Some(name_str) = item.get("name").and_then(|n| n.as_str()) {
                model_ids.push(name_str.to_string());
            }
        }
    }

    if model_ids.is_empty() && resolved_type == ApiType::Anthropic {
        model_ids.extend([
            "claude-3-7-sonnet-latest".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-latest".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
        ]);
    }

    model_ids.sort();
    model_ids.dedup();
    Ok(model_ids)
}

/// Helper to manage streaming tokens, wrapping reasoning content in `<think>...</think>` tags
pub struct ReasoningStreamer<F> {
    on_token: F,
    in_reasoning: bool,
    accumulated: String,
}

impl<F: FnMut(String)> ReasoningStreamer<F> {
    pub fn new(on_token: F) -> Self {
        Self {
            on_token,
            in_reasoning: false,
            accumulated: String::new(),
        }
    }

    pub fn push_reasoning(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        if !self.in_reasoning {
            // Only add <think> if not already starting with <think>
            if !chunk.trim_start().starts_with("<think>") {
                let tag = "<think>\n";
                (self.on_token)(tag.to_string());
                self.accumulated.push_str(tag);
            }
            self.in_reasoning = true;
        }
        (self.on_token)(chunk.to_string());
        self.accumulated.push_str(chunk);
    }

    pub fn push_content(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        if self.in_reasoning {
            if !self.accumulated.trim_end().ends_with("</think>") {
                let tag = "\n</think>\n\n";
                (self.on_token)(tag.to_string());
                self.accumulated.push_str(tag);
            }
            self.in_reasoning = false;
        }
        (self.on_token)(chunk.to_string());
        self.accumulated.push_str(chunk);
    }

    pub fn finish(&mut self) -> String {
        if self.in_reasoning {
            if !self.accumulated.trim_end().ends_with("</think>") {
                let tag = "\n</think>\n\n";
                (self.on_token)(tag.to_string());
                self.accumulated.push_str(tag);
            }
            self.in_reasoning = false;
        }
        self.accumulated.clone()
    }
}
#[allow(clippy::too_many_arguments)]
pub async fn stream_chat_completion<F>(
    endpoint: &str,
    api_key: &str,
    api_type: Option<&str>,
    model: &str,
    messages: &[ChatMessage],
    params: &GenerationParams,
    mut cancel_rx: Option<watch::Receiver<bool>>,
    on_token: F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    let resolved_type = ApiType::from_str_or_detect(api_type, endpoint);
    let target_url = normalize_endpoint_for_type(endpoint, resolved_type);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut streamer = ReasoningStreamer::new(on_token);

    match resolved_type {
        ApiType::OpenAi => {
            let mut body = json!({
                "model": model,
                "messages": messages,
                "stream": true,
                "temperature": params.temperature,
                "top_p": params.top_p,
                "frequency_penalty": params.frequency_penalty,
                "presence_penalty": params.presence_penalty,
                "max_tokens": params.max_tokens,
                "min_p": params.min_p,
                "top_k": params.top_k,
                "repetition_penalty": params.repetition_penalty,
            });

            if !params.stop.is_empty() {
                body["stop"] = json!(params.stop);
            }

            let mut request = client.post(&target_url).json(&body);
            if !api_key.is_empty() && api_key != "none" {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }

            let mut es = request
                .eventsource()
                .map_err(|e| format!("Failed to initiate SSE connection to {target_url}: {e}"))?;

            loop {
                tokio::select! {
                    biased;
                    _ = async {
                        if let Some(rx) = &mut cancel_rx {
                            while !*rx.borrow() {
                                if rx.changed().await.is_err() {
                                    futures_util::future::pending::<()>().await;
                                }
                            }
                        } else {
                            futures_util::future::pending::<()>().await;
                        }
                    } => {
                        es.close();
                        break;
                    }

                    event_option = es.next() => {
                        match event_option {
                            Some(Ok(Event::Open)) => continue,
                            Some(Ok(Event::Message(message))) => {
                                let trimmed = message.data.trim();
                                if trimmed == "[DONE]" {
                                    es.close();
                                    break;
                                }

                                if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
                                    let delta = &json["choices"][0]["delta"];

                                    // Extract reasoning content if present (DeepSeek, OpenRouter, Ollama)
                                    let reasoning = delta["reasoning_content"]
                                        .as_str()
                                        .or_else(|| delta["reasoning"].as_str());

                                    if let Some(rc) = reasoning {
                                        streamer.push_reasoning(rc);
                                    }

                                    // Extract regular content
                                    if let Some(content) = delta["content"].as_str() {
                                        streamer.push_content(content);
                                    }
                                }
                            }
                            Some(Err(err)) => {
                                es.close();
                                let final_str = streamer.finish();
                                if final_str.is_empty() {
                                    return Err(format!("Stream error from {target_url}: {err}"));
                                }
                                return Ok(final_str);
                            }
                            None => {
                                es.close();
                                break;
                            }
                        }
                    }
                }
            }
        }

        ApiType::Anthropic => {
            // Separate system messages and conversation messages
            let mut system_parts = Vec::new();
            let mut anthropic_messages: Vec<Value> = Vec::new();

            for msg in messages {
                if msg.role == "system" {
                    system_parts.push(msg.content.clone());
                } else {
                    let role = if msg.role == "assistant" {
                        "assistant"
                    } else {
                        "user"
                    };

                    // Anthropic requires strictly alternating user and assistant messages
                    if let Some(last) = anthropic_messages.last_mut()
                        && last["role"] == role
                    {
                        // Merge consecutive messages of the same role
                        let prev_text = last["content"][0]["text"].as_str().unwrap_or("");
                        let merged = format!("{}\n\n{}", prev_text, msg.content);
                        last["content"][0]["text"] = json!(merged);
                        continue;
                    }

                    anthropic_messages.push(json!({
                        "role": role,
                        "content": [{
                            "type": "text",
                            "text": msg.content
                        }]
                    }));
                }
            }

            // Anthropic messages array MUST start with a user message
            if let Some(first) = anthropic_messages.first()
                && first["role"] == "assistant"
            {
                anthropic_messages.insert(
                    0,
                    json!({
                        "role": "user",
                        "content": [{
                            "type": "text",
                            "text": "..."
                        }]
                    }),
                );
            }

            // Apply prompt caching headers and cache_control breakpoints
            let system_prompt_text = system_parts.join("\n\n");
            let mut body = json!({
                "model": model,
                "messages": anthropic_messages,
                "max_tokens": params.max_tokens,
                "temperature": params.temperature,
                "top_p": params.top_p,
                "stream": true,
            });

            if !system_prompt_text.is_empty() {
                body["system"] = json!([
                    {
                        "type": "text",
                        "text": system_prompt_text,
                        "cache_control": {
                            "type": "ephemeral"
                        }
                    }
                ]);
            }

            if params.top_k > 0 {
                body["top_k"] = json!(params.top_k);
            }

            if !params.stop.is_empty() {
                body["stop_sequences"] = json!(params.stop);
            }

            let mut request = client
                .post(&target_url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "prompt-caching-2024-07-31")
                .json(&body);

            if api_key.is_empty() || api_key == "none" {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }

            let mut es = request
                .eventsource()
                .map_err(|e| format!("Failed to initiate SSE connection to {target_url}: {e}"))?;

            loop {
                tokio::select! {
                    biased;
                    _ = async {
                        if let Some(rx) = &mut cancel_rx {
                            while !*rx.borrow() {
                                if rx.changed().await.is_err() {
                                    futures_util::future::pending::<()>().await;
                                }
                            }
                        } else {
                            futures_util::future::pending::<()>().await;
                        }
                    } => {
                        es.close();
                        break;
                    }

                    event_option = es.next() => {
                        match event_option {
                            Some(Ok(Event::Open)) => continue,
                            Some(Ok(Event::Message(message))) => {
                                let trimmed = message.data.trim();
                                if trimmed == "[DONE]" {
                                    es.close();
                                    break;
                                }

                                if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
                                    let event_type = json["type"].as_str().unwrap_or(message.event.as_str());

                                    if event_type == "content_block_delta" {
                                        let delta = &json["delta"];
                                        let delta_type = delta["type"].as_str().unwrap_or("");

                                        if delta_type == "thinking_delta" {
                                            if let Some(thinking) = delta["thinking"].as_str() {
                                                streamer.push_reasoning(thinking);
                                            }
                                        } else if delta_type == "text_delta" {
                                            if let Some(text) = delta["text"].as_str() {
                                                streamer.push_content(text);
                                            }
                                        } else if let Some(text) = delta["text"].as_str() {
                                            streamer.push_content(text);
                                        }
                                    } else if event_type == "message_stop" {
                                        es.close();
                                        break;
                                    }
                                }
                            }
                            Some(Err(err)) => {
                                es.close();
                                let final_str = streamer.finish();
                                if final_str.is_empty() {
                                    return Err(format!("Stream error from Anthropic {target_url}: {err}"));
                                }
                                return Ok(final_str);
                            }
                            None => {
                                es.close();
                                break;
                            }
                        }
                    }
                }
            }
        }

        ApiType::KoboldCpp => {
            // Build formatted ChatML prompt for KoboldCpp
            let mut prompt = String::new();
            for msg in messages {
                match msg.role.as_str() {
                    "system" => {
                        prompt
                            .push_str(&format!("<|im_start|>system\n{}<|im_end|>\n", msg.content));
                    }
                    "user" => {
                        prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", msg.content));
                    }
                    "assistant" => {
                        prompt.push_str(&format!(
                            "<|im_start|>assistant\n{}<|im_end|>\n",
                            msg.content
                        ));
                    }
                    _ => {
                        prompt.push_str(&format!(
                            "<|im_start|>{}\n{}<|im_end|>\n",
                            msg.role, msg.content
                        ));
                    }
                }
            }
            if !messages
                .last()
                .map(|m| m.role.as_str() == "assistant")
                .unwrap_or(false)
            {
                prompt.push_str("<|im_start|>assistant\n");
            }

            let mut body = json!({
                "prompt": prompt,
                "max_length": params.max_tokens,
                "temperature": params.temperature,
                "top_p": params.top_p,
                "min_p": params.min_p,
                "top_k": params.top_k,
                "rep_pen": params.repetition_penalty,
                "stream": true,
            });

            if !params.stop.is_empty() {
                body["stop_sequence"] = json!(params.stop);
            }

            let mut request = client.post(&target_url).json(&body);
            if !api_key.is_empty() && api_key != "none" {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }

            let mut es = request
                .eventsource()
                .map_err(|e| format!("Failed to initiate SSE connection to {target_url}: {e}"))?;

            loop {
                tokio::select! {
                    biased;
                    _ = async {
                        if let Some(rx) = &mut cancel_rx {
                            while !*rx.borrow() {
                                if rx.changed().await.is_err() {
                                    futures_util::future::pending::<()>().await;
                                }
                            }
                        } else {
                            futures_util::future::pending::<()>().await;
                        }
                    } => {
                        es.close();
                        break;
                    }

                    event_option = es.next() => {
                        match event_option {
                            Some(Ok(Event::Open)) => continue,
                            Some(Ok(Event::Message(message))) => {
                                let trimmed = message.data.trim();
                                if trimmed == "[DONE]" {
                                    es.close();
                                    break;
                                }

                                if let Ok(json) = serde_json::from_str::<Value>(trimmed)
                                    && let Some(tok) = json.get("token").and_then(|t| t.as_str())
                                    && !tok.is_empty()
                                {
                                    streamer.push_content(tok);
                                }
                            }
                            Some(Err(err)) => {
                                es.close();
                                let final_str = streamer.finish();
                                if final_str.is_empty() {
                                    return Err(format!("Stream error from KoboldCpp {target_url}: {err}"));
                                }
                                return Ok(final_str);
                            }
                            None => {
                                es.close();
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(streamer.finish())
}
