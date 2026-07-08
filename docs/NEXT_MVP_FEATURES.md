# AxumRouter — Next MVP Features

Purpose: daftar fitur next setelah Phase 1 provider stability. Eksekusi hanya kalau operator perintah.

Status current:

```txt
✅ API-key provider guide
✅ API-key provider template
✅ ProviderSpec + ProviderQuirks
✅ ErrorClassifier modular text rules
✅ KeyManager lock/cooldown/failover
✅ runtime provider reload after key add/delete
✅ buffered SSE parser
✅ stream first-chunk/stall timeout
✅ stream token usage logging
✅ request detail inspector in /admin/logs
✅ tool-call normalizer basic
✅ Mistral provider stable baseline
```

---

## P0 — Best next move

### 1. Provider generator script

File target:

```txt
scripts/new_apikey_provider.py
```

Function:

```txt
Generate src/providers/<id>/ from docs/provider_templates/apikey/*.txt
Patch src/providers/mod.rs
Patch src/providers/registry.rs
Print next manual TODO
```

Input:

```txt
provider_id
provider_name
provider_full_name
base_url
validate_url
auth_header: bearer | x-api-key
supports_streaming
supports_tools
supports_vision
```

Command target:

```bash
python scripts/new_apikey_provider.py \
  --id opr \
  --name OpenRouter \
  --full-name openrouter \
  --base-url https://openrouter.ai/api \
  --validate-url https://openrouter.ai/api/v1/models \
  --auth bearer
```

Impact:

```txt
Add provider jadi 1 command + edit model list.
Reduce copy-paste bugs.
```

Risk:

```txt
Registry patch bisa salah kalau format berubah.
Need backup before patch.
```

Acceptance:

```txt
[ ] generates provider folder
[ ] no overwrite unless --force
[ ] patches mod.rs
[ ] patches registry.rs
[ ] cargo build --release passes
```

---

### 2. ProviderSpec-driven admin UI

Files likely:

```txt
src/admin/providers.rs
src/types/provider.rs
src/providers/traits.rs
```

Function:

```txt
Show ProviderSpec fields in provider detail page.
Show quirks badges:
- auth: bearer/x-api-key
- stream usage on/off
- tools on/off
- max token field
- drop stream_options/tools/tool_choice
```

Impact:

```txt
Operator tahu provider behavior dari UI.
Debug provider lebih cepat.
```

Acceptance:

```txt
[ ] provider detail shows base_url/validate_url
[ ] provider detail shows quirks badges
[ ] no secret key leak
```

---

### 3. Dynamic provider reload after key add/delete ✅ DONE

Current status:

```txt
DONE — add/delete key calls ProviderManager::reload_provider(provider_id).
No restart required.
```

Files likely:

```txt
src/admin/keys.rs
src/providers/manager.rs
src/state.rs
```

Function:

```txt
After add/delete provider key:
- acquire provider_manager write lock
- call reload_provider(provider_id)
```

Impact:

```txt
No restart needed after adding keys.
Better UX.
```

Acceptance:

```txt
[x] add key from UI
[x] immediately usable without restart
[x] delete key removes from runtime without restart
```

---

## P1 — Stability improvements

### 4. Request detail inspector ✅ DONE

Reference 9router:

```txt
/tmp/9router/open-sse/utils/requestLogger.js
/tmp/9router/open-sse/handlers/chatCore/requestDetail.js
```

Files likely:

```txt
src/db/migrations.rs
src/db/models.rs
src/admin/logs.rs
src/api/chat.rs
```

Function:

```txt
Store per request:
- request_body
- response_body
- provider id
- key id
- error
- latency
- tokens
- stream final chunk
```

UI:

```txt
/admin/logs expandable REQUEST_DETAIL row
```

Impact:

```txt
Debug provider errors easier.
Can inspect 400/422 body.
```

Risk:

```txt
Sensitive data in request logs.
Need masking/truncation toggle.
```

Acceptance:

```txt
[x] expandable log row opens detail
[x] request/response rendered pretty JSON
[x] API keys masked basic markers
[x] body truncated safely
```

---

### 5. Error classifier config table ✅ PARTIAL DONE

Current status:

```txt
PARTIAL DONE — classifier now uses modular TextRule list in src/providers/error_classifier.rs.
Still not DB/provider-spec configurable yet.
```

Next:

```txt
Optional later: make rules configurable per provider/spec if many providers need overrides.
```

Rules:

```txt
text contains rate limit => 429 lock
text contains quota exceeded => 429 lock
text contains overloaded => transient
status 401/403 => auth lock
status 429 => rate limit lock
status 5xx => transient retry/lock
```

Impact:

```txt
New provider weird errors easier to tune.
```

Acceptance:

```txt
[x] classifier supports text rules
[ ] provider-specific override possible
[ ] logs show classified kind
```

---

### 6. Stream timeout config ✅ DONE

Reference 9router:

```txt
STREAM_FIRST_CHUNK_TIMEOUT_MS = 200000
STREAM_STALL_TIMEOUT_MS = 360000
FETCH_CONNECT_TIMEOUT_MS = 60000
```

Current status:

```txt
DONE — connect timeout + first chunk timeout + stall timeout implemented for Mistral.
```

