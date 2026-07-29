# Import Key JSON — Validasi & Schema

> **File**: `frontend/src/api/import-schemas.ts`
> Validasi jalan di FE sebelum kirim ke BE.

## Format JSON

Single object atau array of objects:

```json
{
  "provider_id": "ocf",
  "key_type": "apikey",
  "key_value": "sk-xxxxxxxxxxxxxxxx",
  "label": "my-key-1"
}
```

```json
[
  { "provider_id": "ocf", "key_type": "apikey", "key_value": "sk-xxx", "label": "key-1" },
  { "provider_id": "fb",  "key_type": "oauth",  "key_value": "{}",  "email": "user@x.com" }
]
```

## Schema per Provider

### API Key (ocf, sfp, mmx, ...)

| Field | Wajib | Catatan |
|-------|-------|---------|
| `provider_id` | ✅ | Harus terdaftar di system |
| `key_type` | ✅ | `"apikey"` |
| `key_value` | ✅ | Min 1 karakter |
| `label` | ❌ | Opsional |

### Cloudflare (cf)

| Field | Wajib | Catatan |
|-------|-------|---------|
| `provider_id` | ✅ | `"cf"` |
| `key_type` | ✅ | `"apikey"` |
| `key_value` | ✅ | Token biasa atau JSON `{"apiKey":"...","accountId":"..."}` |
| `label` | ❌ | Opsional |

### OAuth (fb)

| Field | Wajib | Catatan |
|-------|-------|---------|
| `provider_id` | ✅ | `"fb"` |
| `key_type` | ✅ | `"oauth"` |
| `email` | ✅ | Email akun |
| `access_token` | ✅ | Token akses |
| `refresh_token` | ❌ | Token refresh (kalo ada) |
| `expires_in` | ✅ | Masa berlaku (detik) |

## Cara Nambah Schema Baru

Buka `frontend/src/api/import-schemas.ts`, tambah entry:

```ts
'provider_id:key_type': [
  { key: 'field_name', label: 'Label', required: true, minLength: 1, allowed: ['val1', 'val2'] },
  { key: 'field_name', label: 'Label', required: false },
],
```

Reuse schema yang udah ada pake alias string:

```ts
'sfp:apikey': 'ocf:apikey',  // reuse field rules dari ocf:apikey
```

## Validasi Output

Import sukses: `✅ Imported 3 file(s)`

Import dengan error:
```
❌ Imported 1, skipped 2: Item ocf: Missing 'API Key' (min 1 char); Provider 'xxx' not found in system (+1 more)
```
