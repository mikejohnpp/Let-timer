# AGENTS.md — Let-timer

## Read this first

| Document | What it answers | Status |
|---|---|---|
| `docs/product_spec.md` | **What** the product does locally: requirements (`R1…`), lifecycle, timer rules, CLI surface, decisions (`D1…`). | Implemented (P0–P9) |
| `docs/implementation_plan.md` | How the local product was built (phases P0–P9). | Historical |
| `docs/server_web_spec.md` | **What** the server, web app and remote CLI must do: requirements (`S1…`), stack, API, decisions (`SD1…`). | **Current** |
| `docs/server_web_plan.md` | **In what order / how**: phases S0–S8, files, tests, "done when". | **Current** |
| `docs/web_design_plan.md` | Web **design**: flows, IA, wireframes, timer states, tokens, components, phases W0–W5, decisions (`DW1…`). | **Current** |
| `docs/motion_plan.md` | Web **motion**: tokens, moment specification (A01–A31), calm budget, reduced-motion parity. | **Current** |
| `docs/playwright_mcp_nix.md` | Safe Playwright-MCP-on-Nix setup and the rules for agents that drive a browser. | **Current** |
| This file | **Conventions**: layout, code style, invariants, workflow. | |

If these disagree: specs > plans > this file's examples. Fix the lower-priority document in the same change.
`docs/API_DESIGN.md` is **superseded** by `docs/server_web_spec.md` §4 and the generated `openapi.json`.

---

## Project Overview

**Let-timer** is a CLI **daily task manager with a background timer**. The user
schedules tasks per day (optionally with an estimated time), *starts* one task,
and a timer runs inside the daemon while the task's status follows what the user
does. When the estimate runs out the user is **notified** — the task is **never**
completed automatically; only an explicit `done` completes it. Tasks live in
workspaces; the active workspace scopes the task commands. Tasks can reference a
MediaList (image / music / poem) that a future web client (public API) will render.

It follows a **client ↔ daemon** architecture where a background daemon owns the
database and the CLI binary talks to it over a Unix domain socket. The same daemon
can also run as a **server** (HTTP + web app, see `docs/server_web_spec.md`), and
the CLI can target any configured **server profile** (`let-timer server …`).

