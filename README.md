# Let-timer

**Let-timer** is a Linux CLI daily task manager with a background daemon timer.
Schedule tasks per day (with optional estimated durations), start a task, and let the timer
run seamlessly inside the background daemon while task status tracks your work.

When an estimate runs out, the daemon sends a desktop notification via `notify-send` — the task is
**never** completed automatically. Only an explicit `done` command completes it.

---

## Features

- **Background Daemon & IPC**: A lightweight daemon owns the SQLite database (in WAL mode) and background timers, communicating with the CLI via a Unix domain socket.
- **Strict Invariants**:
  - At most **one running task globally** across all workspaces.
  - Nothing completes a task except an explicit `done` command.
  - Derived timer values (elapsed and remaining time are computed from session timestamps, never drift).
- **Workspace Scoping**: Organize tasks into workspaces (e.g. `Work`, `Personal`). Active workspace is daemon-owned.
- **Status Bar Integration**: Fast `let-timer tasks current --format short` prints single-line timer status suitable for `tmux`, Waybar, Polybar, or shell prompts, and exits 0 cleanly when idle or offline.
- **Media Lists**: Curated collections of images, music, and poems that future web frontends can display alongside active tasks.

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

---

## Installation & Build

### Prerequisites
- Stable Rust (2024 edition)
- SQLite (bundled by default via `rusqlite`)
- `libnotify` / `notify-send` (for desktop notifications)

### Building
```bash
# Clone the repository
git clone https://github.com/mikejohnpp/Let-timer.git
cd Let-timer

# Build workspace binaries (let-timer and let-timer-daemon)
cargo build --release

# Run test suite
cargo test --workspace
```

The resulting binaries will be in `target/release/let-timer` and `target/release/let-timer-daemon`.

---

## Running the Daemon

### 1. Manual / Foreground Run
```bash
cargo run --bin let-timer-daemon
# Or with release binary:
./target/release/let-timer-daemon
```

### 2. Using systemd User Service
Copy the provided service file to your systemd user units:
```bash
mkdir -p ~/.config/systemd/user
cp systemd/let-timer-daemon.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now let-timer-daemon.service

# Check status or restart
systemctl --user status let-timer-daemon.service
systemctl --user restart let-timer-daemon.service
```

### 3. Using Devenv
If using [devenv](https://devenv.sh/):
```bash
devenv up
```

---

## Daily Workflow Example

```bash
# 1. Check daemon status
let-timer daemon status

# 2. Switch to or create a workspace
let-timer workspaces create -n "Study"
let-timer workspaces switch Study

# 3. Create tasks for today with estimated durations
let-timer tasks create -n "Read chapter 3" -t 25 -o today -p immediate
let-timer tasks create -n "Implement exercises" -t 45 -o today -p urgent

# 4. View your schedule for today
let-timer tasks list --today

# 5. Start a task
# Pick interactively from pending tasks, or start highest priority task:
let-timer tasks start --next

# 6. Check current running work
let-timer tasks current
# Or format for status bar (e.g. Polybar/Waybar/tmux):
let-timer tasks current --format short
# Output: ⏳ 24:15 Read chapter 3

# 7. Pause, resume, or restart if needed
let-timer tasks pause
let-timer tasks resume

# 8. Mark done when finished
let-timer tasks done
```

---

## Documentation

- [docs/commands.md](docs/commands.md): Complete CLI command and flag reference.
- [docs/API_DESIGN.md](docs/API_DESIGN.md): Public HTTP REST & SSE API specification.
- [docs/product_spec.md](docs/product_spec.md): Product requirements and architecture decisions.
- [AGENTS.md](AGENTS.md): Development guide, domain invariants, and code conventions.

---

## License

GPL-3.0-or-later
