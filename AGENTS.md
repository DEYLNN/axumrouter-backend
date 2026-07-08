# AGENTS.md — AxumRouter Project Rules
# Dibaca otomatis setiap agent menyentuh project ini.

## Project
AxumRouter — OpenAI-compatible API Gateway di Rust (Axum).
Provider saat ini: Mistral (mst). Multi-key round-robin, failover, cooldown lock.

## Workspace
- Root: `/root/.hermes/projects/axumrouter-backend`
- DB: `data/axumrouter.db` (SQLite)
- Binary: `target/release/axumrouter`
- Port: 3000

## Build & Run
```bash
cargo build --release
strip target/release/axumrouter
# Start:
./target/release/axumrouter
# Kill:
kill $(pgrep -f "axumrouter$")
```

## DB safety
- `data/axumrouter.db` jangan pernah di-rm atau di-drop
- Ada auto-backup di `data/backups/` tiap startup
- Kalo mau reset: matikan server, hapus file, lalu start ulang (auto-bikin baru)

## Architecture
```
src/
├── main.rs              # entry, startup
├── app.rs               # Axum Router builder
├── state.rs             # AppState
├── error.rs             # GatewayError enum, IntoResponse
├── api/
│   ├── mod.rs           # /v1/* routes (chat, models, health, providers)
│   └── chat.rs          # /v1/chat/completions handler
├── middleware/
│   └── auth.rs          # Bearer ***  (cuma untuk /admin/* bypass)
├── admin/
│   ├── ui.rs            # Admin dashboard HTMX
│   ├── settings.rs      # CONFIG + GATEWAY_KEYS
│   ├── keys.rs           # Provider API keys CRUD
│   ├── gateway_keys.rs  # Gateway keys CRUD
│   ├── logs.rs          # Usage logs
│   ├── providers.rs     # Provider detail + model management
│   └── routing.rs       # Routing table
├── providers/
│   ├── manager.rs       # ProviderManager (RwLock)
│   ├── registry.rs      # Provider registry
│   ├── traits.rs        # Provider trait
│   ├── key_manager.rs   # Multi-key, cooldown lock
│   └── mistral/
│       ├── provider.rs  # Mistral implementation
│       ├── client.rs    # HTTP client
│       ├── auth.rs      # Auth handler
│       └── constants.rs # Model list
├── types/               # Chat request/response, model, provider structs
├── config/              # App config models
└── db/                  # SQLite migrations, model queries, log queries
```

## Code Conventions
- Gas darat pake **raw string** `r#"..."#` buat HTML di `format!()` — hindari backslash escape hell
- Tiap error path WAJIB di-`log_usage()` — ga boleh silent fail
- `drop(pm); return Err(...)` pattern pas mau return error sambil pegang `provider_manager` lock
- SVG icons inline di HTML (no external deps)
- UI stack: **HTMX** + inline JS. No React/Vue. Tiap action pake `<form>` atau fetch + reload.

## Edit Protocol (mandatory)

Before any code change — feature, fix, or refactor — follow this sequence:

1. **Scan first.** Read all files that may need editing. Understand current state before touching anything.
2. **Edit one file at a time.** Each edit must be complete and correct — no partial stubs, no "will finish later".
3. **Verify after each file.** `cargo build --release` before moving to the next file.
4. **Report after every step.** After each file edit or completed cycle, send a structured report:

   ```
   📝 Edit summary
   File: src/admin/providers.rs
   What: Replaced model list text buttons with SVG icons (TEST, COPY, X)
   Why: UI consistency — mobile-friendly icon-only controls
   Side effects: None. Existing onClick handlers unchanged.
   ```

   This makes rollbacks and error tracking trivial. If something breaks, the report tells you exactly where and what to revert.

5. **Never batch unrelated changes.** A single edit = one file + one logical change. No mixed concerns.

