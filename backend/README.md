Backend service (Axum + Tokio)

Overview
- Minimal Axum-based HTTP server with:
  - Health route: GET /health
  - WebSocket endpoint stub: GET /api/ws
  - REST stubs:
    - POST /api/session
    - POST /api/session/:id/accept-hostkey
    - GET /api/encodings
  - JSON error responses
  - CORS with origin allowlist
  - Env-based configuration
  - Structured logging via tracing

Configuration (env)
- APP_HOST: interface to bind (default: 0.0.0.0)
- APP_PORT: port to bind (default: 8080)
- APP_ALLOW_ORIGINS: comma-separated list of allowed origins for CORS. Use * for any. Example: http://localhost:3000,https://example.com
- RUST_LOG: optional tracing filter, e.g.: RUST_LOG=debug,tower_http=info

Run locally
- Prerequisites: Rust toolchain (stable), cargo
- Commands:
  - cd backend
  - cargo run
  - Or with env vars: APP_PORT=8080 APP_ALLOW_ORIGINS=http://localhost:3000 cargo run

Build
- cd backend && cargo build --release
- Binary at backend/target/release/backend

Docker
- Build: docker build -f Dockerfile.backend -t backend:latest .
- Run: docker run --rm -p 8080:8080 -e APP_PORT=8080 -e APP_ALLOW_ORIGINS=* backend:latest

HTTP API
- GET /health -> {"status":"ok"}
- GET /api/encodings -> {"encodings":["utf-8", ...]}
- POST /api/session -> 201 {"id":"sess_stub_1","message":"session created (stub)"}
- POST /api/session/:id/accept-hostkey -> 204 (no content)
- GET /api/ws -> WebSocket upgrade; server sends "welcome" then closes (stub)

Project layout
- backend/
  - Cargo.toml
  - src/
    - main.rs: bootstrap, router
    - config.rs: env-based config loader
    - errors.rs: JSON error responses
    - security.rs: CORS helpers and future security utils
    - ws.rs: WebSocket endpoint stub

Notes
- This backend is scaffolded only; business logic, persistence, and real WS handling should be implemented next.
