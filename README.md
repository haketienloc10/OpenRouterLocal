# openrouter-local

Local gateway tương thích OpenAI API (`/v1/chat/completions`) với khả năng route nhiều provider (OpenAI HTTP, Gemini HTTP, CLI), fallback model, logging request và dashboard theo dõi.

## Tính năng chính

- API tương thích OpenAI Chat Completions:
  - `POST /v1/chat/completions` (stream + non-stream)
  - `GET /v1/models`
- Router theo model, hỗ trợ `fallback_models` khi model chính lỗi.
- Provider hỗ trợ:
  - `openai_http`
  - `gemini_http`
  - `cli` (ví dụ `gemini`, `qwen`)
- Ghi log request/response/token/cost vào SQLite (`openrouter_local.db`).
- Dashboard web:
  - `GET /dashboard`
  - Chi tiết request: `/dashboard/requests/:id`

## Yêu cầu

- Rust toolchain ổn định (khuyến nghị mới)
- (Tuỳ chọn) CLI tools nếu dùng provider `cli`:
  - `gemini`
  - `qwen`

## Cấu hình

### 1) Environment

```bash
cp .env.example .env
# cập nhật API keys thật
```

Biến môi trường mặc định:

- `OPENAI_API_KEY`
- `GEMINI_API_KEY`
- `ROUTER_CONFIG` (tuỳ chọn, mặc định `./config/router.yaml`)

### 2) Router config

File mặc định: `config/router.yaml`

Các phần chính:

- `server.bind`, `server.port`
- `providers.<id>`:
  - `kind`: `openai_http` | `gemini_http` | `cli`
  - HTTP provider dùng `base_url`, `api_key_env`
  - CLI provider dùng `command`, `args`
- `models.<model_name>.provider`
- `models.<model_name>.pricing`
- `fallback_models` (danh sách model dùng khi model trước đó thất bại)

## Chạy ứng dụng

### Chạy foreground

```bash
cargo run
# hoặc
cargo run -- serve
```

### Quản lý tiến trình background

```bash
cargo run -- start
cargo run -- stop
cargo run -- restart
cargo run -- logs -n 200
cargo run -- logs -f
```

Log và PID được lưu trong thư mục `logs/`.

## API examples

### Non-streaming

```bash
curl -s http://127.0.0.1:18790/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"gpt-4o",
    "messages":[{"role":"user","content":"Say hello from openrouter-local"}],
    "stream":false
  }'
```

### Streaming

```bash
curl -N http://127.0.0.1:18790/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"gemini-cli",
    "messages":[{"role":"user","content":"Stream a short poem"}],
    "stream":true
  }'
```

### Danh sách model

```bash
curl -s http://127.0.0.1:18790/v1/models | jq
```

## Dashboard

Mở trình duyệt tại:

- `http://127.0.0.1:18790/dashboard`

Dashboard hiển thị danh sách request, filter theo model/provider/lỗi, và xem chi tiết từng request.

## Ghi chú về CLI provider

Với cấu hình mặc định trong `config/router.yaml`:

- `gemini-cli` dùng lệnh tương đương:
  - `gemini -m gemini-2.5-flash -p "<prompt>"`
- `qwen-cli` dùng lệnh tương đương:
  - `qwen "<prompt>"`

Đảm bảo các binary tương ứng tồn tại trong `PATH`.
