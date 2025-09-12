use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
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
            temperature: Some(0.7),
            stream: if streaming { Some(true) } else { None },
        }
    }

    pub fn benchmark_request(model: &str, streaming: bool) -> Self {
        Self::simple_text_request(
            model,
            "Write a short paragraph about the benefits of API performance benchmarking.",
            streaming,
        )
    }
}