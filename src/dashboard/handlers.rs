use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Deserialize;

use crate::{
    dashboard::pages,
    logging::db::{DbLogger, RequestListSearch},
};

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    page: Option<u32>,
    page_size: Option<u32>,
    model: Option<String>,
    provider: Option<String>,
    has_error: Option<String>,
    q: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
}

impl DashboardQuery {
    fn into_search(self) -> RequestListSearch {
        RequestListSearch {
            page: self.page.unwrap_or(1).max(1),
            page_size: self.page_size.unwrap_or(20).clamp(1, 200),
            model: self.model.filter(|s| !s.trim().is_empty()),
            provider: self.provider.filter(|s| !s.trim().is_empty()),
            has_error: self.has_error.as_deref() == Some("1"),
            q: self.q.filter(|s| !s.trim().is_empty()),
            from: self.from,
            to: self.to,
        }
    }
}

pub async fn dashboard_errors_page(
    State(db): State<DbLogger>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let mut search = query.into_search();
    search.has_error = true;
    match db.distinct_model_provider_values().await {
        Ok((models, providers)) => Html(pages::render_dashboard_error_page(
            &models, &providers, &search,
        ))
        .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "failed to load dashboard error filters");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(pages::render_error_page("Failed to load error dashboard.")),
            )
                .into_response()
        }
    }
}

pub async fn dashboard_page(
    State(db): State<DbLogger>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let search = query.into_search();
    match db.distinct_model_provider_values().await {
        Ok((models, providers)) => {
            Html(pages::render_dashboard_page(&models, &providers, &search)).into_response()
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to load dashboard filters");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(pages::render_error_page("Failed to load dashboard.")),
            )
                .into_response()
        }
    }
}

pub async fn requests_rows_partial(
    State(db): State<DbLogger>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let search = query.into_search();
    match db.list_requests(&search).await {
        Ok(result) => Html(pages::render_requests_table(&result, &search)).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "failed to load dashboard request rows");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<div class=\"p-4 text-red-700\">Failed to load requests.</div>".to_string()),
            )
                .into_response()
        }
    }
}

pub async fn request_detail_page(
    State(db): State<DbLogger>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.get_request(&id).await {
        Ok(Some(row)) => Html(pages::render_request_detail(&row)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Html(pages::render_not_found_page(&id)),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, request_id = %id, "failed to load request detail");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(pages::render_error_page("Failed to load request details.")),
            )
                .into_response()
        }
    }
}
