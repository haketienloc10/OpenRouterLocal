use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::providers::{ProviderAdapter, ProviderError};
use crate::types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk};

pub struct GeminiHttpAdapter {
    client: Client,
    base_url: String,
    api_key: String,
}

impl GeminiHttpAdapter {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }
}

#[async_trait]
impl ProviderAdapter for GeminiHttpAdapter {
    async fn chat(&self, req: NormalizedChatRequest) -> Result<NormalizedChatResponse, ProviderError> {
        let contents: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let role = if m.role == "assistant" { "model" } else { "user" };
                json!({"role": role, "parts": [{"text": m.content}]})
            })
            .collect();

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            req.model,
            self.api_key
        );

        let payload = json!({ "contents": contents });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        if !status.is_success() {
            return Err(ProviderError::Http(body.to_string()));
        }

        let text = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(NormalizedChatResponse {
            content: text,
            finish_reason: "stop".to_string(),
        })
    }

    async fn chat_stream(
        &self,
        req: NormalizedChatRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), ProviderError> {
        let res = self.chat(req).await?;
        for chunk in res.content.as_bytes().chunks(64) {
            tx.send(StreamChunk {
                content_delta: String::from_utf8_lossy(chunk).to_string(),
            })
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        }
        Ok(())
    }
}
