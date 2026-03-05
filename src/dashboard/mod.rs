pub mod handlers;
pub mod pages;

use axum::{routing::get, Router};

use crate::logging::db::DbLogger;

pub fn router(db: DbLogger) -> Router {
    Router::new()
        .route("/dashboard", get(handlers::dashboard_page))
        .route("/dashboard/errors", get(handlers::dashboard_errors_page))
        .route(
            "/dashboard/requests/:id",
            get(handlers::request_detail_page),
        )
        .route(
            "/dashboard/partials/requests",
            get(handlers::requests_rows_partial),
        )
        .with_state(db)
}
