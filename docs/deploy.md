# Deploying Let-timer Server & Web App

This guide walks you through deploying Let-timer on a VPS or cloud server with Docker Compose and Caddy (auto-HTTPS), or via systemd on bare-metal.

---

## 1. Quick Start with Docker Compose (Recommended)

### Prerequisites

- A Linux server (Debian/Ubuntu, Rocky, Arch, etc.) with Docker and Docker Compose installed.
- A domain or subdomain pointed to your server's public IP (e.g. `timer.example.com`).
- Ports `80` and `443` open in your firewall.

### Step 1: Clone Repository and Setup Environment

```bash
git clone https://github.com/mikejohnpp/Let-timer.git
cd Let-timer/deploy

cp .env.example .env
nano .env
```

Set your configuration in `.env`:

```env
DOMAIN=timer.example.com
LET_TIMER_BOOTSTRAP_TOKEN=your-random-secret-token-here
LET_TIMER_TIMEZONE=Asia/Ho_Chi_Minh
LET_TIMER_NOTIFY=ntfy
LET_TIMER_NTFY_URL=https://ntfy.sh/my-private-topic
```

### Step 2: Start the Containers

```bash
docker compose up -d
```

Caddy automatically provisions a free Let's Encrypt TLS certificate for your domain and proxies HTTP/WebSocket/SSE traffic to Let-timer with zero manual configuration.

### Step 3: Create an API Token for Your Devices

If you didn't set a bootstrap token or want an additional named token:

```bash
docker compose exec let-timer let-timer-daemon token create laptop
```

This will print a secure token once:

```text
Created token 'laptop':
lt_tok_9a8b7c6d5e4f3a2b1c...
```

### Step 4: Connect from the Web App and Remote CLI

1. **Web App**: Visit `https://timer.example.com` in your browser. Paste your token to log in. You can also install it as a PWA on iOS/Android or desktop.
2. **CLI**: On your laptop, configure the remote server profile:

```bash
let-timer server add vps https://timer.example.com --token-stdin
# Paste the token when prompted

# Switch your CLI to talk to the VPS
let-timer server use vps

# All daily commands now interact seamlessly with your VPS server!
let-timer tasks list
let-timer tasks start
let-timer tasks current
```

---

## 2. Upgrades and Maintenance

### Upgrading to New Releases

```bash
cd Let-timer/deploy

# 1. Automatic pre-upgrade DB backup
docker compose exec let-timer cp /data/let-timer.db "/data/let-timer.db.bak.$(date +%Y%m%d%H%M%S)"

# 2. Pull new images and restart
docker compose pull
docker compose up -d
```

Database migrations run automatically on startup via `PRAGMA user_version` and are transactional.

---

## 3. Backups and Disaster Recovery

### Local Backup

To create an ad-hoc snapshot:

```bash
docker compose exec let-timer sqlite3 /data/let-timer.db ".backup '/data/backup-$(date +%Y%m%d).db'"
```

### Continuous S3 / R2 Replication (Litestream)

To enable automatic continuous replication to Cloudflare R2 or AWS S3:

1. Configure `LITESTREAM_*` in `deploy/.env`.
2. Start the backup profile:

```bash
docker compose --profile backup up -d
```

### Restore Drill

To restore from a Litestream replica:

```bash
docker compose run --rm litestream restore -o /data/let-timer.db /data/let-timer.db
docker compose up -d let-timer
```

---

## 4. Bare-Metal Deployment with Systemd

If you prefer running without Docker:

1. Build the release binaries:

```bash
cargo build --release --bin let-timer-daemon --bin let-timer --features let-timer-daemon/embed-web
sudo cp target/release/let-timer-daemon /usr/local/bin/
sudo cp target/release/let-timer /usr/local/bin/
```

2. Copy the systemd service file:

```bash
cp systemd/let-timer-daemon.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now let-timer-daemon
```

3. Configure reverse proxy (Caddy / Nginx) to forward to `http://127.0.0.1:8765`. For Caddy, ensure `flush_interval -1` is configured for SSE streams.

---

## 5. Environment Variables Reference

| Variable | Default | Purpose |
|---|---|---|
| `DOMAIN` | `localhost` | Server public domain for automatic HTTPS with Caddy (e.g. `timer.example.com`). |
| `LET_TIMER_LISTEN` / `LET_TIMER_HTTP` | `0.0.0.0:8765` | Address and port for the HTTP/REST/SSE server. |
| `LET_TIMER_BOOTSTRAP_TOKEN` | *(none)* | Optional initial token generated at first daemon startup. |
| `LET_TIMER_ALLOWED_HOSTS` | *(none)* | Comma-separated allow-list of `Host` header values (e.g. `timer.example.com,localhost`). Requests with mismatched hosts receive `421 Misdirected Request` to block DNS rebinding. |
| `LET_TIMER_CORS_ORIGINS` | *(same-origin)* | Comma-separated list of allowed origins (e.g. `https://timer.example.com`). If unset, only same-origin requests are permitted. |
| `LET_TIMER_DEV` | `0` | If `1` or `true`, automatically allows Vite dev origins (`http://localhost:5173`, `http://127.0.0.1:5173`). |
| `LET_TIMER_ENABLE_DOCS` | `0` (remote) / `1` (local) | Set to `1` to expose `/swagger-ui` and `/api/v1/openapi.json` on non-loopback interfaces. Disabled by default on remote networks (returns 404). |
| `LET_TIMER_TRUSTED_PROXY` | `1` (Docker) | Set to `1` when running behind a trusted reverse proxy (Caddy, Nginx). Enables client IP extraction from `X-Forwarded-For` for audit logs and the failed-auth rate limiter. |
| `LET_TIMER_MAX_UPLOAD_MB` | `200` | Maximum allowable media upload size in megabytes. |
| `LET_TIMER_MEDIA_DIR` | `/data/medias` | Filesystem storage path for uploaded images, audio, and poem files. |
| `LET_TIMER_NOTIFY` | `ntfy` | Notification provider (`desktop`, `log`, `ntfy`, `webhook`). |
| `LET_TIMER_NTFY_URL` | *(none)* | Topic endpoint for ntfy push notifications. |
| `LET_TIMER_TIMEZONE` | `UTC` | Server fallback timezone (overridden per-client via header or settings). |

---

## 6. Security Checklist

- [x] **No root execution**: Containers run as non-root user `let-timer (uid 1000)`.
- [x] **Read-only rootfs**: All state is confined to `/data`.
- [x] **Tokens hashed at rest**: Tokens are SHA-256 hashed in the SQLite database and shown only once.
- [x] **Headers-only token transport**: No tokens accepted in query params or cookies.
- [x] **Failed-authentication rate limiter**: 10 failed authentications per minute per IP returns `429 Too Many Requests`.
- [x] **DNS rebinding & CSRF protection**: `Host` header validation and Origin check for mutating requests.
- [x] **Per-route body limits**: 1 MiB for JSON / RPC; 200 MiB for uploads with streaming directly to disk.
- [x] **Strict media validation**: Whitelist of extensions and binary magic bytes; active web content (`html`, `svg`, `js`) rejected.
- [x] **Sanitized errors**: Internal 500 errors stripped of database syntax, table names, and server paths; mapped to `request_id`.
- [x] **Minimal firewall surface**: Only ports 80/443 exposed publicly; daemon port 8765 is internal to the Docker network.
- [x] **CORS restricted**: Same-origin only by default. Override with `LET_TIMER_CORS_ORIGINS` when hosting frontend on a separate domain.
- [x] **SSE connection flush**: Caddy proxy config flushes chunks immediately (`flush_interval -1`).
