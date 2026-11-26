# Security Guide — Let-timer

## 1. Threat Model & Operational Modes

Let-timer operates in three distinct security modes:

| Mode | Description | Exposure & Controls |
|---|---|---|
| **A. Local, no token** | Daemon bound to loopback (`127.0.0.1`), no API tokens created (local developer/desktop convenience mode) | Guarded against browser drive-by attacks via Host/Origin checks and client headers. |
| **B. Local, with token** | Loopback binding with at least one active API token | Authentication required for non-public endpoints. |
| **C. Server** | Remote / reverse-proxy deployment with public network exposure | Mandatory authentication via API tokens (`Authorization: Bearer`), CORS allow-listing, rate limiting. |

### Trust Boundary
- **Unix Domain Socket** (`$XDG_RUNTIME_DIR/let-timer.sock`): Trusted local channel (file permissions `0600`, same local UID). Local CLI commands with filesystem path operations (`import_file`) are permitted here.
- **HTTP Transport** (`/rpc`, REST, SSE): Untrusted by default in all modes. No command accepting a server-side filesystem path is accepted over HTTP.

---

## 2. Remote Command Policy (`policy::validate_remote`)

All incoming HTTP commands (`/api/v1/rpc` and REST handlers executing commands) MUST pass `policy::validate_remote` prior to dispatch.
- **Exhaustive matching**: The compiler enforces that every variant in `Command` is explicitly classified as remote-safe or local-only.
- **Forbidden over HTTP**: `CreateMedia { import_file: Some(_) }` is rejected with `403 forbidden_remote_command`. Remote clients upload files via `POST /api/v1/media/upload`.

---

## 3. Storage Hardening (`copy_local`)

Even on the local Unix socket, `copy_local` enforces defense-in-depth:
- Rejects non-regular files (symlinks, devices, FIFOs, and directories).
- Canonicalizes the source path.
- Refuses paths inside the daemon's own data directory (e.g., SQLite DB and WAL files), except from within the media directory.
- Enforces an upload/import size cap (default 200 MB, configurable via `LET_TIMER_MAX_UPLOAD_MB`).

---

## 4. Browser-Origin Defenses (CORS)

- **Default policy**: Same-origin only. No cross-origin `Access-Control-Allow-Origin` headers are emitted by default, shielding the daemon against drive-by web requests.
- **Configurable allow-list**: Setting `LET_TIMER_CORS_ORIGINS=https://app1.com,https://app2.com` permits specific trusted domains.
- **Development mode**: `LET_TIMER_DEV=1` automatically allows Vite dev server origins (`http://localhost:5173`, `http://localhost:5174`, `http://127.0.0.1:5173`, `http://127.0.0.1:5174`).
- **Restricted headers**: Allowed request headers are restricted to `Authorization`, `Content-Type`, `Range`, and `X-Let-Timer-*` variants.

---

## 5. Safe Path Resolution (`fsutil::safe_join`)

