# AxumRouter — Provider Architecture

> Cooldown, strategy, lock, custom models, new-provider checklist.  
> Read this before creating `src/providers/<new-folder>/`.

---

## 1. KeyManager: Strategy + Cooldown + Lock

Every provider owns a `KeyManager` — responsible for key selection, error cooldown, and auto-deactivation.

### 1.1 Strategy

Two built-in strategies, selected via `strategy_from_config()`:

| Strategy | Behavior |
|----------|----------|
| **FillFirst** (default) | Pick first available key (priority-sorted). Skip locked/excluded. |
| **RoundRobin** | Sticky round-robin: stay on current key for `sticky_limit` uses, then rotate to LRU. |

```rust
// Default (fill-first)
KeyManager::new_with_pool(keys, provider_id, db)

// Explicit
KeyManager::new_with_pool(keys, provider_id, db)
    .with_strategy(Box::new(RoundRobin::new(3))) // sticky limit 3
```

### 1.2 Error Classification

`error_classifier.rs` categorizes upstream errors into 4 buckets:

| Kind | HTTP codes | Text patterns | Retryable | Cooldown |
|------|-----------|---------------|-----------|----------|
| **Auth** | 401, 403 | `invalid api key`, `unauthorized`, `forbidden` | ✅ | 120s fixed |
| **RateLimit** | 429 | `rate limit`, `quota exceeded` | ✅ | 90s→300s exponential |
| **Transient** | 5xx, timeout, DNS | `overloaded`, `timeout`, `connection refused` | ✅ | 120s fixed |
| **Permanent** | 404, 410, 422 | — | ❌ | None (return immediately) |

### 1.3 Cooldown Config

```rust
KeyLockConfig {
    auth_cooldown_secs: 120,             // Auth errors: 2 min
    rate_limit_backoff_base: 90,         // Rate-limit: starts 90s, doubles each hit
    rate_limit_backoff_max: 300,         // Rate-limit: capped at 5 min
    transient_cooldown_secs: 120,        // Transient: 2 min
}
```

### 1.4 Lock Types

| Type | Scope | Trigger | KeyManager method |
|------|-------|---------|-------------------|
| **Account lock** | All models on key | Auth/rate-limit/transient error | `lock_key()` / `lock_key_for_model(key, None, ...)` |
| **Per-model lock** | Specific model only | Same error on a model | `lock_key_for_model(key, Some("model-id"), ...)` |
| **Auto-deactivate** (`is_active=0`) | Key disabled permanently | 3 consecutive account-level errors | Automatic inside `lock_key()` when `consecutive_error_count >= 3` |

### 1.5 DB Persistence

`KeyManager` supports DB-backed state via `new_with_pool()`. Without pool, locks are **in-memory only** (lost on restart).

```rust
// ✅ Persists locks to api_keys table — survive restart
KeyManager::new_with_pool(keys, provider_id, Some(pool))

// ❌ In-memory only — gone on restart
KeyManager::new(keys, provider_id)
```

DB columns auto-migrated (migration v10):
```
api_keys.locked_until          TEXT    — lock expiry ISO timestamp
api_keys.last_error_status     INTEGER — HTTP status of last error
api_keys.last_error_message    TEXT    — last error body
api_keys.last_error_at         TEXT    — timestamp of last error
api_keys.backoff_level         INTEGER — exponential backoff level
api_keys.consecutive_error_count INTEGER — for auto-deactivate
```

### 1.6 Test Endpoint Cooldown

`POST /admin/api/providers/:id/test` now returns lock state:

```json
{
  "ok": false,
  "error": "All 1 key(s) locked — cooldown active",
  "locked_keys": [
    {
      "key_id": "key_abc123",
      "remaining_secs": 119,
      "reason": "HTTP 429 — rate limit: (cooldown 90s, backoff_level=1, errors=1)"
    }
  ]
}
```

If all keys locked (`active_count == 0`), endpoint returns early — no wasted HTTP call.

---

## 2. Custom Models

### 2.1 Two Tables — Different Purposes

| Table | Purpose | Populated by | Relationship |
|-------|---------|-------------|--------------|
| `custom_provider_models` | Models defined when creating a custom provider in Admin → Custom Providers | Admin CRUD only | `provider_id` FK → `custom_providers(id)` |
| `custom_models` | User-added models per ANY provider (apikey, OAuth, custom) via Settings → Models | Admin CRUD or FE Settings page | No FK constraint |

### 2.2 How Models Flow

```
v1/models request
    │
    ▼
ProviderManager::list_all_models()
    ├─ provider.list_models()        ← static/TOML models
    └─ list_custom_models(db, name)  ← merge custom_models table
         │
         ▼
    format: "{prefix}/{model_id}"    ← dedup by id

Admin provider detail
    │
    ▼
provider.list_models()                ← MUST merge custom_models
```

### 2.3 OAuth Providers — Mandatory Merge

All providers that implement `list_models()` **MUST** merge `custom_models` from DB. Pattern:

