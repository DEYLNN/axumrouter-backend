# AxumRouter

Lightweight AI Gateway — **Rust + Axum 0.7**. Multi-provider LLM router with admin dashboard.

## Documentation

| Doc | Location |
|-----|----------|
| **Full Guide** | `docs/GUIDE.md` — setup, config, run, deploy, troubleshooting |
| **Project Rules** | `AGENTS.md` — arsitektur, conventions |
| **Provider Guide** | `docs/API_KEY_PROVIDER_GUIDE.md` — cara tambah provider baru |
| **Provider Templates** | `docs/provider_templates/apikey/` — template siap copy |

## Quick Start

```bash
# Copy config
cp config/config.example.toml config/config.toml

# Build
cargo build --release

# Run
./target/release/axumrouter
```

Open `http://localhost:3000/admin/` — admin dashboard.

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
│   │   ├── registry.rs      # Provider registration
│   │   ├── manager.rs       # Provider lifecycle
│   │   ├── traits.rs        # Provider trait
│   │   ├── key_manager.rs   # Key failover, cooldown
│   │   ├── error_classifier.rs
│   │   └── <id>/            # Per-provider: constants, provider, auth, client
│   ├── engine/              # Generic OpenAI-compat engine
│   ├── services/            # Gateway, caveman, tool normalizer, RTK
│   ├── middleware/           # Auth (bearer token), logging
│   └── types/               # Shared: chat, model, provider structs
├── config/
│   ├── config.example.toml
│   └── config.toml          # Actual config (git-ignored)
├── public/
│   ├── providers/           # Provider icons
│   └── admin/               # Frontend SPA build
└── docs/
    ├── GUIDE.md
    └── API_KEY_PROVIDER_GUIDE.md
```

## Tech Stack

| Layer | Tech |
|-------|------|
| Framework | Axum 0.7 |
| Database | SQLite (sqlx 0.8) |
| HTTP Client | reqwest 0.12 |
| Auth | JWT + Bearer tokens |
| Config | TOML + env vars (AXUM\_ prefix) |
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
- *(full list di `docs/GUIDE.md`)*

## Providers

69+ providers registered. 15 core (Mistral, OpenCode, Codex, xAI, FreeBuff, dll.) + ~54 OpenAI-compat API Key providers.

## Related

- Frontend: `../frontend/`
