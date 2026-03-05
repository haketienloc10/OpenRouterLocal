use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::providers::{split_text_by_char_count, ProviderAdapter, ProviderError};
use crate::types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk};

pub struct CliAdapter {
    command: String,
    args: Vec<String>,
}

impl CliAdapter {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }

    fn format_prompt(req: &NormalizedChatRequest) -> String {
        req.messages
            .iter()
            .map(|m| format!("{}: {}", m.role.to_uppercase(), m.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl ProviderAdapter for CliAdapter {
    async fn chat(&self, req: NormalizedChatRequest) -> Result<NormalizedChatResponse, ProviderError> {
        let prompt = Self::format_prompt(&req);
        let output = Command::new(&self.command)
            .args(&self.args)
            .arg(prompt)
            .output()
            .await
            .map_err(|e| ProviderError::Cli(e.to_string()))?;

        if !output.status.success() {
            return Err(ProviderError::Cli(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();

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
        for chunk in split_text_by_char_count(&res.content, 48) {
            tx.send(StreamChunk {
                content_delta: chunk,
            })
            .await
            .map_err(|e| ProviderError::Cli(e.to_string()))?;
        }
        Ok(())
    }
}
