use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{router::model_router::ModelRouter, types::openai::{ModelItem, ModelsResponse}};

pub async fn list_models(State(router): State<Arc<ModelRouter>>) -> Json<ModelsResponse> {
    let data = router
        .config
        .models
        .iter()
        .map(|(id, m)| ModelItem {
            id: id.clone(),
            object: "model".to_string(),
            owned_by: m.provider.clone(),
        })
        .collect();

    Json(ModelsResponse {
        object: "list".to_string(),
        data,
    })
}
