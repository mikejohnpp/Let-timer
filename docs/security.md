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


