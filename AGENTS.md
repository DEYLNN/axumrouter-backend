# AGENTS.md — AxumRouter Project Rules

## Project
AxumRouter — OpenAI-compatible API Gateway di Rust (Axum).
Multi-provider router dengan OAuth, API Key, device code flow.

## Workspace
- Root: `/root/.hermes/projects/axumrouter-backend`
- DB: `data/axumrouter.db` (SQLite)
- Binary: `target/release/axumrouter`
- Port: 3000

## Build & Run
```bash
cargo build --release
strip target/release/axumrouter
./target/release/axumrouter
kill $(pgrep -f "axumrouter$")
```

## DB safety
- `data/axumrouter.db` jangan pernah di-rm atau di-drop
- Ada auto-backup di `data/backups/` tiap startup
- Reset: matikan server, hapus file, start ulang (auto-bikin baru)

## Architecture

### Backend (Rust/Axum)
```
src/
├── main.rs                # entry, startup
├── app.rs                 # Axum Router builder
├── state.rs               # AppState (db, provider_manager)
├── error.rs               # GatewayError enum
├── config/                # App config (loader, models)
├── db/                    # SQLite (migrations, models, queries)
├── types/                 # Shared structs (chat, model, provider)
├── utils/                 # Helpers
├── middleware/             # Auth + logging
│
├── api/                   # /v1/* endpoints
│   ├── mod.rs             # routes
│   ├── models.rs          # /v1/models
│   ├── responses.rs       # response helpers
│   └── chat/              # /v1/chat/completions
│       ├── mod.rs         # pipeline: normalize → RTK → caveman → route
│       ├── combo.rs       # multi-model routing
│       ├── non_streaming.rs
│       └── streaming.rs
│
├── engine/                # Generic provider engines
│   └── openai_compat/     # OpenAI-compatible provider engine
│       ├── mod.rs, provider.rs, client.rs, auth.rs
│       ├── config.rs, mapper.rs, types.rs
│
├── admin/                 # Admin dashboard API
│   ├── mod.rs
│   ├── api.rs             # thin router → delegates to routes/
│   ├── auth_files.rs      # auth files CRUD
│   ├── routes/            # route handlers
│   │   ├── mod.rs, combos.rs, database.rs, gateway_keys.rs
│   │   ├── keys.rs, logs.rs, models.rs, providers.rs
│   │   ├── quota.rs, settings.rs, usage.rs
│   └── oauth/             # OAuth per-provider handlers
│       ├── mod.rs         # router
│       ├── cx.rs, xai.rs, fb.rs, np.rs
│
├── providers/             # All provider implementations
│   ├── mod.rs, registry.rs, manager.rs, traits.rs
│   ├── key_manager.rs, result.rs, spec.rs
│   ├── error_classifier.rs
│   │
│   ├── mistral/           # API key (openai_compat)
│   ├── opencode_free/     # API key (openai_compat)
│   ├── opencode_go/       # API key (openai_compat)
│   ├── tokenbay/          # API key (openai_compat)
│   ├── xai_api_key/       # API key (openai_compat)
│   ├── nous_api_key/      # API key (openai_compat)
│   ├── cline/             # API key (openai_compat)
│   │
│   ├── mimo_code_free/    # JWT bootstrap, custom auth
│   ├── cloudflare/        # Multi-account, custom auth
│   │
│   ├── xai/               # OAuth authorization_code
│   ├── openai_codex/      # OAuth authorization_code
│   ├── freebuff/          # OAuth device_code
│   └── nous_portal/       # OAuth device_code + auto-refresh
│
└── services/              # Business logic layer
    ├── mod.rs
    ├── caveman.rs         # terse system prompt
    ├── rtk.rs             # Real Token Killer (tool compress)
    ├── rtk_filters.rs     # 11 compression filters
    ├── tool_normalizer.rs # tool message normalization
    ├── gateway.rs         # gateway key validation
    └── usage_tracking.rs  # usage logging
```

### Frontend (React/TypeScript)
```
src/
├── main.tsx, App.tsx
├── api/                  # Modular API layer
│   ├── index.ts          # barrel re-export
│   ├── client.ts         # fetcher helper
│   ├── types.ts          # shared interfaces
│   ├── settings.ts, providers.ts, keys.ts
│   ├── gateway.ts, oauth.ts, usage.ts
│   ├── database.ts, auth-files.ts
│
├── hooks/                # Custom hooks
│   ├── useOAuthFlow.ts
│   └── useAsync.ts
│
├── components/           # Reusable components
│   ├── Layout.tsx, Loading.tsx, ErrorBox.tsx
│   ├── OAuthConnectModal.tsx   # universal OAuth modal
│   ├── GatewayKeysSection.tsx, ModelsSection.tsx
│   ├── DatabaseSection.tsx, FeatureRow.tsx
│   ├── ModelPickerModal.tsx
│
└── pages/                # Route pages
    ├── Endpoint.tsx, Settings.tsx, Providers.tsx
    ├── ProviderDetail.tsx, Playground.tsx
    ├── AuthFiles.tsx, Combos.tsx, Logs.tsx
    ├── Usage.tsx, Quota.tsx, ProxyPool.tsx
```

## Providers (15)

| ID  | Name              | Type     | Auth          | Flow            |
|-----|-------------------|----------|---------------|-----------------|
| mst | Mistral           | API Key  | Bearer        | —               |
| ocg | OpenCode Go       | API Key  | Bearer        | —               |
| ocf | OpenCode Free     | API Key  | Bearer        | —               |
| tbay| TokenBay          | API Key  | Bearer        | —               |
| nrak| Nous Research     | API Key  | Bearer        | —               |
| xak | xAI API Key       | API Key  | Bearer        | —               |
| cl  | Cline             | API Key  | X-Api-Key     | —               |
| mcf | MiMo Code Free    | API Key  | JWT bootstrap  | —               |
| cf  | Cloudflare        | API Key  | Bearer        | —               |
| cx  | OpenAI Codex      | OAuth    | Bearer        | authorization_code |
| xai | xAI               | OAuth    | Bearer        | authorization_code |
| fb  | FreeBuff          | OAuth    | Bearer        | device_code     |
| np  | Nous Portal       | OAuth    | Bearer        | device_code     |

## OAuth Flow Types
- `authorization_code`: cx, xai — redirect URL + code exchange
- `device_code`: fb, np — device activation URL + auto-poll

Each provider implements OAuth in `admin/oauth/<id>.rs` + `providers/<id>/oauth.rs`.

## API Layer (FE)
Modular per-domain — `src/api/` folder with 11 files. Import via `'../api'` barrel.

## Code Conventions
- Raw string `r#"..."#` buat JSON inline di Rust
- Tiap error path WAJIB di-log
- PATCH sections, never full rewrite
- `cargo build --release` after every Rust edit

## Key Rules
1. ❌ Jangan rewrite `data/axumrouter.db`
2. ❌ Jangan rm binary `target/release/axumrouter`
3. ✅ Build (`cargo check` atau `cargo build --release`) sebelum push
4. ✅ `write_file` untuk file >50 lines (patch tool rawan corruption)
