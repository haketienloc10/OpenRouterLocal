# OpenRouterLocal

**OpenRouterLocal** là một **local gateway tương thích OpenAI API** cho phép route request tới nhiều provider khác nhau (OpenAI HTTP, Gemini HTTP, CLI tools như `gemini`, `qwen`).  

Gateway hỗ trợ logging request vào SQLite và cung cấp **dashboard web** để theo dõi request, token usage và lỗi.

---

# Tính năng

- API tương thích OpenAI:
  - `POST /v1/chat/completions`
  - `GET /v1/models`
- Hỗ trợ **streaming và non-streaming**
- Router theo model
- Hỗ trợ **fallback model**
- Provider hỗ trợ:
  - `openai_http`
  - `gemini_http`
  - `cli` (`gemini`, `qwen`)
- Logging request vào SQLite (`openrouter_local.db`)
- Dashboard web để xem log request

---

# Yêu cầu

- Rust toolchain (stable)
- Cargo

Tuỳ chọn nếu dùng CLI provider:

- `gemini`
- `qwen`

Các binary này cần tồn tại trong `$PATH`.

---

# Cài đặt

## 1. Clone repository

```bash
git clone https://github.com/haketienloc10/OpenRouterLocal.git
cd OpenRouterLocal
````

---

## 2. Cấu hình environment

Copy file `.env.example`:

```bash
cp .env.example .env
```

Sau đó cập nhật API key thật:

```env
OPENAI_API_KEY=your_openai_api_key
GEMINI_API_KEY=your_gemini_api_key
```

Tuỳ chọn:

```env
ROUTER_CONFIG=./config/router.yaml
```

---

## 3. Cài binary

Khuyến nghị cài binary để có thể dùng như lệnh hệ thống:

```bash
cargo install --path . --force
```

Binary sẽ được cài vào:

```
~/.cargo/bin/openrouter-local
```

Đảm bảo `~/.cargo/bin` có trong `$PATH`.

---

# Chạy ứng dụng

## Chạy foreground (debug)

```bash
cargo run
```

hoặc

```bash
cargo run -- serve
```

Server sẽ chạy tại:

```
http://127.0.0.1:18790
```

---

### Chạy daemon (background)

> Khuyến nghị cài binary vào PATH để dùng như một lệnh hệ thống.

Cài / cập nhật binary:

```bash
cargo install --path . --force
```

Sau đó bạn có thể quản lý service bằng các lệnh:

```bash
openrouter-local start
openrouter-local stop
openrouter-local logs -f
openrouter-local restart
```

Bạn cũng có thể xem log không follow:

```bash
openrouter-local logs -n 200
```

Log và PID nằm trong thư mục:

```
logs/server.log
logs/server.pid
```

---

# Kiểm tra server

Test nhanh gateway:

```bash
curl http://127.0.0.1:18790/v1/models
```

---

# API examples

## Chat completion (non-stream)

```bash
curl http://127.0.0.1:18790/v1/chat/completions \
-H "Content-Type: application/json" \
-d '{
  "model":"gpt-4o",
  "messages":[{"role":"user","content":"Say hello from openrouter-local"}],
  "stream":false
}'
```

---

## Chat completion (stream)

```bash
curl -N http://127.0.0.1:18790/v1/chat/completions \
-H "Content-Type: application/json" \
-d '{
  "model":"gemini-cli",
  "messages":[{"role":"user","content":"Write a short poem"}],
  "stream":true
}'
```

---

## Danh sách model

```bash
curl http://127.0.0.1:18790/v1/models
```

---

# Dashboard

Mở trình duyệt tại:

```
http://127.0.0.1:18790/dashboard
```

Dashboard hiển thị:

* danh sách request
* token usage
* latency
* cost
* error
* chi tiết request / response

---

# Cấu hình router

File cấu hình mặc định:

```
config/router.yaml
```

Ví dụ:

```yaml
providers:
  openai:
    kind: openai_http
    base_url: https://api.openai.com/v1
    api_key_env: OPENAI_API_KEY

  gemini:
    kind: gemini_http
    base_url: https://generativelanguage.googleapis.com
    api_key_env: GEMINI_API_KEY

  gemini_cli:
    kind: cli
    command: gemini
    args: ["-m", "gemini-2.5-flash"]

models:
  gpt-4o:
    provider: openai

  gemini-cli:
    provider: gemini_cli
```

---

# Cập nhật khi có source mới

Khi repository có cập nhật mới:

```bash
git pull
```

Sau đó build lại binary:

```bash
cargo install --path . --force
```

Restart service:

```bash
openrouter-local restart
```

---

# Development workflow

Khuyến nghị workflow:

```bash
cargo install --path . --force
openrouter-local start
openrouter-local logs -f
```
