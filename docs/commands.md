# Let-timer Command Reference

This document provides a comprehensive reference for all CLI commands in **Let-timer**.

## Global Flags

The following flags can appear in any position:

- `--no-interactive`: Never prompt. Fall back to defaults for missing optional fields; error out if a required field is missing.
- `--json`: Output answers in machine-readable JSON directly from daemon responses.

If a component is invoked without a subcommand (e.g. `let-timer tasks`), it defaults to that component's `list` action.

---

## 1. `tasks` — Task Management

Tasks represent daily items to execute. Commands are scoped to the **active workspace** unless overridden.

### `tasks list`
List tasks in the schedule.

```bash
let-timer tasks list [FLAGS]
```

**Flags**:
- `--today`: Filter by today's date (`scheduled_on = today`).
- `--date <DATE>`: Filter by scheduled date: ISO date (`2026-10-12`), relative day (`today`, `tomorrow`), or weekday (`mon`..`sun`).
- `--status <STATUS>`: Filter by status: `pending`, `in-progress`, `paused`, `completed`, `cancelled`.
- `--priority-label <LABEL>`: Filter by priority: `urgent`, `immediate`, `not-yet`.
- `--sort-priority <asc|desc>`: Sort tasks by priority (`urgent` > `immediate` > `not-yet`).
- `-w, --workspace <NAME|ID>`: Scope list to a specific workspace.
- `--all-workspaces`: Include tasks across all workspaces.

---

### `tasks create`
Create a new task.

```bash
let-timer tasks create [FLAGS]
```

**Flags**:
- `-n, --name <NAME>`: Task title (required).
- `-d, --description <TEXT>`: Optional notes or description.
- `-p, --priority <urgent|immediate|not-yet>`: Priority label (default: `not-yet`).
- `-t, --estimated-mins <MINUTES>`: Estimated duration in minutes.
- `-o, --on <DATE|none>`: Scheduled day (`2026-10-12`, `today`, `tomorrow`, `mon`..`sun`, or `none`).
- `-w, --workspace-id <ID>`: Target workspace (default: active workspace).
- `-m, --media-list-id <ID|none>`: Associated media list.

Missing required fields prompt interactively unless `--no-interactive` is given.

---

### `tasks edit`
Modify fields on an existing task. Fields left out remain unchanged.

```bash
let-timer tasks edit --id <ID> [FLAGS]
```

**Flags**:
- `-i, --id <ID>`: Task ID (required).
- `-n, --name <NAME>`: New name.
- `-d, --description <TEXT>`: New description.
- `-p, --priority <urgent|immediate|not-yet>`: New priority.
- `-t, --estimated-mins <MINUTES>`: Update estimated minutes.
- `--clear-estimate`: Clear the estimate (sets to `NULL`).
- `-o, --on <DATE|none>`: Reschedule day or `none` to unschedule.
- `-m, --media-list-id <ID|none>`: Update or clear associated media list.

*(Note: `edit` cannot alter task `status`; all lifecycle transitions must go through dedicated lifecycle commands).*

---

### `tasks delete`
Delete a task by ID.

```bash
let-timer tasks delete --id <ID>
```

---

### `tasks find`
Search tasks by name substring within active or all workspaces.

```bash
let-timer tasks find <QUERY> [-w <WORKSPACE>] [--all-workspaces]
```

---

### `tasks current`
Report on the work currently in progress or paused.

```bash
let-timer tasks current [--format <text|short|json>]
```

**Formats**:
- `text` (default): Human-readable multi-line summary including elapsed time, remaining estimate, workspace name, and overtime status.
- `short`: Single-line status bar string (suitable for shell prompts, tmux, or status bars). Prints nothing and exits with code 0 if the daemon is idle or unreachable.
  - Running countdown: `⏳ 15:32 Read chapter 3`
  - Running without estimate: `▶ 12:40 Read chapter 3`
  - Overtime: `⚠ +02:15 Read chapter 3`
  - Paused: `⏸ 15:32 Read chapter 3`
  - Idle: `(empty)`
- `json`: Machine-readable snapshot JSON.

---