Function:

```txt
Implement:
- connect timeout
- first chunk timeout
- stall timeout between chunks
```

Impact:

```txt
No infinite hanging streams.
Cleaner stream error when upstream stalls.
```

Acceptance:

```txt
[x] first chunk timeout triggers clean error
[x] stall timeout triggers clean error
[x] long healthy stream not killed by total timeout
```

---

## P1.5 — Performance validation

### 7. Benchmark/load test harness

File target:

```txt
scripts/bench_chat.py
```

Function:

```txt
Run controlled /v1/chat/completions load tests.
Measure RPS, p50, p95, p99, error rate, timeout count, token/s.
Support non-stream and stream mode.
```

Command target:

```bash
python scripts/bench_chat.py   --url http://localhost:3000/v1/chat/completions   --model mst/mistral-small-latest   --concurrency 25   --requests 250   --stream false
```

Impact:

```txt
Know real gateway capacity before adding providers/fallback.
Find bottleneck: SQLite logging, upstream latency, reqwest pool, stream holding, lock contention.
```

Acceptance:

```txt
[ ] reports RPS
[ ] reports p50/p95/p99 latency
[ ] reports error rate by status/code
[ ] supports stream and non-stream
[ ] safe default request body
```

---

## P2 — Provider scale

### 8. Dynamic model discovery

Reference 9router:

```txt
/tmp/9router/open-sse/services/model.js
/tmp/9router/open-sse/config/providerModels.js
```

Function:

```txt
Provider can fetch /models from upstream.
Cache models in DB.
Admin UI refresh button.
```

Files likely:

```txt
src/db/migrations.rs
src/providers/traits.rs
src/admin/providers.rs
src/api/models.rs
```

Impact:

```txt
No hardcode model list for OpenRouter/Together/Fireworks.
```

Acceptance:

```txt
[ ] provider detail has REFRESH MODELS
[ ] models stored/cached
[ ] /v1/models includes discovered models
```

---

### 9. Capability map + routing prep

Reference 9router:

```txt
/tmp/9router/open-sse/providers/capabilities.js
/tmp/9router/open-sse/services/combo.js
```

Function:

```txt
Track model capabilities:
- tools
- vision
- context_window
- streaming
- reasoning
```

Impact:

```txt
Foundation for auto routing / fallback model.
```

Acceptance:

```txt
[ ] capabilities stored per model
[ ] UI shows badges
[ ] route validation checks tools/vision support
```

---

### 10. Combo/fallback model routing

Reference 9router:

```txt
/tmp/9router/open-sse/services/combo.js
```

Function:

```txt
Virtual model maps to multiple real models.
Example:
auto/fast:
  mst/mistral-small-latest
  groq/llama...
  opr/openai/gpt-4o-mini
```

Impact:

```txt
If one provider down/rate-limited, fallback to another.
```

Acceptance:

```txt
[ ] routing table UI supports virtual model
[ ] fallback on 429/5xx
[ ] logs show fallback path
```

---

## P3 — Agentic upgrade

### 11. Full tool-call normalization

Reference 9router:

```txt
/tmp/9router/open-sse/translator/concerns/toolCall.js
/tmp/9router/open-sse/utils/toolDeduper.js
```

Current:

```txt
Basic id sanitize/generate + content None
```

Next:

```txt
- arguments object -> JSON string
- missing tool response insertion
- duplicate tool call dedupe
- streaming tool_call assembly
```

Impact:

```txt
Better compatibility with agent clients.
```

Acceptance:

```txt
[ ] object arguments converted to string
[ ] missing tool result optionally fixed
[ ] duplicate tool_calls deduped
[ ] streaming tool_calls valid
```

---

### 12. OpenAI Responses API

Reference 9router:

```txt
/tmp/9router/open-sse/handlers/responsesHandler.js
/tmp/9router/open-sse/transformer/responsesTransformer.js
```

Endpoint:

```txt
/v1/responses
```

Impact:

```txt
Better compatibility with newer OpenAI agent SDKs.
```

Acceptance:

```txt
[ ] /v1/responses non-stream
[ ] /v1/responses stream
[ ] maps to chat providers internally
```

---

## Suggested execution order — updated after stability pass

Completed:

```txt
✅ Dynamic reload after key add/delete
✅ Request detail inspector
✅ Modular ErrorClassifier text rules
✅ Stream timeout config
```

Recommended next order:

```txt
1. Benchmark/load test harness
2. ProviderSpec-driven admin UI
3. Dynamic model discovery
4. Capability map + route validation
5. Combo/fallback model routing
6. Provider generator script
7. Full tool-call normalization
8. OpenAI Responses API
```

Why this order:

```txt
Benchmark first: know real RPS, p95, p99, error rate before adding more provider complexity.
ProviderSpec UI next: operator sees quirks/timeouts/capabilities while debugging.
Dynamic models + capability map before fallback: fallback needs accurate model metadata.
Provider generator after standards stable: templates should copy final runtime pattern, not old code.
```

---

## Notes

Do not implement all at once.

Recommended next session command:

```txt
bantu bikin benchmark/load test harness buat AxumRouter
```

or:

```txt
bantu implement ProviderSpec-driven admin UI
```

