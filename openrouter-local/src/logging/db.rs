use sqlx::{Pool, Sqlite, SqlitePool};

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

impl DbLogger {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        let url = format!("sqlite://{}", path);
        let pool = SqlitePool::connect(&url).await?;
        sqlx::query(include_str!("schema.sql")).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn persist(&self, rec: LogRecord) {
        let query = sqlx::query(
            r#"INSERT INTO llm_requests
            (id, created_at, model, provider, request_json, response_text, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
        )
        .bind(rec.id)
        .bind(rec.created_at)
        .bind(rec.model)
        .bind(rec.provider)
        .bind(rec.request_json)
        .bind(rec.response_text)
        .bind(rec.prompt_tokens)
        .bind(rec.completion_tokens)
        .bind(rec.total_tokens)
        .bind(rec.latency_ms)
        .bind(rec.cost)
        .bind(rec.error)
        .execute(&self.pool)
        .await;

        if let Err(err) = query {
            tracing::error!(error = %err, "failed to persist log record");
        }
    }
}