Any filesystem path serving or storage management code (serving static files via `LET_TIMER_WEB_DIR`, serving managed media assets, or deleting files) MUST use `let_timer_core::fsutil::safe_join`.
- Rejects directory traversal tokens (`..`), NUL characters (`\0`), and Windows path separators (`\`).
- Strictly enforces that only `Component::Normal` path segments are permitted.
- Resolves all symlinks via `canonicalize()` and verifies that the canonical path resides strictly within the configured root directory.

---

## 6. Authentication by Construction & Signed Media URLs

### 6.1 Route Protection by Default
All routes registered in the HTTP server are protected by default:
- **Public Allow-list**: Controlled strictly by a single constant `PUBLIC_ROUTES = &["/api/v1/health"]`.
- **Protected Endpoints**: All API endpoints (`/api/v1/...`), `/metrics`, `/swagger-ui/`, and `/api/v1/openapi.json` are wrapped by `AuthLayer`. Requests without a valid API token in token-enabled mode are rejected with `401 Unauthorized`.
- **SPA Fallback**: Fallback routes serve only static assets / `index.html`. Any unknown path starting with `api/` returns `404 Not Found` and will never serve the SPA HTML shell.

### 6.2 Signed Media URLs (HMAC-SHA256)
Because browser media elements (`<img>`, `<audio>`, and CSS `background-image`) cannot attach custom authorization headers, media access is protected using short-lived signed URLs:
- **Signing Scheme**: `sig = HMAC-SHA256(media_signing_secret, "<path>|<exp>")`, where `exp` is a Unix epoch timestamp.
- **Secret Management**: A cryptographically random 32-byte secret is generated on first startup and persisted in the `settings` database (`media_signing_secret`).
- **Constant-Time Verification**: Signatures are compared using constant-time byte comparisons to prevent timing side-channels.
- **Dedicated Endpoint & DTOs**: `POST /api/v1/media/{id}/url` returns a signed URL. Media objects returned by `list_medias`, `create_media`, `upload_media`, and `get_media_list` are pre-signed with a 1-hour TTL.
- **Range & Audio Playback**: `serve_media` supports HTTP `Range` requests (returning `206 Partial Content`) for smooth audio scrubbing.
- **Response Headers**: Media responses omit wildcard `Access-Control-Allow-Origin: *` and enforce `Cache-Control: private, max-age=<ttl>`, `X-Content-Type-Options: nosniff`, `Content-Security-Policy: sandbox; default-src 'none'`, and `Cross-Origin-Resource-Policy: same-origin`.

---

## 7. Host and Origin Validation (DNS Rebinding & CSRF Guard)

To safeguard local instances (especially in Mode A where no tokens are configured), incoming HTTP requests are validated against DNS rebinding and cross-site request forgery attacks via `guard::host_origin_guard`:

- **Host Header Validation**: Every incoming HTTP request must specify a `Host` header belonging to either loopback addresses (`localhost`, `127.0.0.1`, `[::1]`) or hostnames explicitly listed in `LET_TIMER_ALLOWED_HOSTS`. Any foreign or spoofed host (such as `attacker.example:8080`) is rejected with `421 Misdirected Request`, neutralizing DNS rebinding attacks.
- **Origin Validation**: On mutating requests (`POST`, `PUT`, `PATCH`, `DELETE`) carrying an `Origin` header, the origin must either match the server's own `Host` header (same-origin) or be present in `LET_TIMER_CORS_ORIGINS` (or Vite dev origins when `LET_TIMER_DEV=1`). Foreign origins are rejected with `403 Forbidden`.
- **Simple-Request CSRF Protection**: Mutating requests (`POST`, `PUT`, `PATCH`) must specify `Content-Type: application/json` or `multipart/form-data`. Simple form submissions (`application/x-www-form-urlencoded`, `text/plain`) that browsers could issue cross-site without preflights are rejected with `415 Unsupported Media Type`.
- **Mode A Client Header**: In Mode A (`is_loopback` and no active API tokens), mutating requests must include the custom header `X-Let-Timer-Client: web|cli`. Browsers cannot attach custom headers cross-origin without triggering a CORS preflight, which untrusted origins will fail. CLI and web client bindings automatically attach this header on all requests.

---

## 8. Upload and Media Serving Safety

Media upload and serving are hardened to prevent denial of service (RAM exhaustion), arbitrary file upload, and cross-site scripting:

- **Per-Route Body Limits**: All standard JSON API endpoints and `/rpc` are strictly limited to 1 MiB (`DefaultBodyLimit::max(1024 * 1024)`). Oversized payloads immediately return `413 Payload Too Large`. The media upload route alone allows up to `LET_TIMER_MAX_UPLOAD_MB` (default 200 MB, with 25 MB for images and 1 MB for poems).
- **Streaming Directly to Disk**: Upload chunks are streamed directly into an atomic temporary file in the media directory without buffering into memory. If the upload is aborted or exceeds size limits, the temporary file is deleted immediately by a RAII guard.
- **Strict Extension & Magic Bytes Validation**: Uploaded files and local imports are validated against strict allow-lists for both extension and binary magic bytes:
  - **Images**: `png`, `jpg`/`jpeg`, `gif`, `webp`, `avif` with matching magic bytes.
  - **Audio**: `mp3`, `m4a`, `aac`, `ogg`/`opus`, `wav`, `flac` with container/header inspection.
  - **Poems**: `txt`, `md` (must be valid UTF-8 and must not contain HTML/XML/SVG/script tags).
  - Web executables and active content (`html`, `htm`, `svg`, `js`, `xml`, `xhtml`) are strictly rejected (`415 Unsupported Media Type`). Multipart `Content-Type` is never trusted.
- **Serving Security Headers**: Media endpoints serve verified `Content-Type` headers from the allow-list table (never guessed from untrusted input), `X-Content-Type-Options: nosniff`, `Content-Security-Policy: sandbox; default-src 'none'`, and `Cross-Origin-Resource-Policy: same-origin`.
- **Startup Audit**: The daemon scans the media directory at startup and logs warnings for any unrecognized or disallowed files without modifying or deleting them.

---

## 9. Secret Hygiene & Token Transport

- **Headers-Only Transport**: API tokens are accepted strictly through HTTP headers (`Authorization: Bearer <token>` or `X-Let-Timer-Token`). Query parameter (`?token=...`) and cookie (`let_timer_token=...`) extraction is permanently removed, preventing credential leakage in access logs, browser history, and `Referer` headers.
- **Fetch-Based Server-Sent Events**: The web client uses a custom `fetch`-based SSE client with a streaming line parser (`SseParser`) instead of native `EventSource`, allowing standard `Authorization: Bearer <token>` headers without URL token exposure.

---

## 10. Headers, Errors, Observability & Posture Visibility

- **Standard Security Headers**:
  - `X-Content-Type-Options: nosniff` on all responses.
  - `Referrer-Policy: no-referrer`.
  - `X-Frame-Options: DENY`.
  - `Permissions-Policy: camera=(), microphone=(), geolocation=()`.
  - SPA `Content-Security-Policy`:
    `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data: blob:; media-src 'self' https: blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'`.
    (Media routes serve an isolated `Content-Security-Policy: sandbox; default-src 'none'`).
- **Request Tracing & Error Sanitization**:
  - Every request is tagged with a unique `X-Request-Id` (preserved from client header if supplied, otherwise generated).
  - All 500 internal server error responses are intercepted and sanitized into a structured JSON envelope:
    `{ "error": { "code": "internal_error", "message": "An internal error occurred", "request_id": "<id>" } }`.
  - Database error details (`rusqlite`, `SQLITE_`), table names, and server filesystem paths are never leaked over the wire. The full error details are logged to server stderr with the associated request ID.
- **Conditional API Documentation**:
  - Swagger UI (`/swagger-ui`) and the raw OpenAPI definition (`/api/v1/openapi.json`) are mounted by default only on loopback connections.
  - On non-loopback interfaces, API documentation is disabled by default and returns 404 unless explicitly enabled via `LET_TIMER_ENABLE_DOCS=1`.
- **Structured Audit Logging**:
  - Security and state-altering events are recorded with `tracing::info!(target: "audit", ...)`:
    - API token created (token name, prefix, request IP).
    - API token rotated (token name, prefix, request IP).
    - API token revoked (token name, request IP).
    - RPC command executed (command name only without argument payload, caller IP).
    - Media upload succeeded (file ID, size, mime, IP).
    - Authentication failed (`tracing::warn!(target: "audit", ...)`, path, IP, failure reason).
  - Raw token secrets are never logged (invariant 11).
- **Daemon Posture Visibility**:
  - `let-timer daemon status` inspects and reports current daemon security configuration:
    - Authentication mode (`token (protected)` vs `no-token (Mode A / local convenience)`).
    - HTTP bind address.
    - Configured CORS origins.
    - API documentation state (enabled / disabled).

---

## 11. Failed-Authentication Rate Limiting

- **In-Memory Rate Limiter**:
  - `AuthRateLimiter` tracks failed authentication attempts in memory keyed by client IP.
  - Client IP is resolved from `ConnectInfo<SocketAddr>`, or from reverse-proxy headers (`X-Forwarded-For` / `X-Real-IP`) when explicitly enabled via `LET_TIMER_TRUSTED_PROXY=1`.
- **Sliding Window Policy**:
  - Max failed attempts: 10 failed authentications within a 60-second window.
  - When the limit is exceeded, subsequent *failed* attempts from that IP are immediately rejected with `429 Too Many Requests`.
  - Response headers include `Retry-After: <seconds>` indicating remaining cooldown.
  - Response body returns a standardized error envelope:
    `{ "error": { "code": "rate_limited", "message": "Too many failed authentication attempts. Please retry after 60 seconds." } }`.
  - **Validate-first order (login-lockout fix)**: the token is validated *before* the limiter is consulted. A request with a valid token always passes and clears that IP's failure counter, so a legitimate login can never be locked out by background failures (e.g. an unauthenticated tab polling `/events`). Only failed attempts consult and feed the limiter, which keeps brute-force protection intact.
  - Known trade-off: every request performs one token validation (SHA-256 + indexed lookup) before the limiter decision, so a locked-out IP still costs one cheap DB read per request instead of being rejected upfront. Acceptable for a single-user server; see `docs/security_hardening_plan.md` (open refinement: check-then-validate).
- **Resource Protection & Bounded Memory**:
  - Up to 10,000 distinct IPs are tracked concurrently.
  - When capacity is reached, expired failure records are swept and evicted before inserting new tracking entries.






