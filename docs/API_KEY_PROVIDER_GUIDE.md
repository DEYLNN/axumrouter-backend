# AxumRouter Provider Guide — Cara Tambah Provider Baru

Target: tambah provider baru ke AxumRouter.

Ada 2 pattern:
1. **OpenAI-compatible (API Key)** — recommended, paling cepat
2. **Full Custom (OAuth / special logic)** — untuk provider yang butuh auth custom

---

## Pattern 1: OpenAI-compatible (API Key) — Recommended

Provider yang pakai API key standar (Authorization: Bearer xxx) dan API endpoint OpenAI-compatible.

### Buat folder baru

```bash
mkdir -p src/providers/<id>/
```

### File yang wajib dibuat (3 file)

#### 1. `src/providers/<id>/mod.rs`

```rust
mod constants;
mod provider;

pub use provider::Provider;
```

#### 2. `src/providers/<id>/constants.rs`

Provider ID (nama folder), nama display, warna, icon, dan `provider_spec()`:

```rust
pub const PROVIDER_ID: &str = "xxx";
pub const PROVIDER_NAME: &str = "XXX Name";
pub const COLOR: &str = "#6366F1";
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

#### 3. `src/providers/<id>/provider.rs`

Delegate ke `openai_compat` generic engine — nggak perlu nulis HTTP client:

```rust
use crate::providers::openai_compat::{OpenAICompatProvider, OpenAICompatConfig};
use crate::providers::traits::Provider;

use super::constants;

pub struct Provider {
    inner: OpenAICompatProvider,
}

impl Provider {
    pub fn new() -> Self {
        let config = OpenAICompatConfig {
            base_url: "https://api.xxx.com/v1",
            api_key_header: "Authorization",  // bisa "x-api-key" kalo beda
            supports_tools: true,
        };
        Self {
            inner: OpenAICompatProvider::new(config, constants::provider_spec()),
        }
    }
}

#[async_trait::async_trait]
impl Provider for Provider {
    fn metadata(&self) -> crate::providers::spec::ProviderSpec {
        self.inner.metadata()
    }

    async fn list_models(&self) -> Result<Vec<crate::types::model::Model>, crate::error::GatewayError> {
        self.inner.list_models().await
    }

    async fn chat_completion(
        &self,
        request: crate::types::chat::ChatCompletionRequest,
    ) -> Result<crate::providers::result::ProviderResult, crate::error::GatewayError> {
        self.inner.chat_completion(request).await
    }
}
```

### Register (edit 2 file)

#### 4. `src/providers/mod.rs`

Tambah baris:

```rust
pub mod xxx;
```

#### 5. `src/providers/registry.rs`

Panggil `register_openai_compat()`:

```rust
register_openai_compat(xxx::constants::PROVIDER_ID, xxx::constants::provider_spec());
```

Baris ini letakkan di dalam fungsi `register_all()` setelah provider lain.

### 6. Icon (optional tapi recommended)

Taruh file PNG 64x64:

```
backend/public/providers/<id>.png
```

Kalo nggak ada icon, UI bakal pake fallback inisial provider ID.

### 7. Build & Restart

```bash
cargo build --release
pkill axumrouter
./target/release/axumrouter
```

### Selesai

Provider otomatis muncul di:
- `/admin/api/providers` (daftar semua provider)
- `/admin/api/providers/<id>` (detail + models + test button)
- `/admin/api/providers/<id>/test` (test model)

Tambah key via admin UI → langsung aktif.

---

## Pattern 2: Full Custom (OAuth / special logic)

Untuk provider yang butuh OAuth, auth custom, atau logika khusus (contoh: `xai/`, `openai_codex/`, `freebuff/`).

### File yang dibuat (5 file)

```
src/providers/<id>/
├── mod.rs         → declare submodules
├── constants.rs   → sama kaya Pattern 1 (PROVIDER_ID, COLOR, provider_spec())
├── auth.rs        → token exchange, refresh token logic
├── client.rs      → HTTP client + streaming handler
├── provider.rs    → impl Provider trait
```

### `mod.rs`

```rust
pub mod auth;
pub mod client;
pub mod constants;
pub mod provider;

