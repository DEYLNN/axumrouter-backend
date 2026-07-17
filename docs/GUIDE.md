# AxumRouter — Backend Guide

## Overview

OpenAI-compatible AI Gateway. Rust (Axum 0.7), SQLite, multi-provider.

Routes LLM requests to 69+ providers — API Key & OAuth. Admin dashboard (SPA) di `/admin/*`.

---

## System Requirements

| Requirement | Minimal | Recomended |
|------------|---------|-----------|
| Rust | 1.75+ | 1.81+ |
| Disk | 200MB | 1GB |
| RAM | 256MB | 512MB |
| SQLite | bundled (sqlx) | — |

### Dependencies (Cargo.toml)
- **axum 0.7** — HTTP framework
- **sqlx 0.8** (SQLite) — database
- **reqwest 0.12** — HTTP client ke LLM providers
- **tower-http** — CORS, logging, static files
- **config** — config loader (TOML + env vars)
- **jsonwebtoken** — JWT auth
- **dotenvy** — .env loader
- **tracing** — logging (text atau JSON)

---

## Config

### Priority (high → low)
1. `AXUM_` env vars (e.g. `AXUM_SERVER__PORT=4000`)
2. `config/config.toml`

### `config/config.toml` (wajib ada)

```toml
[server]
host = "0.0.0.0"
port = 3000
# public_url = "http://your-vps:3000"    # Optional: override auto-detect

[gateway]
timeout_secs = 120

[auth]
api_key_header = "Authorization"
api_key_prefix = "Bearer "

[database]
url = "sqlite:data/axumrouter.db?mode=rwc"
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AXUM_SERVER__HOST` | `0.0.0.0` | Listen address |
| `AXUM_SERVER__PORT` | `3000` | Listen port |
| `AXUM_SERVER__PUBLIC_URL` | auto-detect | Override public URL |
| `AXUM_GATEWAY__TIMEOUT_SECS` | `120` | LLM request timeout |
| `AXUM_DATABASE__URL` | `sqlite:data/axumrouter.db?mode=rwc` | DB path |
| `AXUM_AUTH__ADMIN_PASSWORD` | — | Admin login password |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |
| `RUST_LOG_FORMAT` | — | Set `json` for structured JSON logging |

### `.env` (optional)
Dibaca otomatis via dotenvy di `main.rs`. Bisa taruh env vars di sini.

---

## Setup

Ada 2 cara config — pilih salah satu:

### Opsi A: Pakai `.env` (recommended)
```bash
# 1. Clone
cd ~/.hermes/projects/axumrouter/backend

# 2. Copy .env
cp .env.example .env
# Edit sesuai kebutuhan — semua env udah terisi default dev
# nano .env

# 3. Build
cargo build --release

# 4. Run (otomatis baca .env)
./target/release/axumrouter
```

### Opsi B: Pakai `config/config.toml`
```bash
# 1. Clone
cd ~/.hermes/projects/axumrouter/backend

# 2. Copy config
cp config/config.example.toml config/config.toml

# 3. Build
cargo build --release

# 4. Run
./target/release/axumrouter
```

### Env Reference

Semua env pake prefix `AXUM_` — format `AXUM_<SECTION>__<FIELD>` (double underscore = nested).

```
# ── Server ──────────────────────────────────
AXUM_SERVER__HOST=0.0.0.0
AXUM_SERVER__PORT=7444
AXUM_SERVER__PUBLIC_URL=http://vps-ip:7444   # Optional

# ── Database ────────────────────────────────
AXUM_DATABASE__URL=sqlite:data/dev.db?mode=rwc

# ── Gateway ─────────────────────────────────
AXUM_GATEWAY__TIMEOUT_SECS=120

# ── Auth ────────────────────────────────────
AXUM_AUTH__ADMIN_PASSWORD=admin
AXUM_AUTH__JWT_SECRET=dev-secret-axumrouter-2026

# ── Logging ─────────────────────────────────
RUST_LOG=info
RUST_LOG_FORMAT=json    # Production
```

