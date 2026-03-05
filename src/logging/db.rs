use std::{path::Path, time::Duration};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Pool, QueryBuilder, Sqlite,
};

#[derive(Clone)]
pub struct DbLogger {
    pub pool: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
pub struct LogRecord {
    pub id: String,
    pub created_at: i64,
    pub model: String,
    pub provider: String,
    pub request_json: String,
    pub response_text: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub latency_ms: Option<i64>,
    pub cost: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DashboardRequestRow {
    pub id: String,
    pub created_at: i64,
    pub model: String,
    pub provider: String,
    pub request_json: String,
    pub response_text: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub latency_ms: Option<i64>,
    pub cost: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RequestListSearch {
    pub page: u32,
    pub page_size: u32,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub has_error: bool,
    pub q: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RequestListResult {
    pub rows: Vec<DashboardRequestRow>,
    pub total_count: i64,
}

impl DbLogger {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        let db_path = Path::new(path);
        if let Some(parent) = db_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }

        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await?;

        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous=NORMAL;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout=5000;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON;")
            .execute(&pool)
            .await?;

        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_llm_requests_created_at ON llm_requests (created_at)",
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_llm_requests_model ON llm_requests (model)")
            .execute(&pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_llm_requests_provider ON llm_requests (provider)",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    pub async fn persist(&self, rec: LogRecord) {
        const MAX_RETRIES: usize = 3;

        tracing::debug!(id = %rec.id, "db persist start");

        for attempt in 0..=MAX_RETRIES {
            let query = sqlx::query(
                r#"INSERT INTO llm_requests
                (id, created_at, model, provider, request_json, response_text, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, error)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            )
            .bind(&rec.id)
            .bind(rec.created_at)
            .bind(&rec.model)
            .bind(&rec.provider)
            .bind(&rec.request_json)
            .bind(&rec.response_text)
            .bind(rec.prompt_tokens)
            .bind(rec.completion_tokens)
            .bind(rec.total_tokens)
            .bind(rec.latency_ms)
            .bind(rec.cost)
            .bind(&rec.error)
            .execute(&self.pool)
            .await;

            match query {
                Ok(_) => {
                    tracing::debug!(id = %rec.id, "db persist success");
                    return;
                }
                Err(err) if is_sqlite_busy(&err) && attempt < MAX_RETRIES => {
                    let delay_ms = 50_u64 * (1_u64 << attempt);
                    tracing::warn!(
                        error = %err,
                        id = %rec.id,
                        attempt = attempt + 1,
                        delay_ms,
                        "sqlite busy/locked during persist; retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        sqlite_code = ?err.as_database_error().and_then(|d| d.code()),
                        id = %rec.id,
                        "failed to persist log record"
                    );
                    return;
                }
            }
        }
    }

    pub async fn list_requests(
        &self,
        search: &RequestListSearch,
    ) -> anyhow::Result<RequestListResult> {
        let limit = i64::from(search.page_size.min(200));
        let offset = i64::from(search.page.saturating_sub(1)) * limit;

        let mut rows_query = QueryBuilder::<Sqlite>::new(
            "SELECT id, created_at, model, provider, request_json, response_text, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, error FROM llm_requests WHERE 1=1",
        );
        apply_filters(&mut rows_query, search);
        rows_query.push(" ORDER BY created_at DESC LIMIT ");
        rows_query.push_bind(limit);
        rows_query.push(" OFFSET ");
        rows_query.push_bind(offset);

        let rows = rows_query
            .build_query_as::<DashboardRequestRow>()
            .fetch_all(&self.pool)
            .await?;

        let mut count_query =
            QueryBuilder::<Sqlite>::new("SELECT COUNT(*) as count FROM llm_requests WHERE 1=1");
        apply_filters(&mut count_query, search);
        let total_count: i64 = count_query
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await?;

        Ok(RequestListResult { rows, total_count })
    }

    pub async fn get_request(&self, id: &str) -> anyhow::Result<Option<DashboardRequestRow>> {
        let row = sqlx::query_as::<_, DashboardRequestRow>(
            "SELECT id, created_at, model, provider, request_json, response_text, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, error FROM llm_requests WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn distinct_model_provider_values(
        &self,
    ) -> anyhow::Result<(Vec<String>, Vec<String>)> {
        let models = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT model FROM llm_requests WHERE model IS NOT NULL AND model != '' ORDER BY model ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let providers = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT provider FROM llm_requests WHERE provider IS NOT NULL AND provider != '' ORDER BY provider ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok((models, providers))
    }
}

fn is_sqlite_busy(err: &sqlx::Error) -> bool {
    let code = err.as_database_error().and_then(|d| d.code());
    matches!(code.as_deref(), Some("5") | Some("6"))
        || err.to_string().contains("database is locked")
}

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, search: &RequestListSearch) {
    if let Some(model) = &search.model {
        qb.push(" AND model = ");
        qb.push_bind(model.clone());
    }

    if let Some(provider) = &search.provider {
        qb.push(" AND provider = ");
        qb.push_bind(provider.clone());
    }

    if search.has_error {
        qb.push(" AND error IS NOT NULL AND trim(error) != ''");
    }

    if let Some(term) = &search.q {
        let like = format!("%{term}%");
        qb.push(" AND (request_json LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR response_text LIKE ");
        qb.push_bind(like.clone());
        qb.push(")");
    }

    if let Some(from) = search.from {
        qb.push(" AND created_at >= ");
        qb.push_bind(from);
    }

    if let Some(to) = search.to {
        qb.push(" AND created_at <= ");
        qb.push_bind(to);
    }
}
