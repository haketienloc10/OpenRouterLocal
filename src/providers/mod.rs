use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk};

pub mod cli;
pub mod gemini_http;
pub mod openai_http;

#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("http upstream error: {0}")]
    Http(String),
    #[error("cli execution error: {0}")]
    Cli(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("config error: {0}")]
    Config(String),
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn chat(&self, req: NormalizedChatRequest) -> Result<NormalizedChatResponse, ProviderError>;
    async fn chat_stream(
        &self,
        req: NormalizedChatRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), ProviderError>;
}
