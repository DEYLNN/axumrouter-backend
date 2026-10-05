# NuvCode

Lightweight AI Gateway — **Rust + Axum 0.7**. Multi-provider LLM router with admin dashboard.

## Quick Start

```bash
# Copy config
cp config/config.example.toml config/config.toml

# Build
cargo build --release

# Run
./target/release/axumrouter
```

Open `http://localhost:7444/admin/` — admin dashboard.

## Architecture

```
backend/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # Router builder (health, /v1/*, /admin/*, SPA)
│   ├── state.rs             # AppState (config, db, provider manager)
│   ├── error.rs             # GatewayError — OpenAI-compatible error format
│   ├── config/              # Config loader (TOML + AXUM_ env vars)
│   ├── db/                  # SQLite — migrations, models, queries
│   ├── api/                 # /v1/* — chat completions, models, health
│   ├── admin/               # /admin/api* — providers, keys, logs, usage, OAuth
│   ├── providers/           # 69+ provider implementations
│   ├── engine/              # Generic OpenAI-compat engine
│   ├── services/            # Gateway, caveman, tool normalizer, RTK
│   ├── middleware/          # Auth (bearer token), logging
│   └── types/               # Shared: chat, model, provider structs
├── config/
│   ├── config.example.toml
│   └── config.toml          # Actual config (git-ignored)
├── public/
│   ├── providers/           # Provider icons
│   └── admin/               # Frontend SPA build
└── docs/
```

## Tech Stack

| Layer | Tech |
|-------|------|
| Framework | Axum 0.7 |
| Database | SQLite (sqlx 0.8) |
| HTTP Client | reqwest 0.12 |
| Auth | JWT + Bearer tokens |
| Config | TOML + env vars |
| Logging | tracing + env-filter |

## API

### OpenAI-compatible
- `GET /health` — health check
- `GET /v1/models` — list models (`provider_id/model_name`)
- `POST /v1/chat/completions` — chat completion (stream & non-stream)

### Admin
- `GET /admin/api/providers` — list providers
- `POST /admin/api/keys` — add API key
- `GET /admin/api/logs` — request logs
- `GET /admin/api/usage/stats` — usage stats

## Providers

69+ providers registered. 15 core + ~54 OpenAI-compatible API Key providers.

## License

MIT
