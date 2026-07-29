# AxumRouter Usage Tracking Guide — Cara Wiring Usage Tracking untuk Custom Provider

> **Goal**: Setiap request yang masuk ke engine/provider HARUS ke-record ke
> tabel `usage` dengan token counts + key attribution. Tanpa ini, stats di
> `/admin/usage` kosong.

## High-level flow

```
Chat request → api/chat/mod.rs dispatcher
  ├─ stream   → api/chat/streaming.rs   → record_success / record_error
  └─ non-str  → api/chat/non_streaming.rs → record_success / record_error
                       │
                       ▼
            state.usage_tracker.save(UsageEntry)
                       │
                       ▼
                  db/save_request_usage → INSERT INTO usage
```

Dispatcher (`streaming.rs` + `non_streaming.rs`) adalah **satu-satunya** tempat
yang nulis ke `usage`. Provider engine **cuma populate field di struct**:

- `ChatResult { used_key_id, failed_keys, response }`
- `ChatStreamResult { used_key_id, failed_keys, stream }`

Dispatcher yang extract token counts dari response, lalu call helper.

## Yang harus dilakukan provider engine

### 1. Track `used_key_id`

Setiap key yang coba → track attempt. Set `last_attempted_key_id` di tiap
loop iter + isi di struct return:

```rust
let mut last_attempted_key_id: Option<String> = None;

for _attempt in 0..total.max(1) {
    let key = match self.keys.next_excluding(None, &excluded) {
        Ok(k) => k,
        Err(_) => break,
    };
    let key_id = key.id.clone();
    last_attempted_key_id = Some(key_id.clone());  // ← wajib

    match self.client.chat_stream(&auth, &provider_req).await {
        Ok(resp) => {
            self.keys.mark_success(&key_id);
            return Ok(ChatStreamResult {
                stream: stream.boxed(),
                used_key_id: Some(key_id),                       // ← wajib
                failed_keys: failed,
                last_attempted_key_id,                            // ← wajib
            });
        }
        Err(e) => {
            // retryable → push ke failed, continue
            failed.push(FailedKeyAttempt {
                key_id: key_id.clone(),
                error: e,
            });
            last_attempted_key_id = Some(key_id.clone());          // ← wajib
            continue;
        }
    }
}

// Total failure → ProviderHttpError HARUS bawa key_id
Err(GatewayError::ProviderHttpError {
    status: last_status.unwrap_or(503),
    body: last_error.unwrap_or_default(),
    provider: self.config.provider_id.clone(),
    key_id: last_attempted_key_id,                                // ← wajib
})
```

### 2. Populate `failed_keys` per attempt

Setiap key yang auth-build gagal atau upstream return error (retryable)
HARUS di-push ke `failed_keys`:

```rust
let auth = match self.build_auth(&key) {
    Ok(a) => a,
    Err(e) => {
        self.keys.lock_key(&key_id, 400, e.to_string());
        excluded.push(key_id.clone());
        failed.push(FailedKeyAttempt {                          // ← wajib
            key_id: key_id.clone(),
            error: GatewayError::ProviderError(e.to_string()),
        });
        continue;
    }
};
```

Dispatcher akan iterate `failed_keys` → `record_error(...)` per key.

### 3. Surface usage dari stream chunks (SSE)

Provider yang stream **HARUS** populate field `usage` di `ChatCompletionChunk`
yang di-emit di akhir stream (OpenAI style) atau di event `MessageDelta`
(Anthropic style). Tanpa ini, `prompt_tokens`/`completion_tokens` di
row usage = 0.

**OpenAI**: pastikan request body ada `stream_options.include_usage = true`.
Mapper `openai_compat` udah auto-inject, custom provider gak perlu set manual.

**Anthropic**: selalu emit `usage` block di event `message_delta`.
Cek mapper di `engine/anthropic_compat/mapper.rs:334` — fill `usage` field.

## Yang TIDAK perlu dilakukan provider

- ❌ Call `state.usage_tracker.save(...)` langsung — dispatcher yang handle
- ❌ Bikin row INSERT manual — `UsageTracker::save` udah handle
- ❌ Track `prompt_tokens`/`completion_tokens` manual untuk stream — dispatcher
  pake `StreamRecorder` yang auto-accumulate dari chunk events
- ❌ Compute latency — `record_success` pake `Instant::elapsed()` otomatis

## Helper API yang tersedia

