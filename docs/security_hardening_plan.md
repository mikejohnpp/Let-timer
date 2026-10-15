# Let-timer — Security Hardening Plan (HTTP server) and Quality Follow-ups

> Audience: implementing agents. Baseline: branch `dev` @ `83afc59`.
> Read first: `AGENTS.md`, `docs/server_web_spec.md` §5.5 (security baseline), `docs/deploy.md`.
> Status of the findings: they come from **static reading** of the Rust code (the reviewer could not build or run the server). **Every task starts by reproducing the finding with a failing test.** If a finding does not reproduce, record the evidence in the PR and close that item; do not "fix" what is not broken.
> Revision 2: folds in the agent review (see section 3a): V2 mechanism reworded, SEC-1.2 struck (already implemented), V4 narrowed to the vectors that can actually work.
> Out of scope: TLS termination (Caddy), multi-user accounts, OAuth/OIDC, bundle size.

---

## 1. Threat model

| Mode | Description | Exposure |
|---|---|---|
| **A. Local, no token** | Daemon on `127.0.0.1`, no API tokens exist (today's convenience mode) | Any web page the user opens can send requests to `127.0.0.1:PORT` |
| **B. Local, with token** | Same, with at least one token | Same network exposure, auth required |
| **C. Server** | Behind a reverse proxy, tokens required, public Internet | Unauthenticated Internet clients; stolen or leaked tokens |

Attackers considered: (1) a malicious website in the user's browser (drive-by requests, CSRF, DNS rebinding); (2) a host on the same LAN reaching an exposed port; (3) an unauthenticated Internet client; (4) an authenticated client that must still not be able to read arbitrary server files.

Trust rules: the **Unix socket** is trusted (mode `0600`, same local user); **HTTP is untrusted by default**, in every mode. A command that is acceptable on the socket is not automatically acceptable over HTTP.

## 2. Findings

| ID | Severity | Finding | Evidence (verify first) |
|---|---|---|---|
| **V1** | **Critical** | Any HTTP caller can make the server **read an arbitrary local file** and publish it: `POST /api/v1/rpc` accepts the full `Command` enum, including `CreateMedia { import_file: Some(path) }`; the daemon then calls `copy_local(path)` with no restriction and stores the bytes in the media directory | `core/src/protocol/mod.rs` (`import_file`), `daemon/src/lib.rs` `create_media`, `core/src/storage.rs` `copy_local`; REST handlers pass `import_file: None`, `/rpc` does not filter |
| **V2** | **High** | Media files are served **without authentication**, and with `Access-Control-Allow-Origin: *` | `http/src/router.rs`: `.fallback(static_or_spa_handler)` is called **after** `.layer(DefaultBodyLimit/AuthLayer/cors)`. In axum 0.8.9 `Router::layer` wraps the routes **and the fallbacks that exist at that moment**; a later `.fallback(...)` replaces the endpoint with a fresh, unwrapped one (`fallback_endpoint` → `set_fallback`), and the axum docs say to add routes and fallback *before* calling `layer`. Supporting hint: the media branch inserts `ACAO: *` by hand, which would be unnecessary if `CorsLayer` wrapped it. **Differential test decides** (see SEC-2.0) |
| **V3** | **High** | `CorsLayer::allow_origin(Any)` plus mode A (no token ⇒ authorized): any website can call the API, read the answers, upload files, and chain V1+V2 to exfiltrate local files. No `Host` check ⇒ DNS rebinding also works | `router.rs` `CorsLayer`, `auth.rs` (`is_loopback && !has_tokens ⇒ Authorized`) |
| **V4** | **High** (when `LET_TIMER_WEB_DIR` is set) | Path traversal: the web-dir branch joins the raw URI path with no `..` check (literal `..` segments); the media branch only does a substring check, so the realistic bypass there is a **symlink** inside the media dir. Percent-encoded variants (`%2e%2e`) are expected to be harmless because the path is not decoded: do not spend effort on them | `router.rs` `static_or_spa_handler` |
| **V5** | Medium | `/metrics` (task, session and token counts) and Swagger UI are reachable without a token: `AuthLayer` skips every path not starting with `/api` | `auth.rs` step 1, `router.rs` |
| **V6** | Medium | Tokens are accepted in the **query string** (`?token=`, used by the web SSE hook) and in a **cookie** with no CSRF protection; URLs leak through logs, history and `Referer` | `auth.rs` `extract_token`, `web/src/hooks/useEventStream.ts` |
| **V7** | Medium | Upload accepts any file name/type, buffers up to 100 MB in memory, and files are served same-origin: an uploaded `.html`/`.svg` is **stored XSS in the app origin** (token lives in `localStorage`) | `rest.rs` `upload_media`, `router.rs` body limit, `serve_file_from_disk` (MIME by extension) |
| **V8** | Low | Global 100 MB body limit on every route; no security headers or CSP; raw DB error text returned to clients; dead exemption for `/api/v1/auth/login` (no such route); `is_loopback` is `bind_addr.starts_with("127.")`; no limiter on failed auth | `router.rs`, `auth.rs`, `daemon/src/main.rs` |
| **V9** | Info | Token stored in `localStorage` (XSS ⇒ token theft) | documented trade-off for single-user; keep V7/CSP tight |

Immediate owner mitigations (before any code lands): create an API token even for local use (`let-timer-daemon token create <name>`) so mode A is not active; do not expose the HTTP port beyond loopback; do not set `LET_TIMER_WEB_DIR` on a server; review `~/.local/share/let-timer/media` for unexpected files.

## 3. Rules for everyone

1. Test first: add a regression test that fails on the current code, then fix it. Put HTTP tests in `crates/let-timer-http/tests/security.rs` (use `tower::ServiceExt::oneshot` against `build_router`), core tests next to the code.
2. One task = one commit: `fix(security): SEC-n …`, listing the finding ids closed.
3. Definition of done: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the OpenAPI drift check (`gen-openapi`), and for web changes `npm run lint && npm test && npm run build` (+ `npm run e2e` locally).
4. No `unwrap()`/`expect()` in non-test code. No new secrets in logs, URLs or error messages.
5. Do not change the Unix-socket behaviour used by the CLI (local `import_file`, directory imports) unless a task says so.
6. Update `docs/security.md` (SEC-10) as part of each task.

## 3a. Review log (agent review of revision 1)

| # | Agent comment | Resolution |
|---|---|---|
| 1 | "`Router::layer` wraps the fallback too; the fallback then hits the `!starts_with(\"/api\")` branch, so it stays unauthenticated" | **Partly accepted.** True that `layer` wraps fallbacks, but only those that exist when it is called (axum 0.8.9 source and docs); here the fallback is set afterwards, so it is **not** wrapped and never reaches `AuthService`. The proposed path also does not fit media: `/api/v1/media/files/*` starts with `/api`, so if `AuthLayer` did run it would return 401, not pass. The conclusion (V2) is kept but flagged as *unconfirmed until the differential test in SEC-2.0 runs*; wording of the V2 row fixed |
| 2 | SEC-1.2 is outdated: `client.rs` already uploads over HTTP | **Accepted.** Verified in `core/src/client.rs`; sub-item struck, lock-in test kept |
| 3 | Encoded traversal (`%2e%2e`) is probably already 404; symlink is the real bypass | **Accepted.** V4 narrowed; no percent-decoding; test priorities reordered |
| – | Part 2 numbers match the repository | No change |
| – | Owner mitigations (token even locally, no exposed port, no `LET_TIMER_WEB_DIR` on servers) should be done today | Kept in section 2; do not wait for code |

## 4. Execution order

```
Day-0 hotfixes:  SEC-1  ->  SEC-3a (CORS allow-list)  ->  SEC-4
Then:            SEC-2  ->  SEC-3b (Host/Origin guard)  ->  SEC-5  ->  SEC-6  ->  SEC-7
Then:            SEC-8, SEC-9, SEC-10   (SEC-9/10 can start any time)
```

---

## SEC-1 — Local-only commands must not be accepted over HTTP (V1)

**Files**: `crates/let-timer-http/src/handlers.rs` (`rpc_handler`), `rest.rs` (`execute_command`), new `crates/let-timer-http/src/policy.rs`, `core/src/protocol/mod.rs` (docs), `daemon/src/lib.rs`.

**Do**

1. Create `policy::validate_remote(cmd: &Command) -> Result<(), PolicyError>` called by **every** HTTP entry point (RPC and REST) before dispatch. It rejects, with `403 forbidden_remote_command`, any command carrying a server-side filesystem path: today `CreateMedia { import_file: Some(_) }`. Implement it as an **exhaustive `match`** (no wildcard arm) so that adding a new `Command` variant fails to compile until someone classifies it as `Remote` or `LocalOnly`.
2. ~~Adjust the client so HTTP uses upload~~ **Already implemented** (verified in `core/src/client.rs`: `Client::Unix` sends `import_file`, `Client::Http` calls `upload_media`). Do not change it; only add a test that locks the behaviour in (see Tests).
3. Harden `copy_local` for the local path too: reject non-regular files (symlinks, devices, FIFOs), canonicalize first, cap size (same limit as upload), and refuse paths inside the daemon's own data directory except the media directory.
4. Document in `protocol/mod.rs` which variants are `LocalOnly`.

**Tests**: `/rpc` with `CreateMedia{import_file:"/etc/hostname"}` ⇒ 403 and **no file appears** in the media dir (use a temp `LET_TIMER_*` environment); REST create ignores the field; the exhaustive match compiles only with a classification for each variant (compile-time guard); lock-in test: CLI over the Unix socket still sends `import_file`, CLI over HTTP uses `upload_media` and never sends a path.
**Done when**: no HTTP request can cause the server to read a path of the client's choosing.

---

## SEC-2 — Authentication by construction: every route is protected unless allow-listed (V2, V5, V8)

**Files**: `router.rs`, `auth.rs`, `handlers.rs`, `docs/server_web_spec.md` (§4).

**Do**

0. **Differential test first** (decides V2 and V5, no assumptions): with at least one token in the DB, send requests with **no token** to (a) an existing media file `/api/v1/media/files/<name>`, (b) `/metrics`, (c) `/swagger-ui/`, (d) a SPA path `/some/route`. Record the status codes in the PR. Expected on the current code if the ordering theory holds: (a) 200, (b) 200, (c) 200, (d) SPA shell. If (a) already returns 401, then `AuthLayer` does wrap the fallback: keep the rest of SEC-2 (signed URLs are still needed because `<img>`/`<audio>` cannot send headers, and the `!path.starts_with("/api")` bypass still exposes `/metrics`), and record that V2 was not reproduced in token mode.
1. Replace the path-prefix exemption logic in `AuthService` with **route grouping**:

```rust
let public = Router::new()
    .route("/api/v1/health", get(health));                  // the entire public API surface

let protected = Router::new()
    /* all /api/v1/... routes, /metrics */
    .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

let media = Router::new()
    .route("/api/v1/media/files/{*path}", get(serve_media))  // SEC-2.3
    .route_layer(middleware::from_fn_with_state(state.clone(), require_media_access));

Router::new()
    .merge(public).merge(protected).merge(media)
    .fallback(spa_handler)                                   // static SPA only, no data
    .layer(/* security headers, CORS, body limits: see SEC-3/5/7 */)
```
   Remove the `!path.starts_with("/api")` bypass and the dead `/api/v1/auth/login` exemption. A table-driven test enumerates every registered route and asserts the expected auth requirement, so a new route cannot be added unprotected by accident (maintain one `const PUBLIC_ROUTES: &[&str]`).
2. `/metrics` and `/swagger-ui` + `/api/v1/openapi.json`: behind auth, **or** disabled unless `LET_TIMER_ENABLE_DOCS=1` / loopback (decision D4). Metrics never public in mode C.
3. **Media authorization** (decision D1): `<img>`, `<audio>` and CSS backgrounds cannot send headers, so use **signed, expiring URLs**:
   - New authenticated endpoint `POST /api/v1/media/{id}/url` (or include `url` signing in media DTOs) returns `/api/v1/media/files/<path>?exp=<unix>&sig=<hex>`.
   - `sig = HMAC-SHA256(server_secret, "<path>|<exp>")`; `server_secret` is 32 random bytes generated on first run and stored in `settings` (new key `media_signing_secret`); compare with `subtle::ConstantTimeEq`; default TTL 1 hour; reject expired or tampered requests with 401.
   - Supports `Range` (the player needs it): signature covers path and expiry only.
   - Alternative (if the owner prefers): an HttpOnly, `SameSite=Strict`, `Secure` cookie issued by a real `POST /api/v1/auth/login`. Choose one in the PR and record it in `docs/security.md`.
4. Remove `Access-Control-Allow-Origin: *` from media responses. If the wallpaper-harmonize feature needs pixel access, it must load same-origin media (it does) or use `crossorigin` against an explicit allow-list.
5. Add `Cache-Control: private, max-age=<ttl>` on signed media, not `public, immutable`.
6. The web client (`web/src`) requests a signed URL (via the generated API client) wherever it builds a media `src`. Update `AudioPlayerContext`, `MediaView`, `FocusView`, `useWallpaper`, `palette.ts` consumers; add one helper `useSignedMediaUrl(media)` with caching until near expiry.

**Tests**: no-token requests: `/api/v1/tasks`, `/metrics`, `/swagger-ui`, media files ⇒ 401 whenever any token exists; `/api/v1/health` ⇒ 200; valid signature ⇒ 200 including a `Range: bytes=0-99` ⇒ 206; expired ⇒ 401; changed path or `exp` ⇒ 401; route-enumeration test (above); web unit tests for the URL helper; e2e: the player plays an uploaded track under a token.
**Done when**: the only unauthenticated server responses are `/api/v1/health`, the SPA shell/assets, and correctly signed media.

---

## SEC-3 — Browser-origin defences (V3)

### SEC-3a — CORS allow-list (hotfix)
**Files**: `router.rs`, `daemon/src/main.rs`, `docs/deploy.md`.
**Do**: replace `CorsLayer::allow_origin(Any)` with an allow-list from `LET_TIMER_CORS_ORIGINS` (comma separated). Default: **none** (same-origin only, which is what the embedded SPA needs). In dev (`LET_TIMER_DEV=1`) allow the Vite origins (`http://localhost:5173/5174`, `http://127.0.0.1:…`). Never combine a wildcard with credentials. Allowed headers: `Authorization`, `Content-Type`, `X-Let-Timer-*`, `Range`.
**Tests**: preflight from `https://evil.example` ⇒ no `Access-Control-Allow-Origin`; from an allow-listed origin ⇒ allowed; same-origin SPA still works (e2e).

### SEC-3b — Host/Origin guard (DNS rebinding and CSRF, mode A and B)
**Files**: new `crates/let-timer-http/src/guard.rs`, `router.rs`, `daemon/src/main.rs`.
**Do**

1. Middleware `host_origin_guard` applied to everything except `/api/v1/health`:
   - `Host` must be in the allowed set derived from the bind address and port (`127.0.0.1:P`, `localhost:P`, `[::1]:P`) plus `LET_TIMER_ALLOWED_HOSTS` (for the public hostname in mode C). Otherwise 421/403.
   - For non-`GET`/`HEAD` requests: if `Origin` is present it must equal the request's own origin or be in the CORS allow-list; also require `Content-Type: application/json` (or multipart for upload) so cross-site simple requests cannot be forged.
2. In mode A (no token) additionally require the header `X-Let-Timer-Client: web|cli` on mutating requests (forces a CORS preflight for any cross-origin caller). Make the web client and the HTTP CLI client send it.
3. Replace `is_loopback = bind_addr.starts_with("127.")` with `socket_addr.ip().is_loopback()` after resolving the bind address (handles `localhost`, `::1`).
4. Decision D2: keep mode A (guarded, as above) **or** refuse to start the HTTP listener without a token unless `--insecure-no-auth` is passed. Default in this plan: guarded mode A plus a startup warning in the log and in `daemon status` ("no API token: create one").

**Tests**: request with `Host: attacker.example` ⇒ rejected; `POST` with foreign `Origin` ⇒ rejected; text/plain form POST ⇒ rejected; guarded same-origin request ⇒ OK; `[::1]` recognised as loopback; e2e unaffected.
**Done when**: a page on another origin cannot read or change data through a user's local daemon (verify manually with a test page on a different port).

---

## SEC-4 — Safe path joining for static and media files (V4)

**Files**: new `crates/let-timer-core/src/fsutil.rs` (or `http/src/paths.rs`), `router.rs`, `core/src/storage.rs`.

**Do**: one function used by every file-serving branch:

```rust
pub fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() || rel.contains('\0') || rel.contains('\\') { return None; }
    let p = Path::new(rel);
    if p.components().any(|c| !matches!(c, Component::Normal(_))) { return None; } // no .., no root, no prefix
    let root = root.canonicalize().ok()?;
    let full = root.join(p).canonicalize().ok()?;                                  // resolves symlinks
    full.starts_with(&root).then_some(full)
}
```
Use it for `LET_TIMER_WEB_DIR` and media serving, and in `LocalStorageBackend::delete`/`put` (replace the `contains("..")` substring checks). Do **not** percent-decode: stored names are sanitised and content-hashed, so literal handling is sufficient and safest; a request containing `%` sequences that do not match an existing file simply 404s.

**Tests** (priority order): literal `..` in the web-dir branch (`GET /../x` and `/a/../../b` over a raw socket), **a symlink inside the root pointing outside**, absolute paths, NUL and backslash ⇒ all `None`/404; one single assertion that `/%2e%2e/%2e%2e/etc/passwd` and `/..%2fsecret` return 404 (expected already dead, no further work); normal nested files OK; SPA fallback still serves `index.html` for unknown routes.
**Done when**: no request can read outside the configured roots (confirm with a raw-socket test sending `GET /../x HTTP/1.1`).

---

## SEC-5 — Upload and media serving safety (V7, V8)

**Files**: `rest.rs` (`upload_media`), `router.rs`, `core/src/storage.rs`, `deploy/Caddyfile`.

**Do**

1. **Body limits per route**: default 1 MiB (`DefaultBodyLimit::max`) for all JSON routes and `/rpc`; the upload route alone gets `LET_TIMER_MAX_UPLOAD_MB` (default 200 for audio, 25 for images, 1 for poems; enforce by `media_type`).
2. **Stream to disk** instead of `field.bytes().await` into a `Vec` (write chunks to a temp file in the media dir, count bytes, abort at the limit, then rename to the content-hashed name). Memory stays flat.
3. **Allow-lists** by media type for extension *and* magic bytes (no new dependency needed for a small table): images `png jpg jpeg gif webp avif`; audio `mp3 m4a aac ogg opus wav flac`; poems `txt md`. Reject everything else (415). Never accept `html`, `htm`, `svg`, `js`, `xml`, `xhtml`. Do not trust the multipart `Content-Type`.
4. **Serving headers** on all media responses: `X-Content-Type-Options: nosniff`, `Content-Security-Policy: sandbox; default-src 'none'`, `Cross-Origin-Resource-Policy: same-origin`, and a `Content-Type` taken from our allow-list table (not from guessing by extension).
5. Existing files: add a startup report (log only) of files in the media dir whose type is not allowed. Do not delete automatically.

**Tests**: uploading `evil.html`, `evil.svg`, a `.png` containing HTML, and an oversized file ⇒ rejected, nothing stored; oversized JSON body ⇒ 413; a valid upload of each type ⇒ 201; served headers present; memory use during a 150 MB upload stays bounded (assert via a counting writer, not RSS).
**Done when**: an uploaded file cannot execute script in the app origin.

---

## SEC-6 — No secrets in URLs, one token transport (V6)

**Files**: `auth.rs`, `sse.rs`, `web/src/hooks/useEventStream.ts`, `web/src/api/client.ts`, `core/src/client.rs`.

**Do**

1. `extract_token` accepts only `Authorization: Bearer <token>` and `X-Let-Timer-Token`. **Remove** `?token=` and the `let_timer_token` cookie (nothing sets it; it would need CSRF protection).
2. Web SSE: replace `EventSource(...?token=...)` with a `fetch`-based stream (`fetch('/api/v1/events', { headers: { Authorization } })` + `ReadableStream` line parser, with reconnect and the existing resync logic). Keep the hook's public API unchanged.
3. Make sure no logging layer prints request URIs with sensitive parameters, or `Authorization`/`X-Let-Timer-Token` headers. Add `Referrer-Policy: no-referrer` (SEC-7).
4. Document that tokens are sent only in headers.

**Tests**: `?token=` ⇒ 401 even if the token is valid; cookie ⇒ 401; header variants ⇒ OK; web unit test for the stream parser (chunk boundaries, `\r\n`, comments/keep-alives, reconnect); e2e: live update from a second client arrives.
**Done when**: `grep -rn "token=" crates web/src` finds no transport usage.

---

## SEC-7 — Headers, errors, and observability (V5, V8)

**Files**: `router.rs`, `handlers.rs`, `rest.rs`, `auth.rs`, `daemon/src/main.rs`.

**Do**

1. Security headers middleware for all responses: `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`, `Permissions-Policy: camera=(), microphone=(), geolocation=()`, and a **CSP** for the SPA:
   `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data: blob:; media-src 'self' https: blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'`.
   Tune `img-src`/`media-src` for external wallpaper and media URLs (decision D5) and verify the app (Focus wallpaper, palette harmonize, PWA service worker) with the CSP on. Start in `Content-Security-Policy-Report-Only` for one iteration if needed.
2. Error handling: stop returning raw DB/IO errors to clients (`AuthCheckResult::DatabaseError(msg)` and others). Return `{error:{code:"internal_error", message:"…", request_id}}`; log the detail with the same `request_id` (add a request-id layer).
3. Structured audit log lines for: token created/revoked/rotated, failed auth (no token value), `/rpc` command names (no payloads), upload accepted/rejected.
4. `daemon status` shows security posture: auth mode (tokens present or not), bind address, CORS allow-list, docs enabled.

**Tests**: headers present on SPA, API, 404, and error responses; no response body contains `rusqlite`, `SQLITE_` or a file path; request-id appears in logs; CSP e2e (Focus wallpaper renders, no console CSP violations).
**Done when**: the e2e suite passes with the CSP enforced.

---

## SEC-8 — Failed-authentication limiter (V8)

**Files**: `auth.rs`, `router.rs`.
**Do**: in-memory limiter keyed by peer IP (use `ConnectInfo`; behind Caddy use the proxy-provided address only when `LET_TIMER_TRUSTED_PROXY` is set): 10 failed authentications per minute ⇒ `429` with `Retry-After`; successful requests are not limited; periodic cleanup of old entries. Use `subtle::ConstantTimeEq` wherever a secret is compared in Rust (signatures; token hashes are looked up in SQL).
**Tests**: 11th failure in a minute ⇒ 429; a good token is not blocked by another client's failures; limiter state is bounded.

---

## SEC-9 — Supply chain and CI

**Files**: `.github/workflows/ci.yml`, new `.github/dependabot.yml`, `deny.toml`.
**Do**: add `cargo audit` (RustSec) and `cargo deny check` (licenses: the project is GPL-3.0-or-later; bans; sources) to the Rust job; `npm audit --omit=dev --audit-level=high` to the web job; Dependabot for `cargo`, `npm` (web), `github-actions`, `docker`; pin GitHub Actions by version tag at least. Add a PR template with the security checklist from §6.
**Done when**: CI fails on a known-vulnerable dependency (verify with a temporary pin) and passes on the current tree.

---

## SEC-10 — Documentation and project rules

**Files**: new `docs/security.md`, `docs/deploy.md`, `AGENTS.md`, `docs/server_web_spec.md`.
**Do**: write `docs/security.md` (threat model from §1, modes A/B/C, how to run safely, token management, media authorization design chosen in SEC-2, CSP, backup/restore, what is intentionally not protected). Add to `AGENTS.md`:
- "No command that carries a filesystem path is accepted over HTTP; `policy::validate_remote` classifies every `Command` variant."
- "Every HTTP route is protected by default; the public allow-list is one constant, covered by a test."
- "Never accept credentials in URLs. Never reflect internal errors to clients."
- "Any file-serving code must use `safe_join`."
Update `docs/deploy.md` with the new environment variables: `LET_TIMER_CORS_ORIGINS`, `LET_TIMER_ALLOWED_HOSTS`, `LET_TIMER_MAX_UPLOAD_MB`, `LET_TIMER_ENABLE_DOCS`, `LET_TIMER_TRUSTED_PROXY`, `LET_TIMER_DEV`.

---

## 5. Decisions for the owner (defaults used in this plan)

| # | Decision | Default | Alternative |
|---|---|---|---|
| D1 | How media is authorised | Signed, expiring URLs (HMAC) | HttpOnly SameSite cookie via a real login endpoint |
| D2 | Local no-token mode | Keep, but guarded by Host/Origin/header checks and a warning | Require a token always (`--insecure-no-auth` opt-out) |
| D3 | Token transport | Headers only (remove `?token=` and cookie) | Keep cookie with CSRF protection |
| D4 | Swagger UI and `/metrics` | Behind auth; docs only when `LET_TIMER_ENABLE_DOCS=1` | Public on loopback |
| D5 | CSP strictness | `img-src`/`media-src` allow `https:`; no inline scripts | Per-domain allow-list |
| D6 | Upload limits | 200 MB audio, 25 MB images, 1 MB poems | Different caps |

## 6. Acceptance checklist (whole plan)

- [ ] No HTTP request can make the server read a client-chosen path (SEC-1, SEC-4); an exhaustive match forces every new `Command` to be classified.
- [ ] A route-enumeration test proves each route's auth requirement; the only public surface is `/api/v1/health`, the SPA assets, and signed media.
- [ ] Media, `/metrics` and docs are not reachable without a token (or a valid signature).
- [ ] A page on a foreign origin cannot read or mutate data through the local daemon (CORS, Host, Origin, preflight-forcing header).
- [ ] No credentials in URLs; no cookie auth; no `?token=`.
- [ ] Uploads are type-checked, size-limited, streamed, and served with `nosniff` and a sandbox CSP; no active content can run in the app origin.
- [ ] Security headers and CSP are present; the e2e suite passes with CSP enforced.
- [ ] Errors do not leak internals; request ids correlate logs.
- [ ] CI runs `cargo audit`, `cargo deny`, `npm audit`; Dependabot enabled.
- [ ] `docs/security.md` and `AGENTS.md` are updated; every finding V1–V8 has a regression test.

---

# Part 2 — Quality follow-ups (after Part 1, lower priority)

> Same rules as section 3. These tasks change structure, not behaviour: each must keep all existing tests green and add no new features.

## Q1 — ESLint for the web app

**Files**: `web/eslint.config.js`, `web/package.json`.
**Do**: flat config with `typescript-eslint` (recommended + type-aware where cheap), `eslint-plugin-react-hooks` (`rules-of-hooks`, `exhaustive-deps` as errors), `eslint-plugin-jsx-a11y` (recommended), `no-restricted-syntax` forbidding direct `localStorage`/`sessionStorage` outside `src/lib/storage.ts` (see Q2), `no-console`. Add to `npm run lint` and CI. Fix or justify (`// eslint-disable-next-line <rule> -- reason`) all findings; no blanket disables.
**Done when**: `npm run lint` includes ESLint and is green.

## Q2 — Typed storage layer

**Files**: new `web/src/lib/storage.ts`, tests, then replace the 39 direct `localStorage` uses (`FocusView`, `palette.ts`, `api/client.ts`, settings, theme, motion, wallpaper, …).
**Do**: a single registry of keys (`let_timer_token`, `let_timer_wallpaper`, `let_timer_motion`, `let_timer_palette`, …) with typed `get/set/remove`, JSON parsing with validation and defaults, `try/catch` for private mode/quota, SSR-safe guards in one place, and a cross-tab `storage` event hook (`useStoredValue(key)`).
**Tests**: corrupt JSON falls back to defaults; unavailable storage does not throw; hook updates on `storage` events.

## Q3 — Split large web files (no behaviour change)

Targets (each file below about 400 lines, one responsibility): `MediaView` (973) into `MediaLibrary`, `PlaylistPanel`, `MediaForm`, hooks; `TodayView` (863) into `TaskRow`, `TaskGroup`, `QuickAdd`, `useTodayTasks`, `useTaskActions`; `SettingsView` (687) into one component per settings section; `FloatingPlayerIsland` (657) into `IslandPill`, `IslandCard`, `IslandControls`. Keep the motion ownership rules (one engine per element) and the layout keys (`layoutId`) intact. Move in small commits; snapshot/behaviour tests must stay green; e2e `task-move` and `tabs` must pass.

## Q4 — Split large Rust files (no behaviour change)

`daemon/src/lib.rs` (1,718 lines) into `handlers/{tasks,workspaces,media,settings,tokens,timer}.rs` plus a thin dispatcher; `http/src/rest.rs` (1,345) into `rest/{tasks,timer,workspaces,media,media_lists,settings,tokens}.rs`; `cli/src/main.rs` (1,291) into command modules matching `cli.rs`. Replace the two guarded `.pop().unwrap()` in `daemon/src/lib.rs` with `into_iter().next()` patterns. Keep public signatures; tests move with the code.

## Q5 — UX leftovers from the walkthrough review

1. Task group labels move with their rows (give the headers `layout="position"` inside the same `LayoutGroup`) and the entering row waits about 60 ms, so text never overlaps during start/pause/complete/add.
2. Tab change without a blank frame: let the incoming view overlap the outgoing one briefly (`mode="popLayout"` or two stacked layers) while keeping a single fade-out/fade-in.
3. Wallpaper thumbnails in Settings: `loading="lazy"`, fixed aspect-ratio box and a skeleton/fade-in (A31) so they do not pop in late.
4. Idle clock in Focus gets a label/date ("Local time · Thu 15 Oct") so it is not mistaken for a countdown.
5. `backdrop-blur` audit (40 uses): measure frame times during tab change and wallpaper crossfade (rAF probe in `/dev/motion`), replace blur on animated layers with pre-blurred wallpaper or opaque surfaces where it costs more than 8 ms per frame.

## Q6 — E2E in CI (optional)

Add a `web-e2e` job: build the daemon with `embed-web` or start Vite, run `npx playwright install --with-deps chromium`, set the same Playwright version pinned in `package.json`, upload traces on failure. Keep the Nix-browser path for local runs.
