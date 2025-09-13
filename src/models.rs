use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Option<ChatMessage>,
    pub delta: Option<ChatMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: Option<u32>,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedModel {
    pub model_name: String,
    pub model_provider: String,
    pub created_at: Option<String>,
    pub sudo_model_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<SupportedModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
}

// Streaming event structure for SSE
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StreamingEvent {
    pub event_type: String,
    pub data: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ChatCompletionRequest {
    pub fn simple_text_request(model: &str, message: &str, streaming: bool) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            model: model.to_string(),
            max_completion_tokens: Some(150),
            stream: if streaming { Some(true) } else { None },
            stream_options: None,
        }
    }

    pub fn benchmark_request(model: &str, streaming: bool) -> Self {
        Self::simple_text_request(
            model,
            "Write a short paragraph about the benefits of API performance benchmarking.",
            streaming,
        )
    }

    // For latency, minimize generated tokens to reduce tail time and highlight TTFT.
    pub fn benchmark_latency_request(model: &str, streaming: bool) -> Self {
        let mut req = Self::benchmark_request(model, streaming);
        req.max_completion_tokens = Some(8);
        req
    }

    // For throughput (tokens/sec), allow larger generations to amortize overhead.
    pub fn benchmark_throughput_request(model: &str, streaming: bool) -> Self {
        let mut req = Self::benchmark_request(model, streaming);
        req.max_completion_tokens = Some(512);
        req
    }
}
