# AxumRouter — Quick Guide
# Generated: 2026-07-08 14:40 UTC

## Project
AxumRouter — OpenAI-compatible API Gateway in Rust (Axum 0.7).
8 providers, multi-key failover, combo routing, gateway key management.

## Paths
- Backend: `/root/.hermes/projects/axumrouter-backend`
- Frontend: `/root/.hermes/projects/axumrouter-frontend`
- DB: `data/axumrouter.db` (SQLite, auto-backup on startup)
- Ports: BE=3000, FE=5173
- VPS: `152.42.198.51:3000` (BE), `152.42.198.51:5173` (FE)

## Build & Run
```bash
# Backend
cd /root/.hermes/projects/axumrouter-backend
source "$HOME/.cargo/env"
cargo build --release

# Start server
./target/release/axumrouter

# Kill server
pkill -f "target/release/axumrouter$"

# Frontend
cd /root/.hermes/projects/axumrouter-frontend
npm run dev
```

## Architecture (post-refactor)
```
backend/src/
├── main.rs                  # Entry point
├── app.rs                   # Axum Router builder
├── state.rs                 # AppState
├── error.rs                 # GatewayError enum + HTTP mapping
├── api/
│   ├── mod.rs               # /v1/* routes (health, providers, chat, models)
│   └── chat/
│       ├── mod.rs            # Orchestrator (140 lines)
│       ├── streaming.rs      # handle_streaming()
│       ├── non_streaming.rs  # handle_non_streaming()
│       └── combo.rs          # handle_combo_request + stream variant
├── middleware/
│   └── auth.rs               # Bearer token auth + GatewayKeyInfo extensions
├── admin/
│   ├── mod.rs                # Admin router
│   ├── api.rs                # Per-group route builders (clean)
│   ├── auth_files.rs         # OAuth file management (rebuilt)
│   └── routes/               # Per-entity route handlers
│       ├── providers.rs      # List, detail, test, block/unblock
│       ├── gateway_keys.rs   # CRUD + access_type + max_tokens
│       ├── models.rs         # Global toggle + blocked list
│       ├── combos.rs         # Multi-tier combo management
│       ├── keys.rs           # Provider API key CRUD
│       ├── usage.rs          # Usage stats + per-key tracking
│       ├── quota.rs          # Usage quota + token refresh
│       ├── logs.rs           # Request logging
│       ├── database.rs       # DB info + export/import
│       └── settings.rs       # Global settings toggle
├── services/
│   ├── gateway.rs            # Model access + token limit + usage tracking
│   ├── tool_normalizer.rs    # Tool call normalization
│   └── usage_tracking.rs     # Token estimation
├── providers/
│   ├── manager.rs            # ProviderManager (RwLock)
│   ├── registry.rs           # Provider registration
│   ├── traits.rs             # Provider trait definition
│   ├── key_manager.rs        # Multi-key round-robin + cooldown lock
│   ├── spec.rs               # Provider metadata spec
│   ├── error_classifier.rs   # Error classification
│   ├── openai_compat/        # Generic OpenAI-compatible engine
│   │   ├── provider.rs       # OpenAICompatibleProvider (Arc config)
│   │   ├── client.rs         # HTTP client (Arc config)
│   │   ├── mapper.rs         # Request/response mapper (Arc config)
│   │   ├── config.rs         # Static config struct
│   │   ├── types.rs          # Chat types
│   │   └── auth.rs           # API key auth
│   ├── mistral/ (mst)        # Mistral AI
│   ├── opencode_free/ (ocf)  # OpenCode Free
│   ├── opencode_go/ (ocg)    # OpenCode Go
│   ├── freebuff/ (fb)        # FreeBuff (custom OAuth)
│   ├── xai/                  # xAI (custom OAuth)
│   ├── openai_codex/ (cx)    # OpenAI Cursor/Codex (OAuth)
│   ├── tbay/                 # TokenBay
│   └── cl/                   # Cline
├── types/                    # Chat, model, provider type definitions
├── config/                   # App config models
└── db/                       # SQLite: migrations, queries, usage log
```

## Key Features

### Gateway Keys
- Format: `axm-` + 124 random = 128 chars
- Access types: `full` (all models), `allow` (whitelist), `deny` (blacklist)
- Token limits: `max_tokens` = 0 (unlimited) or N (block at N tokens used)
- Layer 1: global model disable → Layer 2: per-key model access → Layer 3: token limit

### Combos
- Multi-tier sequential fallback
- `model: "combo/combo_name"` → iterate tiers until success
- Streaming + non-streaming supported

### Providers
- 8 providers: mst, ocg, ocf, fb, xai, cx, tbay, cl
- Pattern 1: `openai_compat` engine (mst, ocg, ocf, tbay, cl)
- Pattern 2: Full custom (fb, xai, cx) with OAuth

## Adding a New Provider
See `docs/ADD_PROVIDER.md` for full guide.

## DB Safety
- `data/axumrouter.db` — NEVER delete or drop
- Auto-backup: `data/backups/axumrouter.db.YYYYMMDD_HHmmss.bak`
- Reset: kill server, delete db, restart (auto-create fresh)

## Edit Protocol
1. Scan first — read all affected files
2. Edit one file at a time
3. `cargo build --release` after each file
4. Never batch unrelated changes
5. PATCH files >50 lines → use write_file for full rewrite

## Recent Refactors (2026-07-08)
- chat.rs → 4 modules (mod, streaming, non_streaming, combo)
- admin/api.rs → per-group route functions
- Settings.tsx → 3 components (Database, Models, GatewayKeys)
- Box::leak → Arc in openai_compat
- check_token_limit → single JOIN query
- TokenLimitExceeded error variant
- Shared ModelPickerModal component
- db/migrations.rs → proper tracing
- auth_files.rs → rebuilt (simplified OAuth flows)