| Variable | Wajib? | Default | Description |
|----------|--------|---------|-------------|
| `AXUM_SERVER__HOST` | ✅ | `0.0.0.0` | Listen address |
| `AXUM_SERVER__PORT` | ✅ | `3000` | Listen port |
| `AXUM_DATABASE__URL` | ✅ | `sqlite:data/axumrouter.db?mode=rwc` | DB path |
| `AXUM_GATEWAY__TIMEOUT_SECS` | ✅ | `120` | LLM request timeout |
| `AXUM_AUTH__ADMIN_PASSWORD` | ⚠️ | none | Admin login. Kosong = login disabled |
| `AXUM_AUTH__JWT_SECRET` | ⚠️ | insecure default | Ganti di production! |
| `AXUM_SERVER__PUBLIC_URL` | ❌ | auto-detect | Override public URL |
| `RUST_LOG` | ❌ | `info` | Log level: debug, info, warn, error |
| `RUST_LOG_FORMAT` | ❌ | plain | `json` buat structured logging |

### Priority (high → low)
1. `AXUM_` env vars
2. `config/config.toml`

---

## Run Modes

### Development
```bash
cargo run --release
# atau hot-reload (install cargo-watch dulu)
cargo watch -x run
```

### Production (VPS)
```bash
# Via systemd
nohup ./target/release/axumrouter > server.log 2>&1 &

# Via pm2 (kalo pake pm2)
pm2 start ./target/release/axumrouter --name axumrouter

# Cek log
tail -f server.log
```

### Port Custom
```bash
AXUM_SERVER__PORT=4000 ./target/release/axumrouter
```

---

## Project Structure

```
backend/
├── Cargo.toml
├── config/
│   ├── config.example.toml      ← template config
│   └── config.toml              ← actual config (gitignore)
├── src/
│   ├── main.rs                  ← entry: init tracing, load config, db, serve
│   ├── app.rs                   ← router builder (health, /v1/*, /admin/*, SPA)
│   ├── state.rs                 ← AppState: config, db pool, provider_manager
│   ├── error.rs                 ← GatewayError enum (OpenAI-compat error format)
│   │
│   ├── config/
│   │   ├── loader.rs            ← load config.toml + AXUM_ env vars
│   │   └── models.rs            ← AppConfig structs (server, gateway, auth, db)
│   │
│   ├── db/
│   │   ├── mod.rs               ← init, backup, query helpers
│   │   ├── migrations.rs        ← auto-migrate on startup
│   │   └── models.rs            ← DB row structs
│   │
│   ├── api/
│   │   ├── mod.rs               ← /v1/* routes: health, models, chat, providers
│   │   ├── chat/                ← chat completion pipeline
│   │   ├── models.rs            ← /v1/models
│   │   └── responses.rs
│   │
│   ├── admin/
│   │   ├── api.rs               ← /admin/api/* route groups
│   │   ├── login.rs             ← admin auth
│   │   ├── auth_files.rs        ← OAuth credential files
│   │   ├── oauth/               ← per-provider OAuth flows
│   │   └── routes/              ← handlers: providers, keys, logs, usage, dll
│   │
│   ├── providers/
│   │   ├── registry.rs          ← 69+ provider registrations
│   │   ├── manager.rs           ← provider lifecycle
│   │   ├── traits.rs            ← Provider trait
│   │   ├── spec.rs              ← ProviderSpec, ProviderMetadata
│   │   ├── key_manager.rs       ← key failover, cooldown
│   │   ├── result.rs            ← ProviderResult
│   │   ├── error_classifier.rs  ← categorize errors
│   │   └── <id>/                ← per-provider: constants, provider, etc.
│   │
│   ├── engine/
│   │   └── openai_compat/       ← generic OpenAI-compat engine (7 files)
│   │
│   ├── services/
│   │   ├── gateway.rs           ← gateway key logic
│   │   ├── caveman.rs           ← fallback logic
│   │   ├── tool_normalizer.rs   ← tool call formatting
│   │   ├── usage_tracking.rs    ← token counting
│   │   └── rtk/                 ← Real Token Killer (filters)
│   │
│   ├── middleware/
│   │   ├── auth.rs             ← Bearer token + gateway key validation
│   │   └── logging.rs          ← request/response logging
│   │
│   ├── types/                   ← shared structs
│   │   ├── chat.rs             ← ChatCompletionRequest/Response/Chunk
│   │   ├── model.rs            ← Model types
│   │   └── provider.rs         ← ProviderMetadata
│   │
│   └── utils/                   ← helpers (public IP detect, dll)
��
├── public/
│   ├── providers/               ← provider icons (png)
│   ��── admin/                   ← frontend build (SPA)
│
└── docs/
    ├── GUIDE.md                 ← this file
    ├── API_KEY_PROVIDER_GUIDE.md �� cara nambah provider baru
    └── provider_templates/      ← template files
```

