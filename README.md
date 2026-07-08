# AxumRouter

Lightweight AI Gateway built with **Rust + Axum**.

## Philosophy
- **Provider-first** — each provider is fully isolated
- **Plugin architecture** — add a provider by adding a folder
- **Core agnostic** — gateway never knows provider internals
- **OpenAI Compatible API**

## Quick Start

```bash
cargo build --release
cargo run
```

## Config

Edit `config/config.toml` to set your provider API keys.

## API

| Endpoint | Method | Description |
|---|---|---|
| `/v1/chat/completions` | POST | Chat completion |
| `/v1/models` | GET | List available models |
| `/health` | GET | Health check |

## Adding a Provider

1. Create `src/providers/<name>/`
2. Implement the `Provider` trait
3. Register in `src/providers/registry.rs`

```
providers/
└── openai/
    ├── mod.rs
    ├── provider.rs
    ├── client.rs
    ├── auth.rs
    ├── models.rs
    ├── mapper.rs
    ├── constants.rs
    ├── request.rs
    ├── response.rs
    ├── config.rs
    └── error.rs
```

## Project Structure

```
src/
├── main.rs            # Entry point
├── app.rs             # Router builder
├── state.rs           # Shared application state
├── config/            # Config loading + models
├── routes/            # HTTP route definitions
├── handlers/          # Request handlers
├── providers/         # Isolated provider implementations
├── router/            # Provider selection + strategy
├── middleware/         # Auth, logging
├── types/             # Shared types
└── utils/             # Utility functions
```