| Helper | Lokasi | Fungsi |
|---|---|---|
| `record_success(tracker, start, provider_id, model_id, gateway_key_id, provider_api_key_id, endpoint, usage, ttft_ms)` | `services/usage_tracking.rs` | Record sukses (stream + non-stream) |
| `record_error(tracker, start, provider_id, model_id, gateway_key_id, provider_api_key_id, endpoint, status_code, error_message)` | same | Record error row |
| `StreamRecorder::default()` + `record_chunk(started, prompt, completion)` | same | Accumulate token selama streaming |
| `ChatResult { response, used_key_id, failed_keys }` | `providers/result.rs` | Non-stream return struct |
| `ChatStreamResult { stream, used_key_id, failed_keys, last_attempted_key_id }` | same | Stream return struct |
| `FailedKeyAttempt { key_id, error }` | same | Per-attempt failure |

## Reference implementation

Lihat `engine/openai_compat/provider.rs:131-251` (`chat_completion_stream`)
dan `engine/anthropic_compat/provider.rs:137-251` — keduanya udah bener
semua field. Custom provider / OAuth (e.g. `providers/freebuff/provider.rs`)
harus mirror pattern yang sama.

## Verifikasi

Setelah wire, test dengan 3 skenario:

1. **Valid request** → row `usage` dengan `prompt_tokens > 0`,
   `completion_tokens > 0`, `provider_api_key_id = key_xxx`,
   `status = success`.
2. **Multi-key failover** (2 keys invalid, 1 valid) → 1 row `success`
   + 2 row `error` dengan `provider_api_key_id` per failed key.
3. **All keys invalid** → 1 row `error` dengan `provider_api_key_id`
   = key attempt terakhir (lo set via `last_attempted_key_id`).

Query cek:
```sql
SELECT id, status, prompt_tokens, completion_tokens,
       provider_api_key_id, error_message, created_at
FROM usage ORDER BY created_at DESC LIMIT 10;
```

Kalau ada row dengan `status = success` tapi `prompt_tokens = 0` →
`stream_options.include_usage` ga sampai ke upstream (cek mapper).

---

## Key Lock State Persistence (KeyManager DB pool)

> **Goal**: Lock state (cooldown 429, auth error, backoff) survive restart.
> Disimpan di kolom `api_keys` table: `locked_until`, `last_error_status`,
> `last_error_message`, `last_error_at`, `backoff_level`.

### Auto-wired (TOML providers — openai/anthropic engine)

TOML providers (`providers.toml`) — baik `api_type = "openai"` maupun
`api_type = "anthropic"` — **otomatis dapet DB pool** dari registry.
Gak perlu ngapa-ngapain. Lock/unlock ke DB jalan otomatis.

### Custom provider (manual register) — WAJIB pass pool

Kalo lo bikin provider pake `register_provider!` macro atau manual
`registry.register(...)` di `registry.rs`:

```rust
// ✅ BENAR — pool di-pass
registry.register("id-saya", |keys: Vec<ApiKey>, db: Arc<SqlitePool>| {
    let provider = MyProvider::new(
        config, keys,
        Some((*db).clone()),  // ← WAJIB: pass pool
    );
    Ok(Box::new(provider))
});
```

**WAJIB**:
1. Import `use sqlx::SqlitePool;` di file provider
2. Terima `db: Option<SqlitePool>` di constructor
3. Pass ke `KeyManager::new_with_pool(keys, &config.provider_id, db)`
4. `KeyManager::lock_key()` + `mark_success()` otomatis persist ke DB

**Contoh salah** (gak persist):
```rust
// ❌ SALAH — pool never passed, state gak survive restart
let provider = MyProvider::new(config, keys);
```

### DB columns (auto-migration)

```sql
-- Migration v10: dijalankan otomatis pas startup
ALTER TABLE api_keys ADD COLUMN locked_until TEXT;
ALTER TABLE api_keys ADD COLUMN last_error_status INTEGER;
ALTER TABLE api_keys ADD COLUMN last_error_message TEXT;
ALTER TABLE api_keys ADD COLUMN last_error_at TEXT;
ALTER TABLE api_keys ADD COLUMN backoff_level INTEGER NOT NULL DEFAULT 0;
```

### Lock lifecycle

```
Key sukses    → mark_success() → UPDATE locked_until=NULL,
                                   last_error_at=datetime('now'),
                                   backoff_level=0
Key error     → lock_key_for_model() → UPDATE locked_until=expiry,
                                         last_error_status=status,
                                         last_error_message=msg,
                                         last_error_at=datetime('now')
Restart       → kolom di DB tetap ada, tinggal di-load pas startup
                 (KeyManager::new() dari Vec<ApiKey> — future: load dari kolom)
```

### Catatan OAuth / complex provider

Provider OAuth (freebuff, dll) yg **tidak** pass pool → state cuma
in-memory. Hilang pas restart. Kalo mau persist, ikutin pattern di atas:
tambah `db: Option<SqlitePool>`, pass ke `KeyManager::new_with_pool()`.`
