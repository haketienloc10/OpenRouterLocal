use std::{collections::HashMap, sync::Arc, time::Instant};

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    logging::db::{DbLogger, LogRecord},
    providers::{ProviderAdapter, ProviderError},
    token::TokenCounter,
    types::normalized::{NormalizedChatRequest, NormalizedChatResponse, StreamChunk},
};

#[derive(Clone)]
pub struct ModelRouter {
    pub config: Arc<AppConfig>,
    pub providers: Arc<HashMap<String, Arc<dyn ProviderAdapter>>>,
    pub db: DbLogger,
    pub token_counter: Arc<dyn TokenCounter>,
}

impl ModelRouter {
    pub async fn chat(&self, req: NormalizedChatRequest) -> Result<NormalizedChatResponse, ProviderError> {
        let request_id = Uuid::new_v4().to_string();
        let started = Instant::now();

        let mut try_models = vec![req.model.clone()];
        try_models.extend(self.config.fallback_models.clone());

        let request_json = serde_json::to_string(&req.messages).unwrap_or_default();
        let mut last_err: Option<ProviderError> = None;

        for model in try_models {
            let Some(model_cfg) = self.config.models.get(&model) else {
                continue;
            };
            let Some(provider) = self.providers.get(&model_cfg.provider) else {
                continue;
            };

            let mut call_req = req.clone();
            call_req.model = model.clone();

            match provider.chat(call_req.clone()).await {
                Ok(resp) => {
                    let prompt_tokens = self.token_counter.count_prompt(&model, &call_req.messages);
                    let completion_tokens = self.token_counter.count_completion(&model, &resp.content);
                    let total_tokens = prompt_tokens + completion_tokens;
                    let latency = started.elapsed().as_millis() as i64;
                    let cost = (prompt_tokens as f64 / 1_000_000f64) * model_cfg.pricing.input_per_1m
                        + (completion_tokens as f64 / 1_000_000f64) * model_cfg.pricing.output_per_1m;

                    self.db
                        .persist(LogRecord {
                            id: request_id,
                            created_at: Utc::now().timestamp(),
                            model,
                            provider: model_cfg.provider.clone(),
                            request_json,
                            response_text: Some(resp.content.clone()),
                            prompt_tokens: Some(prompt_tokens as i64),
                            completion_tokens: Some(completion_tokens as i64),
                            total_tokens: Some(total_tokens as i64),
                            latency_ms: Some(latency),
                            cost: Some(cost),
                            error: None,
                        })
                        .await;
                    return Ok(resp);
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.unwrap_or(ProviderError::Config("no route found".to_string())))
    }

    pub async fn chat_stream(
        &self,
        req: NormalizedChatRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(), ProviderError> {
        let model_cfg = self
            .config
            .models
            .get(&req.model)
            .ok_or_else(|| ProviderError::Config("model not found".to_string()))?;
        let provider = self
            .providers
            .get(&model_cfg.provider)
            .ok_or_else(|| ProviderError::Config("provider not found".to_string()))?;
        provider.chat_stream(req, tx).await
    }
}
