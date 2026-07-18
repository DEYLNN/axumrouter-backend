# AxumRouter Provider Guide — Cara Tambah Provider Baru

> **TOML-based system.** Gak perlu buat folder Rust. Gak perlu register manual.

## Cara Tambah Provider Baru (Simple API Key)

Buka `providers.toml` di root backend, tambah blok:

```toml
[[providers]]
id = "xxx"                          # ID unik, jadi prefix model (xxx/model-name)
name = "Xxx AI"                     # Display name di UI
category = "apikey"                 # apikey | oauth
color = "#6366F1"                   # Warna card UI
icon = "xxx.png"                    # Icon file di frontend/public/providers/
base_url = "https://api.xxx.com"    # Base URL API
validate_url = "https://api.xxx.com/v1/models"  # Endpoint validasi auth
api_type = "openai"                 # openai | anthropic — pilih engine
docs_url = "https://xxx.com"        # Optional: link docs
api_key_url = "https://xxx.com/keys" # Optional: link halaman API key
timeout = 120                       # Optional: default 120
first_chunk_timeout = 200           # Optional: default 200
stall_timeout = 360                 # Optional: default 360

# Optional: quirks kalo provider punya edge case
[providers.quirks]
# auth_header = "x-api-key"         # default: bearer
# max_tokens_field = "max_completion_tokens"  # default: max_tokens
# drop_stream_options = true
# force_temperature = 0.7

# Models — tambahin sesuai kebutuhan
[[providers.models]]
id = "deepseek-v4-flash"           # Model ID upstream (full, gak dipotong)
name = "DeepSeek V4 Flash"         # Display name
ctx = 1000000                      # Context window (metadata only, gak dikirim ke API)
vision = false
tools = true

[[providers.models]]
id = "gpt-5.6"                     # Model kedua
name = "GPT 5.6"
ctx = 131072
vision = false
tools = true
```

### Step

1. Edit `backend/providers.toml` — tambah blok `[[providers]]`
2. (Kalo ada icon) — copy icon ke `frontend/public/providers/xxx.png`
3. `cargo check` — verifikasi compile
4. Restart BE

**Gak perlu edit Rust code. Gak perlu register manual. Gak perlu tambah folder.**

## Cara Hapus Provider

1. Hapus blok `[[providers]]` dari `providers.toml`
2. `cargo check`
3. Restart BE

## Cara Edit Provider

1. Edit field di `providers.toml`
2. `cargo check`
3. Restart BE

## Provider Custom (OAuth / Complex Logic)

Untuk provider yang butuh OAuth atau logika khusus (cf, fb, kc, np, cx, xai):
- Tetap pake folder `backend/src/providers/<id>/`
- Register manual di `backend/src/providers/registry.rs`
- Lihat contoh di folder `cloudflare/`, `freebuff/`, dll

## Arsitektur

```
providers.toml (58 provider) → ProviderRegistry::new()
                                    ├── api_type="openai"    → OpenAICompatibleProvider
                                    └── api_type="anthropic" → AnthropicCompatibleProvider
custom modules (6)              → manual register_provider!()
```

## Model ID Convention

- FE display: `provider_id/upstream_model_id` (contoh: `ocg/deepseek-v4-flash`)
- API request: strip `provider_id/` → kirim upstream ID as-is
- `ctx` = metadata only — gak pernah dikirim ke API
- Gak ada stripping/penambahan prefix selain `provider_id/`