### `tasks start`
Start a task's timer in the daemon. Enforces the invariant of **at most one running task globally**.

```bash
let-timer tasks start [--id <ID>] [--next] [--switch]
```

- With `--id <ID>`: Starts the specified task.
- Without `--id`:
  - Interactive: Displays a picker showing candidates (pending and paused tasks in active workspace, sorted today first, priority desc, oldest first).
  - With `--next`: Automatically picks the top candidate without prompting.
- `--switch`: Automatically pauses any currently running task before starting.

---

### `tasks pause`
Pause the running task (or specified task). Preserves elapsed time.

```bash
let-timer tasks pause [--id <ID>]
```

---

### `tasks resume`
Resume a paused task (defaults to the most recently paused task).

```bash
let-timer tasks resume [--id <ID>]
```

---

### `tasks stop`
Stop the task. Transitions status back to `pending` while **keeping accumulated elapsed time** (Decision D1).

```bash
let-timer tasks stop [--id <ID>]
```

---

### `tasks restart`
Restart a task. Resets the timer back to 0:00 (creates a new run `run_no + 1`).

```bash
let-timer tasks restart [--id <ID>]
```

---

### `tasks done`
Mark a task as completed. **Nothing else completes a task.**

```bash
let-timer tasks done [--id <ID>] [--force]
```

- Allowed from `in-progress` and `paused`.
- Completing a task directly from `pending` (without having run it) requires `--force` (Decision D2).

---

### `tasks cancel`
Cancel a task.

```bash
let-timer tasks cancel [--id <ID>]
```

---

### `tasks reopen`
Reopen a completed or cancelled task back to `pending`.

```bash
let-timer tasks reopen --id <ID>
```

---

## 2. `workspaces` — Workspace Scoping

Workspaces group tasks. The active workspace is owned by the daemon.

### `workspaces list`
List all workspaces.

```bash
let-timer workspaces list
```

### `workspaces create`
Create a new workspace.

```bash
let-timer workspaces create -n <NAME> [-d <DESCRIPTION>]
```

### `workspaces switch`
Switch the active workspace by ID or name.

```bash
let-timer workspaces switch [TARGET]
```

If `TARGET` is omitted and running interactively, displays a select prompt of available workspaces.

### `workspaces current`
Show the currently active workspace.

```bash
let-timer workspaces current
```

### `workspaces delete`
Delete a workspace and cascade-delete all its tasks. Refused if it is the only workspace.

```bash
let-timer workspaces delete --id <ID>
```

---

## 3. `media-lists` — Curated Media Collections

### `media-lists list`
List all media collections.

```bash
let-timer media-lists list
```

### `media-lists create`
Create a media list.

```bash
let-timer media-lists create -n <NAME> [-d <DESCRIPTION>]
```

### `media-lists show`
Display details and member items of a media list.

```bash
let-timer media-lists show --id <ID>
```

### `media-lists add-media`
Attach a media item to a media list.

```bash
let-timer media-lists add-media -l <LIST_ID> -m <MEDIA_ID>
```

### `media-lists remove-media`
Remove a media item from a media list.

```bash
let-timer media-lists remove-media -l <LIST_ID> -m <MEDIA_ID>
```

### `media-lists delete`
Delete a media list. Removes all join entries and clears `tasks.media_list_id` to `NULL`.

```bash
let-timer media-lists delete --id <ID>
```

---

## 4. `medias` — Media Items

### `medias list`
List media items, optionally filtered by type.

```bash
let-timer medias list [-t <image|music|poem>]
```

### `medias create`
Create a new media record.

```bash
let-timer medias create -n <NAME> -u <URL> -t <image|music|poem> [-d <DESCRIPTION>]
```

### `medias delete`
Delete a media record. Cascades and cleans up memberships in media lists.

```bash
let-timer medias delete --id <ID>
```

---

## 5. `daemon` — Daemon Management

### `daemon status`
Query the running daemon for health status, version, socket path, database location, and active task.

```bash
let-timer daemon status
```

To restart the background daemon, use your service manager:
```bash
systemctl --user restart let-timer-daemon
```
