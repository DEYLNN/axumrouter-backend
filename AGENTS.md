# AGENTS.md — AxumRouter Backend

## Project
AxumRouter — OpenAI-compatible API Gateway di Rust (Axum).
**2 providers** (OAuth device_code + API-key) using 2 engines:
- `openai_compat` — generic OpenAI-compatible (handles all current providers)
- `anthropic_compat` — reserved for future Anthropic-format providers

Single `/v1/chat/completions` endpoint. No `/v1/responses` (reserved stub deleted).

## Workspace
- Backend: `backend/` (Rust, Axum 0.7, port **7444**)
- Frontend: `frontend/` (React/TypeScript, Vite, port 5173)
- DB: `data/dev.db` (SQLite) — **SATU-SATUNYA DB yg boleh disentuh**
- Binary: `target/release/axumrouter`
- Config: `config/config.toml` (admin_password & jwt_secret in `.env`, not in TOML)

## Build & Run
```bash
# BE
cargo build --release
cargo run --release

# FE (dev)
cd ../frontend && npm run dev

# FE (build)
cd ../frontend && npx vite build

# Run bookend: never `pkill axumrouter` (matches prod binary too).
lsof -i :7444 -t | xargs kill   # stop dev PID only
```

## Architecture

### Backend
```
src/
├── main.rs, app.rs, state.rs, error.rs
├── config/         # App config (loader, models)
├── db/             # SQLite (migrations, models, queries, helpers)
├── types/          # Shared structs (chat, model, provider)
├── middleware/     # Auth + logging
├── api/            # /v1/* endpoints (public-facing)
│   ├── chat/       # streaming + non-streaming chat completions
│   ├── models.rs   # /v1/models listing
│   ├── mod.rs      # /v1/health, /v1/providers, router wiring
│   └── responses.rs DELETED — was a reserved-for-future stub
├── engine/         # Generic engines
│   ├── openai_compat/   # OpenAI-compatible engine (7 files)
│   └── anthropic_compat/ # Anthropic-compatible engine (7 files)
├── admin/          # Admin dashboard (auth + CRUD routes)
│   ├── login.rs
│   └── routes/      # split per concern:
│       ├── database.rs, gateway_keys.rs, keys.rs, models.rs
│       ├── providers/  ← split into list/detail/validate/test/block
│       ├── custom_providers.rs, settings.rs, usage.rs
│       └── mod.rs (barrel)
├── providers/      # **2 active providers**
│   ├── freebuff/   # fb — OAuth device_code
│   ├── openai_codex/ # cx — OAuth auth_code (behind ocf custom URL)
│   ├── manager.rs, registry.rs, key_manager.rs
│   ├── traits.rs, spec.rs, toml_provider.rs
│   ├── error_classifier.rs, result.rs
│   └── mod.rs
└── services/       # Business logic, provider-agnostic
    ├── caveman.rs, gateway.rs, tool_normalizer.rs
    ├── usage_tracking.rs ← UsageTracker (provider-agnostic save)
    ├── ponytail.rs       # Deliberate simplifications tracker
    ├── rtk_filters.rs    # RTK entry point
    └── rtk/             # Real Token Killer (12 filter files)
```

### Active providers

| ID  | Name          | Type     | Auth          |
|-----|---------------|----------|---------------|
| fb  | FreeBuff      | OAuth    | Bearer        | device_code flow
| ocf | OpenCode Free | API key  | Bearer        | (custom OX platform, OAuth auth_code)

> `ocf` is a custom alias pointing to a user-defined OpenAI-compatible
> endpoint. It is the actively-used provider in dev.
> Custom OAuth (auth_code) and custom API-key providers are wired through
> `custom_providers` admin CRUD (no new `src/providers/` folder needed).

## DB schema (SQLite)
```
api_keys            (provider api keys, multiplexed for failover)
gateway_keys        (admin-issued client keys, label/access_type/allowed_models/max_tokens)
custom_providers    (user-defined base_url/validate_url/timeout)
custom_provider_models (per-custom-provider model definitions)
blocked_models      (per-provider model denial)
disabled_models     (global disabled, insertion auto-runs on add-key)
proxies             (proxy pool — not yet exposed in UI)
settings            (rtk_enabled, caveman_enabled, future toggles)
usage               ← NEW: per-request token tracking (created 2026-07-28)
```

## Usage tracking (provider-agnostic)
Every engine — and every future custom provider — must call:
```rust
state.usage_tracker.save(UsageEntry { … }).await;
```
once per upstream request. Helpers:
- `UsageTracker::save(entry)` — backend-agnostic, never breaks the request
- `StreamRecorder::record_chunk(prompt, completion)` — accumulate SSE tokens

Admin endpoints:
- `GET /admin/api/usage/stats` — totals (requests, prompt/completion tokens)
- `GET /admin/api/usage/keys` — per-gateway-key breakdown
- `GET /admin/api/logs?page=1&limit=50` — paginated recent

## Env Vars (AXUM_ prefix, double underscore = nested)
```
AXUM_SERVER__HOST=0.0.0.0
AXUM_SERVER__PORT=7444
AXUM_DATABASE__URL=sqlite:data/dev.db?mode=rwc
AXUM_AUTH__ADMIN_PASSWORD=admin
AXUM_AUTH__JWT_SECRET=dev-secret-axumrouter-2026
RUST_LOG=info
```

## Code Conventions
- `cargo build` after every Rust edit
- File < 500 baris — split before exceeding
- No commented code
- Edit-only-this-folder — production is off-limits
- Custom providers built via `admin/api/custom-providers` CRUD; do not
  hardcode new `src/providers/<name>/` folders unless wiring a fundamentally
  different auth flow (e.g. PKCE, device_code with a quirk).

## Docs (`backend/docs/`)

- `GUIDE.md` — workspace overview (start here).
- `API_KEY_PROVIDER_GUIDE.md` — adding a provider via `providers.toml`
  (TOML-driven, no Rust folder needed for plain API-key flows).
- `USAGE_TRACKING.md` — **required reading before touching any engine /
  provider / OAuth flow**. Covers `used_key_id`, `failed_keys`,
  `last_attempted_key_id`, `include_usage`, and what the dispatcher
  (`api/chat/streaming.rs` + `non_streaming.rs`) does automatically so
  the custom provider does not.
- `PROVIDER_ARCHITECTURE.md` — cooldown, strategy, lock, custom models,
  provider trait checklist. Read before creating `src/providers/<new>/`.
- `IMPORT_SCHEMA.md` — JSON import key validation schema.
- `provider_templates/apikey/*.rs.txt` — copy-paste starters for new
  custom API-key providers (when TOML is not enough).
