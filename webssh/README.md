WebSSH (monolithic, no Docker)

A single Rust binary that serves a static Web UI and simple API/WebSocket stubs. Built with axum + tokio. No Node/NPM and no Docker required.

Quick start

- Requirements: Rust toolchain (rustup), cargo
- Run: cd webssh && cargo run
- Open: http://localhost:8080

Environment

- PORT: port to bind (default 8080)
- ORIGIN_ALLOWLIST: optional comma-separated list of allowed origins for CORS when serving the API, e.g. "https://example.com,https://admin.example.com". If unset, only same-origin requests are allowed.

Routes

- GET / -> serves static/index.html
- GET /static/* -> serves static assets
- POST /api/session -> stub that returns a new session id
- POST /api/session/:id/accept-hostkey -> stub that accepts host key
- GET /api/encodings -> returns a list of character encodings
- WS /api/ws -> WebSocket echo server (stub)

Notes

- This is a scaffold: SSH functionality is stubbed in src/ssh/.
- The static UI uses xterm.js via CDN and includes a minimal connection form.