pub use provider::Provider;
```

### `constants.rs`

Sama persis kaya Pattern 1 di atas.

### `auth.rs`

Implementasi credential + token refresh:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XxxOAuthCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub email: Option<String>,
}

impl XxxOAuthCredential {
    pub fn is_expired(&self) -> bool {
        // cek expires_at vs sekarang
        true
    }

    pub async fn refresh(&mut self) -> Result<(), String> {
        // POST ke OAuth token endpoint
        // update access_token, expires_at
        Ok(())
    }
}
```

### `client.rs`

HTTP client buat streaming chat:

```rust
use crate::error::GatewayError;

pub struct XxxClient {
    http: reqwest::Client,
}

impl XxxClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn send_stream(
        &self,
        body: serde_json::Value,
        cred: &super::auth::XxxOAuthCredential,
    ) -> Result<futures::stream::BoxStream<'static, Result<crate::types::chat::ChatCompletionChunk, GatewayError>>, GatewayError> {
        // HTTP request + SSE parsing
        unimplemented!()
    }
}
```

### `provider.rs`

Implementasi `Provider` trait dari awal:

```rust
use crate::providers::traits::Provider;
use super::constants;

pub struct Provider {
    client: super::client::XxxClient,
}

impl Provider {
    pub fn new() -> Self {
        Self {
            client: super::client::XxxClient::new(),
        }
    }
}

#[async_trait::async_trait]
impl Provider for Provider {
    fn metadata(&self) -> crate::providers::spec::ProviderSpec {
        constants::provider_spec()
    }

    async fn list_models(&self) -> Result<Vec<crate::types::model::Model>, crate::error::GatewayError> {
        // return hardcoded models atau fetch dari API
        Ok(vec![])
    }

    async fn chat_completion(
        &self,
        request: crate::types::chat::ChatCompletionRequest,
    ) -> Result<crate::providers::result::ProviderResult, crate::error::GatewayError> {
        // ambil key dari KeyManager
        // call client.send_stream()
        // parse response
        unimplemented!()
    }
}
```

Register dan icon sama kaya Pattern 1 (step 4-6).

### Jika nanti butuh WHAM/rate limit endpoint khusus

Bikin file `usage.rs` di folder provider, sama kaya `openai_codex/usage.rs`:

```rust
// src/providers/<id>/usage.rs
pub async fn fetch_usage(token: &str) -> (Vec<serde_json::Value>, Option<String>) {
    // panggil endpoint rate limit provider
    // parse response
    // return (rate_limits, plan_type)
}
```

Lalu di `admin/routes/quota.rs` tinggal manggil via:

```rust
if provider_id == "<id>" {
    providers::<id>::usage::fetch_usage(token).await
}
```

---

## Contoh Provider yang sudah ada

| Provider | Pattern | File |
|----------|---------|------|
| mst (Mistral) | 1 (openai_compat) | `src/providers/mistral/` |
| ocg (Nexrouter) | 1 (openai_compat) | `src/providers/ocg/` |
| ocf (OpenCode Free) | 1 (openai_compat) | `src/providers/ocf/` |
| xai (xAI Grok) | 2 (OAuth) | `src/providers/xai/` |
| cx (Codex) | 2 (OAuth) | `src/providers/openai_codex/` |
| fb (FreeBuff) | 2 (session-based) | `src/providers/freebuff/` |

---

## Arsitektur Metadata

```
constants.rs → provider_spec() → ProviderMetadata → /admin/api/providers → React UI
```

Setiap provider cukup define `provider_spec()` di constants. Backend auto:
1. Register ke `/admin/api/providers`
2. Kirim color + icon_url ke React
3. Tampil di card + sidebar nanti

Nggak perlu hardcode di frontend.
