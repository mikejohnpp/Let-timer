# Let-timer Public HTTP API Design (v1)

> [!NOTE]
> **This document is superseded by [`docs/server_web_spec.md`](./server_web_spec.md) §4 and the machine-generated OpenAPI specification at [`web/openapi.json`](../web/openapi.json).**

This document was originally authored in Phase P8. For the authoritative API reference and schemas, see `web/openapi.json` or run the server and view `/swagger-ui/`.

---

## Command ↔ REST & RPC Mapping Table

Every command in `let_timer_core::protocol::Command` is reachable either via `POST /api/v1/rpc` (lossless CLI parity) or via idiomatic REST endpoints:

| `let_timer_core::Command` | HTTP Method & Path | Description |
|---------------------------|--------------------|-------------|
| `Ping` | `GET /api/v1/health` (or `/rpc`) | Daemon health check & version info |
| `Create(NewTask)` | `POST /api/v1/tasks` | Create task |
| `Delete { id }` | `DELETE /api/v1/tasks/{id}` | Delete task |
| `Edit { id, update }` | `PATCH /api/v1/tasks/{id}` | Edit task fields (name, estimate, date, priority, media list) |
| `Find { query, workspace }` | `GET /api/v1/tasks?q={query}&workspace={scope}` | Search tasks by name substring |
| `List(ListQuery)` | `GET /api/v1/tasks` | List tasks with filters (`status`, `date`, `priority`, `sort`, `workspace`) |
| `Current` | `GET /api/v1/tasks/current` or `GET /api/v1/timer` | Get active task timer snapshot |
| `Start { id, next, switch }` | `POST /api/v1/tasks/{id}/start?switch={bool}` | Start task timer |
| `StartCandidates { workspace }` | `GET /api/v1/tasks/start-candidates` | Ordered list of candidate tasks to start |
| `Pause { id }` | `POST /api/v1/tasks/{id}/pause` or `POST /api/v1/timer/pause` | Pause task |
| `Resume { id }` | `POST /api/v1/tasks/{id}/resume` or `POST /api/v1/timer/resume` | Resume task |
| `Stop { id }` | `POST /api/v1/tasks/{id}/stop` or `POST /api/v1/timer/stop` | Stop task (keeps elapsed) |
| `Restart { id }` | `POST /api/v1/tasks/{id}/restart` | Restart task (resets run_no/elapsed) |
| `Done { id, force }` | `POST /api/v1/tasks/{id}/done?force={bool}` or `POST /api/v1/timer/done` | Mark task as completed |
| `Cancel { id }` | `POST /api/v1/tasks/{id}/cancel` | Cancel task |
| `Reopen { id }` | `POST /api/v1/tasks/{id}/reopen` | Reopen completed/cancelled task |
| `ListWorkspaces` | `GET /api/v1/workspaces` | List all workspaces |
| `CreateWorkspace(NewWorkspace)` | `POST /api/v1/workspaces` | Create workspace |
| `SwitchWorkspace { target }` | `POST /api/v1/workspaces/switch` | Switch active workspace (body: `{ "id": 1 }` or `{ "name": "Work" }`) |
| `CurrentWorkspace` | `GET /api/v1/workspaces/current` | Get active workspace |
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
| `GetTimezone` | `GET /api/v1/settings/timezone` | Get server configured timezone |
| `SetTimezone(tz)` | `PUT /api/v1/settings/timezone` | Set server configured timezone |

---

## Authentication & Headers

- Non-loopback requests require an active API token via `Authorization: Bearer <token>` or `X-Let-Timer-Token: <token>`.
- Client scoping headers:
  - `X-Let-Timer-Workspace`: Workspace ID or name override for the request.
  - `X-Let-Timer-Timezone`: IANA timezone name override for deriving "today".
  - `X-Let-Timer-Protocol: 1`: Required for `/api/v1/rpc` calls.
