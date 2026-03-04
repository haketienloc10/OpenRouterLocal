mod api;
mod config;
mod logging;
mod providers;
mod router;
mod token;
mod types;

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{routing::{get, post}, Router};
use providers::{cli::CliAdapter, gemini_http::GeminiHttpAdapter, openai_http::OpenAiHttpAdapter, ProviderAdapter};
use router::model_router::ModelRouter;
use token::{naive::NaiveTokenCounter, TokenCounter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::AppConfig, logging::db::DbLogger};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,openrouter_local=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(AppConfig::load()?);
    let db = DbLogger::new("openrouter_local.db").await?;

    let mut providers_map: HashMap<String, Arc<dyn ProviderAdapter>> = HashMap::new();
    for (id, p) in &config.providers {
        let adapter: Arc<dyn ProviderAdapter> = match p.kind.as_str() {
            "openai_http" => {
                let api_env = p.api_key_env.clone().ok_or_else(|| anyhow::anyhow!("api_key_env missing"))?;
                let key = std::env::var(api_env)?;
                Arc::new(OpenAiHttpAdapter::new(
                    p.base_url.clone().ok_or_else(|| anyhow::anyhow!("base_url missing"))?,
                    key,
                ))
            }
            "gemini_http" => {
                let api_env = p.api_key_env.clone().ok_or_else(|| anyhow::anyhow!("api_key_env missing"))?;
                let key = std::env::var(api_env)?;
                Arc::new(GeminiHttpAdapter::new(
                    p.base_url.clone().ok_or_else(|| anyhow::anyhow!("base_url missing"))?,
                    key,
                ))
            }
            "cli" => Arc::new(CliAdapter::new(
                p.command.clone().ok_or_else(|| anyhow::anyhow!("command missing"))?,
                p.args.clone(),
            )),
            _ => return Err(anyhow::anyhow!("unsupported provider kind")),
        };
        providers_map.insert(id.clone(), adapter);
    }

    let token_counter: Arc<dyn TokenCounter> = Arc::new(NaiveTokenCounter);
    let model_router = Arc::new(ModelRouter {
        config: config.clone(),
        providers: Arc::new(providers_map),
        db,
        token_counter,
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(api::chat::chat_completions))
        .route("/v1/models", get(api::models::list_models))
        .with_state(model_router);

    let bind_addr = format!("{}:{}", config.server.bind, config.server.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("openrouter-local listening on {}", bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
}