---

## Database (SQLite)

### Auto-backup
Setiap startup, DB di-backup ke `data/backups/`. Keep 20 backup terakhir.

### Tables

| Table | Description |
|-------|-------------|
| `api_keys` | Provider API keys (+ OAuth credentials) |
| `gateway_keys` | Client-facing auth keys buat /v1/* |
| `usage` | Token usage logs |
| `request_logs` | Request history |
| `blocked_models` | Per-provider blocked models |
| `disabled_models` | Global model blocklist |
| `settings` | Key-value settings (rtk_enabled, caveman_enabled) |
| `proxies` | Proxy pool |
| `combos` | Provider combo routing |

### Commands

```bash
# Reset DB (hapus dan start ulang)
rm -f data/axumrouter.db && ./target/release/axumrouter

# Backup manual
cp data/axumrouter.db data/axumrouter.db.backup

# Via admin API
curl http://localhost:3000/admin/api/database/export
```

---

## API Endpoints

### Public (OpenAI-compatible)

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| GET | `/health` | No | Health check |
| GET | `/v1/models` | Gateway Key | List models (format: `provider_id/model`) |
| POST | `/v1/chat/completions` | Gateway Key | Chat completion (stream & non-stream) |
| GET | `/v1/providers` | Gateway Key | List providers + status |

### Admin API (internal)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/admin/api/providers` | List all providers |
| GET | `/admin/api/providers/:id` | Provider detail + models |
| POST | `/admin/api/providers/:id/test` | Test model |
| POST | `/admin/api/providers/:id/block` | Block model |
| POST | `/admin/api/providers/:id/unblock` | Unblock model |
| POST | `/admin/api/keys` | Add API key |
| POST | `/admin/api/keys/delete` | Delete API key |
| GET | `/admin/api/logs` | Request logs |
| POST | `/admin/api/logs/clear` | Clear logs |
| GET | `/admin/api/usage/stats` | Usage statistics |
| GET | `/admin/api/usage/stats/keys` | Per-key usage |
| GET | `/admin/api/usage/keys` | OAuth key usage |
| GET | `/admin/api/usage/quota/:key_id` | Quota info |
| POST | `/admin/api/usage/refresh/:key_id` | Refresh OAuth token |
| GET | `/admin/api/settings` | List settings |
| POST | `/admin/api/settings/toggle` | Toggle setting |
| GET | `/admin/api/database` | DB info |
| GET | `/admin/api/database/export` | Export DB (json) |
| POST | `/admin/api/database/import` | Import DB |
| GET/POST/DELETE | `/admin/api/gateway_keys` | Gateway key CRUD |
| GET/POST/DELETE | `/admin/api/combos` | Combo routing CRUD |
| GET | `/admin/api/models/disabled` | Disabled models |
| GET | `/admin/api/models/all` | All models |
| GET | `/admin/api/models/blocked` | Blocked models |

### Model Format
```
POST /v1/chat/completions
{"model": "mst/mistral-large", "messages": [...]}
```
Format: `{provider_id}/{model_name}`

---

## Providers (69 registered)

### 13 Core (Active)
| ID | Name | Type | Auth |
|----|------|------|------|
| mst | Mistral | API Key | Bearer |
| ocg | OpenCode Go | API Key | Bearer |
| ocf | OpenCode Free | API Key | Bearer |
| tbay | TokenBay | API Key | Bearer |
| nak | Nous API Key | API Key | Bearer |
| cl | Cline | API Key | X-Api-Key |
| cf | Cloudflare | API Key | Bearer |
| fb | FreeBuff | Custom | device_code |
| mcf | MiMo Code Free | Custom | JWT bootstrap |
| np | Nous Portal | OAuth | device_code + auto-refresh |
| cx | OpenAI Codex | OAuth | authorization_code |
| xai | xAI Grok | OAuth | authorization_code |
| xak | xAI API Key | API Key | Bearer |

Plus ~56 OpenAI-compat API Key providers (zyloo, tokenrouter, pollinations, groq, openrouter, etc.)

### Menambah Provider Baru
Lihat `docs/API_KEY_PROVIDER_GUIDE.md` — 2 pattern:
1. **OpenAI-compat (API Key)** — 3 file + register, ~15 menit
2. **Full Custom (OAuth)** — 5 file + register

---

## Admin Login

- Default: no password (akses bebas dari localhost)
- Set `AXUM_AUTH__ADMIN_PASSWORD` kalo perlu proteksi
- SPA accessible di `http://localhost:3000/admin/`

---

## Logging

### Standard
```bash
RUST_LOG=info ./target/release/axumrouter
```

### JSON (production)
```bash
RUST_LOG_FORMAT=json RUST_LOG=info ./target/release/axumrouter
```

### Debug
```bash
RUST_LOG=debug ./target/release/axumrouter 2>&1 | head -100
```

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| `config/config.toml` not found | Belum copy config | `cp config/config.example.toml config/config.toml` |
| Port 3000 already in use | Service lain | `lsof -i :3000` + kill, atau ganti `AXUM_SERVER__PORT` |
| `error: failed to run custom build command` | sqlx offline | `cargo install sqlx-cli && cargo build` |
| DB corrupt | Crash saat write | Hapus `data/axumrouter.db`, backup ada di `data/backups/` |
| `Provider not found` | Provider ID salah | Cek `/admin/api/providers` valid IDs |
| `No active keys` | API key belum ditambah | Tambah via admin UI atau `/admin/api/keys` |
| Model blocked | Admin block | Cek `/admin/api/providers/:id` blocked list |
| SPA blank | Frontend belum di-build | `cd ../frontend && npm install && npx vite build` |

---

## VPS Deployment

```bash
# Systemd service file
cat > /etc/systemd/system/axumrouter.service << 'EOF'
[Unit]
Description=AxumRouter AI Gateway
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/.hermes/projects/axumrouter/backend
ExecStart=/root/.hermes/projects/axumrouter/backend/target/release/axumrouter
Restart=on-failure
Environment=RUST_LOG=info
Environment=RUST_LOG_FORMAT=json

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable axumrouter
systemctl start axumrouter
systemctl status axumrouter

# Nginx reverse proxy (kalo ada domain)
# server {
#     listen 80;
#     server_name axm-api.dkzhen.org;
#     location / {
#         proxy_pass http://127.0.0.1:3000;
#         proxy_set_header Host $host;
#         proxy_set_header X-Real-IP $remote_addr;
#     }
# }
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, init tracing, load config, db, serve |
| `src/app.rs` | Router builder — all route mounts |
| `src/state.rs` | AppState shared state |
| `src/config/loader.rs` | Config loader (TOML + env) |
| `src/providers/registry.rs` | All provider registrations |
| `src/db/migrations.rs` | Auto-schema + seed data |
| `docs/API_KEY_PROVIDER_GUIDE.md` | Provider addition guide |

---

## Related

| Resource | Path |
|----------|------|
| Backend AGENTS.md | `AGENTS.md` |
| Frontend | `../frontend/` |
| Frontend AGENTS.md | `../frontend/AGENTS.md` |
| Provider template | `docs/provider_templates/apikey/` |