```rust
async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
    let mut models = self.models_static();
    if let Some(pool) = &self.db {
        let custom = crate::db::list_custom_models(pool, &self.metadata.name).await;
        let prefix = self.metadata
            .model_prefix
            .clone()
            .unwrap_or_else(|| self.metadata.name.clone());
        for cm in custom {
            let id = format!("{}/{}", prefix, cm.model_id);
            if models.iter().any(|m| m.id == id) { continue; }
            models.push(Model {
                id,
                object: "model".to_string(),
                owned_by: self.metadata.display_name.clone(),
                context_length: Some(cm.ctx as u32),
            });
        }
    }
    Ok(models)
}
```

Reference: `OpenAICompatibleProvider::list_models()`, `FbProvider::list_models()`, `KlProvider::list_models()`.

### 2.4 Admin Endpoints for Custom Models

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `GET` | `/admin/api/providers/:id/custom-models` | List custom models for provider |
| `POST` | `/admin/api/providers/:id/custom-models` | Add custom model |
| `DELETE` | `/admin/api/providers/:id/custom-models/:model_id` | Remove custom model |

Settings → Models page: `GET /admin/api/models/all` → `POST /admin/api/models/toggle` (enable/disable via `disabled_models` table).

---

## 3. Provider Checklist — New `src/providers/<id>/` Folder

When coding a fundamentally different auth flow (OAuth, custom session lifecycle, etc.):

### 3.1 Required Files

```
src/providers/<id>/
├── mod.rs          ← re-export Provider struct
├── constants.rs    ← PROVIDER_ID, PROVIDER_NAME, MODELS, base URLs, timeouts
├── provider.rs     ← impl Provider trait (the big one)
├── auth.rs         ← credential struct + parsing
├── client.rs       ← HTTP client (reqwest wrapper)
└── mapper.rs       ← request/response mapping (if non-standard format)
```

### 3.2 Provider Trait — Mandatory Methods

| Method | Returns | Notes |
|--------|---------|-------|
| `metadata()` | `ProviderMetadata` | name, display_name, category, color, icon, oauth_flow |
| `chat_completion(req)` | `Result<ChatResult, GatewayError>` | Non-streaming. Must track `used_key_id` + `failed_keys`. |
| `chat_completion_stream(req)` | `Result<ChatStreamResult, GatewayError>` | SSE streaming. Must track `used_key_id` + `failed_keys` + `last_attempted_key_id`. |
| `list_models()` | `Result<Vec<Model>, GatewayError>` | **Must merge `custom_models` from DB** (see §2.3). |
| `health_check()` | `Result<bool, GatewayError>` | `active_count > 0` |
| `authenticate()` | `Result<(), GatewayError>` | Validate keys exist |
| `locked_keys()` | `Vec<(key_id, remaining_secs, reason)>` | Expose lock state for FE |
| `total_keys()` | `usize` | KeyManager count |
| `active_keys()` | `usize` | Unlocked keys count |

### 3.3 Wiring Checklist

- [ ] Store `db: Option<SqlitePool>` in provider struct
- [ ] Pass pool to constructor + `KeyManager::new_with_pool()`
- [ ] `list_models()` merges `custom_models` from DB
- [ ] `chat_completion()` tracks `used_key_id`, `failed_keys`, `last_attempted_key_id`
- [ ] Error path calls `keys.lock_key()` / `lock_key_on_error()`
- [ ] Success path calls `keys.mark_success()`
- [ ] Register in `registry.rs` via `register_provider!()` macro
- [ ] Add icon to `frontend/public/providers/<id>.png`

### 3.4 KeyManager Usage in chat_completion

```rust
for _attempt in 0..total.max(1) {
    let key = self.keys.next_excluding(None, &excluded)?;
    let key_id = key.id.clone();
    last_attempted_key_id = Some(key_id.clone());

    match self.client.send(&auth, &body).await {
        Ok(resp) => {
            self.keys.mark_success(&key_id);
            return Ok(ChatResult {
                response, used_key_id: Some(key_id), failed_keys: failed,
            });
        }
        Err(e) => {
            let c = lock_key_on_error(&self.keys, &key_id, &e);
            let msg = e.to_string();
            if c.retryable {
                failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                excluded.push(key_id);
                continue;
            }
            return Err(e);
        }
    }
}
// All keys exhausted:
Err(GatewayError::ProviderHttpError {
    status: last_status.unwrap_or(503),
    body: last_error.unwrap_or_default(),
    provider: self.metadata.name.clone(),
    key_id: last_attempted_key_id,  // ← WAJIB
})
```

---

## 4. DB Schema — All Tables

```
api_keys               ← provider keys + OAuth credentials + lock state
gateway_keys           ← client-facing auth keys
custom_providers       ← admin-defined custom providers (base_url, timeout)
custom_provider_models ← models per custom provider
custom_models          ← user-added models per any provider (Settings → Models)
blocked_models         ← per-provider model denial
disabled_models        ← global model blocklist
proxies                ← proxy pool
settings               ← key-value toggles (rtk_enabled, caveman_enabled)
usage                  ← per-request token tracking
```

---

## 5. Error Reference for Providers

| Error variant | When | Contains |
|---------------|------|----------|
| `ProviderHttpError` | Upstream returned HTTP error | `status`, `body`, `provider`, `key_id` |
| `ProviderError` | Auth build failed, parse error | message only |
| `NoAvailableKeys` | All keys locked or exhausted | message |
| `GatewayError` | Catch-all | Varies |

Dispatcher (`api/chat/*`) extracts `key_id` from error for usage tracking via `provider_api_key_id`.
