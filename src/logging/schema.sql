CREATE TABLE IF NOT EXISTS llm_requests (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  model TEXT NOT NULL,
  provider TEXT NOT NULL,
  request_json TEXT NOT NULL,
  response_text TEXT,
  prompt_tokens INTEGER,
  completion_tokens INTEGER,
  total_tokens INTEGER,
  latency_ms INTEGER,
  cost REAL,
  error TEXT
);