- **License**: GPL-3.0-or-later
- **Rust edition**: 2024 (stable toolchain)
- **Dev environment**: [devenv](https://devenv.sh/) + Nix (see `devenv.nix`)
- **Working branch**: `dev`

---

## Domain Invariants (never break these)

1. **At most one running task, globally** (across workspaces). Enforced in the
   service *and* by a unique partial index on open sessions.
2. **Nothing completes a task except the explicit `done` command.** Timer expiry
   only notifies.
3. **Status changes go through `core::service` only.** Repositories do not expose
   status setters to callers; `tasks edit` must not change `status`.
4. **Status and timer sessions change in one DB transaction.**
5. **Timer values are derived, never stored**: `elapsed`/`remaining` are computed
   from `task_sessions` + `now`. Do not add a counter column.
6. **Time = UTC wall clock, RFC 3339 text.** Logic takes `now` as a parameter;
   only the daemon entry point calls `Utc::now()`.
7. **Business rules live in `core`**, not in the daemon handler or the CLI, so a
   future HTTP API reuses them.
8. **Active workspace is daemon-owned** (DB `settings`) for the local socket.
   Remote requests (HTTP) carry the workspace **explicitly** (header / CLI profile
   pin); a shared server must never let clients change each other's workspace.
9. **Every entry point (Unix socket, HTTP `/rpc`, REST) calls the same
   `handle_command` with a `RequestContext`.** Never put rules in a transport.
10. **"Today" comes from `RequestContext.today`** (user time zone), never from
    `now.date_naive()` / `Utc::now()` inside logic.
11. **Auth and secrets**: non-loopback HTTP requires an active token; tokens are
    stored hashed, shown once, never logged and never passed as CLI arguments.

---

## Workspace Layout

```
Let-timer/
├── Cargo.toml                   # Workspace root (resolver = "2")
├── crates/
│   ├── let-timer-core/          # Shared library — models, DB, services, IPC, protocol
│   │   └── src/
│   │       ├── lib.rs           # Public re-exports
│   │       ├── db/              # SQLite via rusqlite
│   │       │   ├── connection.rs    # Database open / migrate / PRAGMA
│   │       │   ├── error.rs         # DbError enum
│   │       │   ├── models.rs        # Priority, TaskStatus, Task, Workspace, Media…
│   │       │   ├── task_repository.rs
│   │       │   ├── workspace_repository.rs
│   │       │   ├── media_repository.rs
│   │       │   ├── media_list_repository.rs
│   │       │   ├── session_repository.rs   # (planned P2) task_sessions
│   │       │   └── settings_repository.rs  # (planned P5) key/value settings
│   │       ├── service/         # (planned P2) lifecycle + timer rules
│   │       │   ├── transition.rs    # pure status state machine
│   │       │   └── snapshot.rs      # pure TimerSnapshot::compute(task, sessions, now)
│   │       ├── notify.rs        # Notifier trait (+ composite, ntfy, webhook: S1/S4)
│       ├── events.rs        # (planned S1) event bus: TimerChanged / TimerExpired
│   │       ├── ipc/             # Unix socket transport
│   │       │   ├── client.rs    # IpcClient (async, tokio UnixStream)
│   │       │   └── server.rs    # IpcServer (accept loop + signal handling)
│   │       ├── protocol/        # Command / Response enums (serde JSON)
│   │       └── parse.rs         # Date/weekday parsing helpers
│   │
│   ├── let-timer-cli/           # CLI binary (`let-timer`)
│   │   └── src/
│   │       ├── main.rs          # Entry point, command dispatch
│   │       ├── cli.rs           # clap definitions (Component → Command)
│   │       ├── interactive.rs   # inquire prompts for missing fields
│   │       └── render.rs        # Human-readable, `short` status line & JSON output
│   │
│   ├── let-timer-http/          # (planned S2) axum router: /rpc, REST, SSE, auth
│   │
│   └── let-timer-daemon/        # Daemon binary (`let-timer-daemon`)
│       └── src/
│           ├── main.rs          # Entry point, IpcServer + thin command handler
│           ├── lib.rs           # Command → service adapters (no business rules)
│           └── scheduler.rs     # (planned P4) fires the "time's up" notification
│
├── web/                         # (planned S6) Vite + React + TS web app (PWA)
├── deploy/                      # (planned S7) docker-compose, Caddyfile, .env.example
├── docs/
│   ├── product_spec.md, implementation_plan.md   # local product (done)
│   ├── server_web_spec.md, server_web_plan.md    # server / web / remote CLI
│   ├── web_design_plan.md, playwright_mcp_nix.md # web design + browser tooling
│   ├── commands.md                               # command reference
│   └── API_DESIGN.md                             # superseded
├── devenv.nix                   # Nix dev environment
└── rust-toolchain.toml          # Pinned Rust channel (stable)
```

Items marked *(planned)* do not exist yet; create them in the phase named.

---

## Architecture

```
┌──────────────┐        Unix socket         ┌──────────────────┐
│  let-timer   │  ───── JSON lines ───────▶ │ let-timer-daemon │
│  (CLI)       │  ◀───── JSON lines ──────  │   (background)   │
└──────────────┘                            │  IPC handler     │
                                            │  Scheduler ──────┼──▶ Notifier
 (future) HTTP API ──────────────┐          └────────┬─────────┘     (notify-send)
                                 ▼                   │
                         core::service  ◀────────────┘
                                 │
                          ┌──────▼───────┐
                          │   SQLite DB  │
                          │  (WAL mode)  │
                          └──────────────┘
```

1. **Protocol** — `Command` and `Response` enums serialised as newline-delimited
   JSON over a Unix socket (`$XDG_RUNTIME_DIR/let-timer.sock` or
   `/tmp/let-timer.sock` fallback; override with `LET_TIMER_SOCKET`).
2. **Database** — SQLite with `rusqlite` (bundled), WAL journal, foreign keys
   enabled. Default location: `~/.local/share/let-timer/let-timer.db` (override
   with `LET_TIMER_DB`). Schema migrations run on open via `PRAGMA user_version`.
3. **Service layer** — `core::service` owns the task lifecycle and timer maths.
   The daemon handler and any future HTTP server are thin adapters over it.
4. **CLI** — `clap` derive API with a two-level subcommand hierarchy:
   `let-timer <component> <action>` (e.g. `tasks create`, `workspaces list`).
   Missing required fields are prompted interactively via `inquire`, unless
   `--no-interactive` is set.

---

## Key Conventions

### Code Style

- **Rust 2024 edition** — use `let` chains, `async fn` in traits, etc.
- Run `cargo fmt` before committing. There is no `.rustfmt.toml` override —
  the default style applies.
- Run `cargo clippy --workspace -- -D warnings` — warnings are errors.
- Section separators use the `// ─── Title ───…` banner style (see `models.rs`).

### Naming & Patterns

- **Repository pattern** — each entity (`Task`, `Workspace`, `Media`,
  `MediaList`, sessions, settings) has a repository struct that borrows a
  `&Connection` and exposes `create`, `list_all`, `update`, `delete`, etc.
  Repositories are *dumb storage*; rules belong in `service`.
- **New / Update structs** — creating an entity uses `New*` (all required
  fields); updating uses `Update*` with `Option` fields (`None` = no change,
  `Some(None)` = set NULL). Every **nullable** column must use `Option<Option<T>>`
  in `Update*` (otherwise it cannot be cleared). The `double_option` serde helper
  preserves the three-state distinction over JSON.
- **Error handling** — `DbError` for the database layer; `TransitionError` for
  illegal lifecycle moves (message must say what to do instead); the CLI defines a
  `Failure` enum (`Daemon`, `Refused`, `Usage`) to produce user-facing
  messages and exit codes.
- **No `unwrap()` / `expect()` in production code** — use `?`, `map_err`, or
  `expect` with a justification comment. Acceptable only in tests. (Mutex poison:
  `lock().unwrap_or_else(|e| e.into_inner())`.)
- **Logging** — `tracing` in daemon/server (no `println!`); never log tokens.
- **Web (`web/`)** — TypeScript strict, no `any`; API types only from the
  generated `openapi-typescript` output; UI follows `docs/web_design_plan.md`.
- **No silent success** — a command that is not implemented returns
  `Response::Error`, never `OkEmpty`.

### CLI Design Rules

- A **component** (`tasks`, `workspaces`, `media-lists`, `medias`) is always required.
- A component with no subcommand defaults to its **list** action.
- `--no-interactive` and `--json` are **global** flags (work at any position).
- Missing fields → interactive prompt (or error if `--no-interactive`).
- Output goes to **stdout**; errors go to **stderr**. Scripts can trust stdout.
- Exception: `tasks current --format short` (status-bar mode) prints nothing and
  exits 0 when the daemon is unreachable.
- Task commands are scoped to the **active workspace** unless `--workspace` /
  `--all-workspaces` is given. `done/pause/resume/stop/cancel` without an id act
  on the running task.

### Web Motion Conventions

- **Plan**: `docs/motion_plan.md` defines the moment-by-moment specification (A01–A33) and calm budget.
- **Calm, minimal, purposeful**: Motion explains state changes, never decorates.
- **One engine per element**: An element may be animated by exactly one engine (Motion props, CSS keyframe/transition class, or WAAPI `animate()` wrapper). Never combine them on the same element.
- **Ownership policy**:
  - Views, list rows, toasts, floating island, shared thumb: Motion (`AnimatePresence`, `layout`, `layoutId`).
  - Radix overlays (drawer, modal, popover, menu, tooltip): CSS keyframes on `data-state`.
  - State tints, hover, press, progress bar, running dot, ring pulses, equalizer: CSS transitions/keyframes.
  - Imperative one-offs (wallpaper crossfade): `animate()` wrapper.
- **Tokens only**: All animation durations and easings must come from `motion.css` and `tokens.ts`. No raw timings or easings in TSX (`duration: 0.x`, `ease: [...]`, `transition-duration: [0-9]`, `cubic-bezier(` outside token files; enforced by `npm run lint`).
- **Reduced-motion parity**: Both `data-motion="reduced"` and `@media (prefers-reduced-motion: reduce)` must produce identical behavior (0px travel, no loops, fades ≤ 100ms). Test mode `data-motion="off"` zeroes all durations.
- **Calm budget**: No easing with overshoot; at most 2 ring pulses per event; max 1 ambient loop on screen (running dot stops after 7 iterations ~28s); timer digits never animate.
- **Compositor only**: Animate only `transform`, `opacity`, and color properties (row collapse A21 and completed disclosure A33 are the sole height exceptions).
- **Idle fade in Focus**: Opacity-only (hidden controls remain in tab order); restores on mouse, key, touch, `:focus-within`; never hides while a menu, drawer, or sheet is open.
- **Verification**: Run `/dev/motion` and `npm run e2e` before merging motion changes.

### Testing

- Tests live in `#[cfg(test)] mod tests` at the bottom of each file.
- Use `Database::open_in_memory()` for repository/service tests — no file I/O.
  **Migration tests must use a file DB** in a temp dir (table rebuilds behave
  differently with real files and PRAGMAs).
- Time-dependent logic takes `now` as a parameter — tests pass fixed instants and
  never sleep. The scheduler is tested with `tokio::time::pause()`
  (`tokio` includes `test-util`) and a `RecordingNotifier`.
- IPC end-to-end tests spawn an in-process `IpcServer` on a temp socket using
  `LET_TIMER_SOCKET` / `LET_TIMER_DB`. Only tests needing an *external* daemon
  skip when none is running.
- A new lifecycle rule is not done until its cell in the transition table has a
  test.

### Definition of Done (every change)

1. `cargo fmt --all --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. New/changed behaviour has tests; new commands appear in
   `every_command_the_daemon_has_is_reachable`.
5. Spec / plan / the schema section below updated if behaviour, wire format or
   schema changed.
6. When `web/` is touched: `npm run lint && npm test`; when the HTTP contract
   changes: regenerate and commit `web/openapi.json` + generated types.
7. For daemon- or timer-visible changes: a short manual smoke test described in
   the PR (what you ran, what you saw).

### Working Rules for Agents

- Work **one plan item at a time**; respect phase dependencies.
- Do not change wire format (`protocol`) or DB schema outside the plan item that
  calls for it. Wire changes need updated tests and spec.
- Never edit `SCHEMA_SQL` alone for a schema change: existing databases need a
  `migrate()` step (and `CREATE TABLE IF NOT EXISTS` cannot alter CHECK
  constraints — rebuild the table).
- Do not add dependencies without a one-line justification in the PR.
- Found an unrelated bug? Add it to the plan's Backlog; don't fix it silently.
- Ask the owner (don't guess) when a decision in the spec's §11 table is
  contradicted by what you find.
- **Browser & UI Testing**: Follow `docs/playwright_mcp_nix.md`. When the user requests visual verification, run Playwright with `headless: false`, `slowMo: 120-150`, and display variables (`DISPLAY=:0 WAYLAND_DISPLAY=wayland-1`) so browser interactions and animations are visibly displayed on screen.

---

## Build & Run

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Start the daemon (foreground). Use a throw-away DB/socket for experiments:
LET_TIMER_DB=/tmp/lt-dev.db LET_TIMER_SOCKET=/tmp/lt-dev.sock cargo run --bin let-timer-daemon

# Use the CLI (daemon must be running; same env vars for the dev daemon)
cargo run --bin let-timer -- tasks list
cargo run --bin let-timer -- tasks create --name "Read chapter 3"
cargo run --bin let-timer -- workspaces create --name "Study"
```

Daily flow:

```bash
let-timer workspaces switch Study
let-timer tasks create -n "Read chapter 3" --estimated-mins 25 --scheduled-on today
let-timer tasks start          # pick a task; timer runs in the daemon
let-timer tasks current --format short    # ⏳ 24:10 Read chapter 3
let-timer tasks pause / resume / restart
let-timer tasks done           # only this completes a task
```

### Devenv (optional)

```bash
devenv shell   # drops you into a shell with Rust toolchain + deps
```

---

## Database Schema

Current: **v2**. Planned: **v3** (S2) and **v4** (S6). Keep this table in sync with `connection.rs`.

| Table              | Purpose                                     | Status |
|--------------------|---------------------------------------------|--------|
| `workspaces`       | Groups tasks into projects                  | v1 |
| `tasks`            | Core entity: name, priority, status, estimate, date | v1; v2 adds `paused` status, `run_no`, `notified_at` |
| `medias`           | Image / music / poem entries                | v1 |
| `media_list`       | Curated lists of media                      | v1 |
| `media_media_list` | Many-to-many join between media and lists   | v1 |
| `task_sessions`    | Running intervals per task run (source of all timer maths); unique partial index ⇒ ≤ 1 open session | v2 |
| `settings`         | Key/value; `active_workspace_id` (+ `timezone` in v3) | v2 |
| `api_tokens`       | Hashed API tokens: name, hash, created/last-used/revoked | v3 (planned) |

- `priority`: integer `0` (not-yet), `1` (immediate), `2` (urgent)
- `status`: text `pending | in-progress | paused | completed | cancelled` (`paused` from v2)
- `scheduled_on`: ISO date (`YYYY-MM-DD`) or `NULL` — added in migration v1.
- v4 (planned): `medias.content TEXT` for poems.
- Timestamps in v2+ tables: RFC 3339 UTC (`2026-10-12T08:30:00Z`).

---

## Adding a New Feature — Checklist

1. **Spec** — is it in `docs/product_spec.md`? If not, add/confirm it there first.
2. **Rules** — put business rules in `core::service` (pure functions where possible).
3. **Protocol** — add a variant to `Command` and/or `Response` in
   `crates/let-timer-core/src/protocol/mod.rs`.
4. **Repository** — implement the query in the appropriate `*_repository.rs`.
5. **Daemon** — add a thin adapter in `crates/let-timer-daemon/src/lib.rs` and
   one arm in `invoke_command` (`main.rs`). No logic in the `match`.
6. **CLI** — add the subcommand in `cli.rs`, wire it in `build_command`
   (`main.rs`), and add rendering in `render.rs`.
7. **Tests** — add unit tests in each touched file. Update the
   `every_command_the_daemon_has_is_reachable` test in `cli.rs`.
8. **Migration** — if the schema changes, bump `SCHEMA_VERSION` in
   `connection.rs`, add a migration step in `migrate()`, and add a migration test
   starting from the previous version's schema.

---

## Out of Scope (for now)

Terminal rendering of media; multi-user accounts / sharing (SD1); OAuth/OIDC;
recurring tasks; sub-tasks; horizontal scaling; native mobile apps; Windows.

---

## Dependencies (workspace-level)

| Crate      | Purpose                                 |
|------------|-----------------------------------------|
| `tokio`    | Async runtime, Unix sockets, signals, scheduler timers |
| `serde`    | Serialization (derive)                  |
| `serde_json` | JSON wire format for IPC             |
| `rusqlite` | SQLite (bundled build, chrono support)  |
| `clap`     | CLI argument parsing (derive)           |
| `inquire`  | Interactive terminal prompts            |
| `chrono`   | Date/time handling (UTC timestamps)     |
| `color-eyre` | Pretty error reports (CLI only)      |