## Key Rules untuk Agent
1. **Jangan pernah rewrite `data/axumrouter.db`** — itu data produksi
2. **Jangan pernah rm binari target/release/axumrouter** — build lama
3. **Jangan ubah SOUL.md / AGENTS.md** si agent itu sendiri tanpa izin operator
4. **Sebelum edit file Rust → cargo check/build dulu**
5. **Kalo stuck di Rust escaping → ganti ke raw string** `r#"..."#`
6. **Patch tool sering corruption >800 chars** — pake `write_file` full rewrite buat file >50 lines

## Adding a New Provider — Quick Reference (2026-07)

**Paling cepat & recommended**: pakai `openai_compat` generic engine (sudah dipakai mst/ocg/ocf).

Metadata pipeline (auto):
```
constants.rs → provider_spec() → ProviderMetadata → /admin/api/providers → React
```

### Pattern 1: OpenAI-compatible (API Key) — Recommended

Buat folder baru: `src/providers/<id>/`

**File yang wajib dibuat (3 file):**

1. `src/providers/<id>/mod.rs`
   ```rust
   mod constants;
   mod provider;
   pub use provider::Provider;
   ```

2. `src/providers/<id>/constants.rs`
   ```rust
   pub const PROVIDER_ID: &str = "xxx";
   pub const PROVIDER_NAME: &str = "XXX Name";
   pub const COLOR: &str = "#hex";
   pub const ICON_URL: &str = "/public/providers/xxx.png";
   pub const CATEGORY: &str = "llm";

   pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
       crate::providers::spec::ProviderSpec {
           id: PROVIDER_ID,
           name: PROVIDER_NAME,
           color: COLOR,
           icon_url: ICON_URL,
           category: CATEGORY,
       }
   }
   ```

3. `src/providers/<id>/provider.rs`
   ```rust
   use super::constants;
   use crate::providers::openai_compat::{OpenAICompatProvider, OpenAICompatConfig};

   pub struct Provider {
       inner: OpenAICompatProvider,
   }

   impl Provider {
       pub fn new() -> Self {
           let config = OpenAICompatConfig {
               base_url: "https://api.xxx.com/v1",
               api_key_header: "Authorization",
               supports_tools: true,
           };
           Self { inner: OpenAICompatProvider::new(config, constants::provider_spec()) }
       }
   }

   // delegate trait impls to inner...
   ```

**Register (2 file edit):**

- `src/providers/mod.rs` → `pub mod <id>;`
- `src/providers/registry.rs` → `register_openai_compat("<id>", constants::provider_spec());`

**Icon**: taruh `backend/public/providers/<id>.png` (atau .webp)

### Pattern 2: Full Custom (OAuth / special logic)

Contoh: `xai/`, `openai_codex/`, `freebuff/`

**File yang dibuat (4-5 file):**

- `constants.rs` (sama seperti di atas)
- `auth.rs` (OAuth token exchange + refresh)
- `client.rs` (HTTP + streaming)
- `provider.rs` (impl `Provider` trait + chat_completion)
- `mod.rs`

Register sama seperti Pattern 1.

### Checklist Lengkap

| Step | File | Action |
|------|------|--------|
| 1 | `src/providers/<id>/constants.rs` | ID, name, color, icon_url, `provider_spec()` |
| 2 | `src/providers/<id>/mod.rs` | declare submodules |
| 3 | `src/providers/<id>/provider.rs` | impl Provider trait (atau delegate ke openai_compat) |
| 4 | `src/providers/mod.rs` | `pub mod <id>;` |
| 5 | `src/providers/registry.rs` | register provider |
| 6 | `backend/public/providers/<id>.png` | icon (64x64 recommended) |
| 7 | `cargo build --release` | verify |

**Template lengkap**: lihat `src/providers/openai_compat/` sebagai contoh Pattern 1.

**Setelah register**:
- Provider otomatis muncul di `/admin/api/providers`
- Bisa test via `/admin/api/providers/<id>/test`
- Bisa tambah key via admin UI
- Metadata (color + icon) langsung ke React tanpa hardcode
