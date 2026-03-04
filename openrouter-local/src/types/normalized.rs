use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub stop: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct NormalizedChatResponse {
    pub content: String,
    pub finish_reason: String,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content_delta: String,
}
