# Let-timer Public HTTP API Design (v1)

> [!WARNING]
> **Superseded by `docs/server_web_spec.md` §4 and the generated `openapi.json`**. Kept for historical reference.

This document specifies the public HTTP API for **Let-timer**, designed in Phase P8.
It outlines the endpoints, data transfer objects (DTOs), authentication, event streaming,
and the mapping between the internal domain engine (`let_timer_core::service`) and HTTP.

---

## 1. Overview & Architectural Principles

1. **Transport Agnostic Domain**:
   All business logic and invariants live in `let_timer_core::service`. The future HTTP API server
   (implemented in a dedicated crate, e.g. `let-timer-http`) acts as a thin adapter over `TaskService`,
   identical to how the Unix socket IPC handler (`let-timer-daemon`) interacts with it.
2. **Localhost & Security**:
   The HTTP server binds to `127.0.0.1` (or `[::1]`) by default. Binding to non-local interfaces
   requires an explicit `--bind` flag and configured bearer token authentication.
3. **CORS Allow-List**:
   Permitted origins for web UI clients are restricted (e.g., `http://localhost:3000`, `http://localhost:5173`).
   Wildcard `*` is prohibited when credentials/bearer tokens are enabled.
4. **Stable Versioned DTOs**:
   All request and response JSON payloads are versioned under the `/api/v1` namespace. Fields are
   frozen to ensure backward compatibility for external web and mobile clients.

---

## 2. Server Configuration & Authentication

- **Base URL**: `http://127.0.0.1:8765/api/v1`
- **Headers**:
  - `Content-Type: application/json`
  - `Accept: application/json`
- **Authentication**:
  - Optional for local unix user on `127.0.0.1`.
  - When enabled: `Authorization: Bearer <api-token>` header required.
  - HTTP 401 Unauthorized returned if token is invalid or missing.

---

## 3. Command ↔ HTTP Mapping Table

Every command in `let_timer_core::protocol::Command` maps directly to an HTTP endpoint:

| `let_timer_core::Command` | HTTP Method & Path | Description |
|---------------------------|--------------------|-------------|
| `Ping` | `GET /api/v1/health` | Daemon ping / health check |
| `Create(NewTask)` | `POST /api/v1/tasks` | Create task |
| `Delete { id }` | `DELETE /api/v1/tasks/{id}` | Delete task |
| `Update { id, update }` | `PATCH /api/v1/tasks/{id}` | Update task fields (name, estimate, date, media_list) |
| `Find(query)` | `GET /api/v1/tasks?q={query}` | Search tasks by name substring |
| `List(ListQuery)` | `GET /api/v1/tasks` | List tasks with filters (`status`, `date`, `workspace`, etc.) |
| `Current` | `GET /api/v1/timer` | Get snapshot of running/paused task |
| `Start { id }` | `POST /api/v1/tasks/{id}/start` | Start task timer |
| `StartNext` | `POST /api/v1/tasks/start-next` | Start highest priority scheduled task |
| `StartCandidates { workspace }` | `GET /api/v1/tasks/start-candidates` | Ordered list of candidate tasks to start |
| `Pause { id }` | `POST /api/v1/tasks/{id}/pause` (or `POST /api/v1/timer/pause`) | Pause current or specified task |
| `Resume { id }` | `POST /api/v1/tasks/{id}/resume` (or `POST /api/v1/timer/resume`) | Resume task |
| `Stop { id }` | `POST /api/v1/tasks/{id}/stop` | Stop task (keeps elapsed) |
| `Restart { id }` | `POST /api/v1/tasks/{id}/restart` | Restart task (resets run_no/elapsed) |
| `Done { id, force }` | `POST /api/v1/tasks/{id}/done?force={bool}` | Mark task as completed |
| `Cancel { id }` | `POST /api/v1/tasks/{id}/cancel` | Cancel task |
| `Reopen { id }` | `POST /api/v1/tasks/{id}/reopen` | Reopen completed/cancelled task |
| `ListWorkspaces` | `GET /api/v1/workspaces` | List all workspaces |
| `CreateWorkspace(NewWorkspace)` | `POST /api/v1/workspaces` | Create workspace |
| `SwitchWorkspace { target }` | `POST /api/v1/workspaces/active` | Switch active workspace (body: `{ "id": 1 }` or `{ "name": "Work" }`) |
| `CurrentWorkspace` | `GET /api/v1/workspaces/active` | Get active workspace |
| `DeleteWorkspace { id }` | `DELETE /api/v1/workspaces/{id}` | Delete workspace |
| `ListMediaLists` | `GET /api/v1/media-lists` | List all media lists |
| `CreateMediaList(NewMediaList)` | `POST /api/v1/media-lists` | Create a media list |
| `GetMediaList { id }` | `GET /api/v1/media-lists/{id}` | Get media list and all its media items |
| `DeleteMediaList { id }` | `DELETE /api/v1/media-lists/{id}` | Delete media list |
| `AddMediaToList { media_list_id, media_id }` | `POST /api/v1/media-lists/{id}/medias` | Attach media item to list (`{ "media_id": 2 }`) |
| `RemoveMediaFromList { media_list_id, media_id }` | `DELETE /api/v1/media-lists/{id}/medias/{media_id}` | Detach media item from list |
| `ListMedias { media_type }` | `GET /api/v1/medias?type={image\|music\|poem}` | List media items |
| `CreateMedia(NewMedia)` | `POST /api/v1/medias` | Create media item |
| `DeleteMedia { id }` | `DELETE /api/v1/medias/{id}` | Delete media item |

