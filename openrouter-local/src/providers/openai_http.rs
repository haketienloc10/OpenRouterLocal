use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::providers::{ProviderAdapter, ProviderError};
use crate::types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk};

pub struct OpenAiHttpAdapter {
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenAiHttpAdapter {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    fn to_payload(req: &NormalizedChatRequest, stream: bool) -> Value {
        json!({
            "model": req.model,
            "messages": req.messages,
            "temperature": req.temperature,
            "top_p": req.top_p,
            "max_tokens": req.max_tokens,
            "stop": req.stop,
            "stream": stream
        })
    }
}

#[async_trait]
impl ProviderAdapter for OpenAiHttpAdapter {
    async fn chat(&self, req: NormalizedChatRequest) -> Result<NormalizedChatResponse, ProviderError> {
        let payload = Self::to_payload(&req, false);
        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
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

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let finish_reason = body["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        Ok(NormalizedChatResponse {
            content,
            finish_reason,
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
