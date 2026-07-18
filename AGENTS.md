# AGENTS.md — AxumRouter Project Rules

## Golden Rule
**Analisa dulu, edit setelah paham.** Sebelum ubah apapun, baca & pahami kode yang ada — struktur, aliran data, konteks.

## Philosophy
- **Clear** — readable > clever. Nama variable/fungsi jelas.
- **Clean** — DRY. Satu tanggung jawab per fungsi. No dead code.
- **Modern** — Rust edition 2021, fitur stabil terbaru.
- **Modular** — file < 500 baris. Gede? Pecah.
- **Maintainable** ��� kalo orang lain baca langsung ngerti.

## Before Any Edit
1. Baca struktur folder repo
2. Pahamin alur data: input → process → output
3. Identifikasi dependensi & side effects
4. Baru edit

## Conventions
- `cargo build --release` tiap abis edit
- Jangan leave commented code
- Jangan tambah dependency kalo stdlib/crate existing cukup
- Commit terpisah per logical change

## Project
OpenAI-compatible AI Gateway di Rust (Axum). Multi-provider router. 69+ providers (API Key, OAuth auth_code, OAuth device_code).

## Workspace
- Backend: `backend/` (Rust, Axum 0.7, port 7444)
- Frontend: `../frontend/` (React/TypeScript, Vite, port 5173)
- DB: `data/axumrouter.db` (SQLite)
- Binary: `target/release/axumrouter`

## Build & Run
```bash
# BE
cargo build --release && ./target/release/axumrouter

# FE (dev)
npm run dev

# FE (build)
npx vite build
```

## Architecture

### Backend
```
src/
├── main.rs, app.rs, state.rs, error.rs
├── config/         # App config (loader, models)
├── db/             # SQLite (migrations, models, queries)
├── types/          # Shared structs (chat, model, provider)
├── middleware/      # Auth + logging
├── api/            # /v1/* endpoints
│   └── chat/       # pipeline: normalize → RTK → caveman → route
├── engine/         # Generic provider engines
│   └── openai_compat/   # OpenAI-compatible engine (7 files)
├── admin/          # Admin dashboard
│   ├── api.rs      # thin router → routes/
│   ├── auth_files.rs # auth files CRUD
│   ├── routes/     # settings, providers, keys, gateway_keys, etc.
│   └── oauth/      # per-provider: cx.rs, xai.rs, fb.rs, np.rs
├── providers/      # 12 providers
└── services/       # Business logic
    ├── caveman.rs, gateway.rs, tool_normalizer.rs, usage_tracking.rs
    └── rtk/        # Real Token Killer (P1 split)
        ├── mod.rs
        └── filters/  # 11 files: git_diff, git_status, grep, find, etc.
```

### Frontend
```
src/
├── api/            # Modular API layer (11 files)
│   ├── client.ts   # fetcher + apiFetch helper
│   ├── types.ts, settings.ts, providers.ts, keys.ts, etc.
│   └── index.ts    # barrel re-export
├── hooks/          # useProviderDetail, useAsync
├── components/     # Reusable
│   ├── Modal.tsx           # P3: universal modal wrapper
│   ├── AuthFileCard.tsx    # P2: extracted from AuthFiles page
│   ├── OAuthConnectModal.tsx, GatewayKeysSection.tsx
│   ├── DatabaseSection.tsx, ModelsSection.tsx
│   ├── ModelPickerModal.tsx, FeatureRow.tsx
│   ├── Layout.tsx, Loading.tsx, ErrorBox.tsx
└── pages/          # Route pages
    ├── Endpoint.tsx, Settings.tsx, Providers.tsx
    ├── ProviderDetail.tsx  # P1: useProviderDetail hook
    ├── Playground.tsx, AuthFiles.tsx, Combos.tsx
    ├── Logs.tsx, Usage.tsx, Quota.tsx, ProxyPool.tsx
```

## Providers (12)

| ID  | Name              | Type     | Auth          | OAuth Flow      |
|-----|-------------------|----------|---------------|-----------------|
| mst | Mistral           | API Key  | Bearer        | — |
| ocg | OpenCode Go       | API Key  | Bearer        | — |
| ocf | OpenCode Free     | API Key  | Bearer        | — |
| tbay| TokenBay          | API Key  | Bearer        | — |
| nrak| Nous Research     | API Key  | Bearer        | — |
| xak | xAI API Key       | API Key  | Bearer        | — |
| cl  | Cline             | API Key  | X-Api-Key     | — |
| cf  | Cloudflare        | API Key  | Bearer        | — |
| cx  | OpenAI Codex      | OAuth    | Bearer        | authorization_code |
| xai | xAI               | OAuth    | Bearer        | authorization_code |
| fb  | FreeBuff          | OAuth    | Bearer        | device_code |
| np  | Nous Portal       | OAuth    | Bearer        | device_code + auto-refresh |

## Key Features
- **apiFetch**: global helper — all `fetch('/admin/...')` → `apiFetch('/...')` via VITE_API_BASE
- **Modal component**: universal `fixed inset-0 z-50` wrapper used in ProviderDetail, ModelPickerModal, Combos
- **useProviderDetail hook**: extracted all state + handlers from ProviderDetail page
- **AuthFileCard**: extracted card UI from AuthFiles page
- **OAuth meta**: `oauth_flow` field in provider metadata — FE auto-detects device_code vs auth_code
- **BE logging**: `RUST_LOG` env, `EnvFilter`, optional JSON format (`RUST_LOG_FORMAT=json`)
- **DB export/import**: real column names, excludes `request_logs` + `usage` only
- **Port config**: `AXUM_SERVER__PORT` via `.env` or `AXUM_` env vars

## Env Vars

### Backend (.env atau AXUM_ prefix)
```
AXUM_SERVER__PORT=7444
AXUM_DATABASE__URL=sqlite:data/axumrouter.db?mode=rwc
RUST_LOG=info
RUST_LOG_FORMAT=     # set to "json" for prod
```

### Frontend (.env)
```
VITE_API_BASE=http://152.42.198.51:3000/admin/api
VITE_GATEWAY_BACKEND_URL=http://152.42.198.51:3000
```

## Code Conventions
- `cargo build --release` after every Rust edit
- `tsc --noEmit` after every FE edit
- PATCH small sections, `write_file` for full rewrites
- Raw string `r#"..."#` for JSON in Rust
