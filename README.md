# openrouter-local

## Run

```bash
cp .env.example .env
# edit .env and set your real API keys
# optional
# export ROUTER_CONFIG=./config/router.yaml
cargo run
```

## curl examples

Non-streaming OpenAI model:

```bash
curl -s http://127.0.0.1:18790/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"gpt-4o",
    "messages":[{"role":"user","content":"Say hello from openrouter-local"}],
    "stream":false
  }'
```

Streaming CLI model:

```bash
curl -N http://127.0.0.1:18790/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"gemini-cli",
    "messages":[{"role":"user","content":"Stream a short poem"}],
    "stream":true
  }'
```