---

## 4. Endpoints Detail

### 4.1 Tasks & Lifecycle

#### `GET /api/v1/tasks`
Query parameters:
- `workspace`: Workspace ID or name (default: active workspace). Set to `all` for all workspaces.
- `status`: `pending | in-progress | paused | completed | cancelled`
- `priority`: `urgent | immediate | not-yet`
- `sort`: `priority_asc | priority_desc`
- `date`: `today | tomorrow | YYYY-MM-DD`
- `q`: Search substring

Response: `200 OK`
```json
[
  {
    "id": 1,
    "workspace_id": 1,
    "media_list_id": 2,
    "name": "Read chapter 3",
    "description": "Pages 45-80",
    "priority": 1,
    "status": "pending",
    "estimated_mins": 25,
    "scheduled_on": "2026-10-12",
    "run_no": 0,
    "notified_at": null,
    "created_at": "2026-10-12T08:00:00Z",
    "updated_at": "2026-10-12T08:00:00Z"
  }
]
```

#### `POST /api/v1/tasks/{id}/start`
Query parameters:
- `switch`: `boolean` (if true, automatically pauses any currently running task before starting this task)

Response: `200 OK`
```json
{
  "task": {
    "id": 1,
    "name": "Read chapter 3",
    "status": "in-progress"
  },
  "timer": {
    "task_id": 1,
    "task_name": "Read chapter 3",
    "state": "running",
    "run_no": 1,
    "elapsed_seconds": 0,
    "estimated_seconds": 1500,
    "remaining_seconds": 1500,
    "overtime_seconds": 0
  }
}
```

#### `POST /api/v1/tasks/{id}/pause`
Response: `200 OK` with updated `TimerSnapshot`.

#### `POST /api/v1/tasks/{id}/done`
Query parameters:
- `force`: `boolean` (required if completing directly from `pending` without running)

Response: `200 OK`
```json
{
  "status": "completed"
}
```

---

### 4.2 Timer & Real-Time Event Stream

#### `GET /api/v1/timer`
Returns the current active timer snapshot across all workspaces (R4 domain invariant: at most one running task globally).

Response: `200 OK`
```json
{
  "task_id": 1,
  "task_name": "Read chapter 3",
  "workspace_id": 1,
  "workspace_name": "Default",
  "media_list_id": 2,
  "state": "running",
  "run_no": 1,
  "elapsed_seconds": 640,
  "estimated_seconds": 1500,
  "remaining_seconds": 860,
  "overtime_seconds": 0
}
```
If idle, returns `200 OK` with `"state": "idle"` and `null` task details.

#### `GET /api/v1/timer/events` (Server-Sent Events)
An SSE endpoint (`Content-Type: text/event-stream`) fed by the daemon scheduler and task transition hooks.

Emitted events:
- `timer_tick`: Emitted every 1 second while a task is running (or clients can derive ticks locally and sync on state events).
- `task_transition`: Emitted on state change (`start`, `pause`, `resume`, `stop`, `done`, `cancel`).
- `timer_expired`: Emitted when `remaining_seconds` reaches 0 (triggering notifications in UI).
- `workspace_changed`: Emitted when active workspace switches.

Example event payload:
```
event: task_transition
data: {"action":"pause","task_id":1,"state":"paused","elapsed_seconds":640}

event: timer_expired
data: {"task_id":1,"task_name":"Read chapter 3","overtime_seconds":0}
```

---

### 4.3 Media & MediaLists (R9)

#### `GET /api/v1/media-lists/{id}`
Response: `200 OK`
```json
{
  "list": {
    "id": 1,
    "name": "Focus Backgrounds",
    "description": "Inspiring art and music"
  },
  "items": [
    {
      "id": 1,
      "name": "Rain on Leaves",
      "url": "https://example.com/rain.mp3",
      "media_type": "music",
      "description": "Gentle rain audio"
    },
    {
      "id": 2,
      "name": "Misty Mountain",
      "url": "https://example.com/mist.jpg",
      "media_type": "image",
      "description": null
    }
  ]
}
```

---

## 5. Future Schema Extension (Poems & Content)

As identified in spec §9.5 and §10:
`Media.url` is currently a URL string. When the web frontend is built to display poems directly,
a schema migration (`v3`) will add a nullable `content TEXT` column to the `medias` table:
```sql
ALTER TABLE medias ADD COLUMN content TEXT;
```
For `media_type = 'poem'`, the `content` field will carry the full inline poem text, while `url` remains
an optional external source link.
