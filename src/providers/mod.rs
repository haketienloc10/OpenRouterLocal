use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk};

pub mod cli;
pub mod gemini_http;
pub mod openai_http;

pub fn split_text_by_char_count(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0;

    for ch in text.chars() {
        if current_chars >= max_chars {
            chunks.push(std::mem::take(&mut current));
            current_chars = 0;
        }
        current.push(ch);
        current_chars += 1;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

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

#[cfg(test)]
mod tests {
    use super::split_text_by_char_count;

    #[test]
    fn keeps_unicode_characters_intact_when_chunking() {
        let chunks = split_text_by_char_count("thầm thơ", 3);
        assert_eq!(chunks, vec!["thầ", "m t", "hơ"]);
        assert_eq!(chunks.concat(), "thầm thơ");
    }
}
