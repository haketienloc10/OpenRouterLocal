use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Sse},
    Json,
};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use crate::{
    router::model_router::ModelRouter,
    types::{
        normalized::{Message, NormalizedChatRequest, StreamChunk},
        openai::{
            ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ErrorBody,
            ErrorEnvelope, Usage,
        },
    },
};

pub async fn chat_completions(
    State(router): State<Arc<ModelRouter>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    if !router.config.models.contains_key(&req.model) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    message: format!("model '{}' not found", req.model),
                    error_type: "invalid_request_error".to_string(),
                },
            }),
        )
            .into_response();
    }

    let normalized = NormalizedChatRequest {
        model: req.model.clone(),
        messages: req
            .messages
            .into_iter()
            .map(|m| {
                if m.content.has_non_text_parts() {
                    tracing::warn!(
                        "Received non-text content parts; ignoring them (text-only MVP)"
                    );
                }

                Message {
                    role: m.role,
                    content: m.content.to_plain_text(),
                }
            })
            .collect(),
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream.unwrap_or(false),
        stop: req.stop,
    };

    if normalized.stream {
        let (tx, rx) = mpsc::channel::<StreamChunk>(32);
        let route = router.clone();
        let model = normalized.model.clone();
        tokio::spawn(async move {
            if let Err(err) = route.chat_stream(normalized, tx.clone()).await {
                let _ = tx
                    .send(StreamChunk {
                        content_delta: format!("[error] {}", err),
                    })
                    .await;
            }
        });

        let stream = ReceiverStream::new(rx).map(move |chunk| {
            let payload = json!({
                "id": format!("chatcmpl_{}", uuid::Uuid::new_v4().simple()),
                "object": "chat.completion.chunk",
                "created": chrono::Utc::now().timestamp(),
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": {"content": chunk.content_delta},
                    "finish_reason": null
                }]
            });
            Ok::<_, axum::Error>(axum::response::sse::Event::default().data(payload.to_string()))
        });

        return Sse::new(stream).into_response();
    }

    match router.chat(normalized.clone()).await {
        Ok(resp) => {
            let prompt_tokens = router
                .token_counter
                .count_prompt(&normalized.model, &normalized.messages);
            let completion_tokens = router
                .token_counter
                .count_completion(&normalized.model, &resp.content);
            let usage = Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            };

            Json(ChatCompletionResponse {
                id: format!("chatcmpl_{}", uuid::Uuid::new_v4().simple()),
                object: "chat.completion".to_string(),
                created: chrono::Utc::now().timestamp(),
                model: normalized.model,
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: resp.content,
                    },
                    finish_reason: resp.finish_reason,
                }],
                usage,
            })
            .into_response()
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    message: err.to_string(),
                    error_type: "upstream_error".to_string(),
                },
            }),
        )
            .into_response(),
    }
}
