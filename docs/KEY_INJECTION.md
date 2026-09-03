# KEY_INJECTION — Bulk API Key Management

Base URL: `{BASE}/admin/api`

Admin token via `POST /admin/api/login` with `{"password":"PASSWORD"}`.

---

## 1. Bulk Add Keys

**POST** `/admin/api/keys/bulk-add`

```bash
curl -X POST {BASE}/admin/api/keys/bulk-add \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"sop","keys":["sk-xxx","sk-yyy"],"label":"optional"}'
```

**Response:**
```json
{"added":2,"duplicates":0,"total":2,"message":"2 added, 0 duplicates skipped"}
```

**Guards:**
- `keys` array must be non-empty
- `provider_id` must exist and be category `apikey`
- Non-apikey or unknown provider → `{"error":"..."}`
- Duplicates silently skipped

---

## 2. Count Keys

**GET** `/admin/api/keys/count?provider_id=sop`

```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "{BASE}/admin/api/keys/count?provider_id=sop"
```

**Response:**
```json
{"provider_id":"sop","total":5,"active":5,"disabled":0}
```

---

## 3. Delete Keys

**POST** `/admin/api/keys/bulk-delete`

### Delete all keys for a provider:
```bash
curl -X POST {BASE}/admin/api/keys/bulk-delete \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"sop","action":"all"}'
```

### Delete by key_value:
```bash
curl -X POST {BASE}/admin/api/keys/bulk-delete \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"sop","action":"by_key","key_value":"sk-xxx"}'
```

**Response:**
```json
{"deleted":2,"message":"Deleted 2 key(s) from sop"}
```

**Guards:**
- `provider_id` must exist and be category `apikey`
- `action` must be `"all"` or `"by_key"` (with `key_value`)
- Invalid action → `{"error":"invalid action"}`
- Reloads provider after deletion if keys were removed