use utoipa::OpenApi;

use crate::dto::*;
use crate::rest::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        list_tasks,
        create_task,
        current_task,
        start_candidates,
        get_task,
        update_task,
        delete_task,
        start_task,
        pause_task,
        resume_task,
        stop_task,
        restart_task,
        done_task,
        cancel_task,
        reopen_task,
        get_timer,
        timer_pause,
        timer_resume,
        timer_stop,
        timer_done,
        list_workspaces,
        create_workspace,
        current_workspace,
        switch_workspace,
        delete_workspace,
        list_medias,
        create_media,
        delete_media,
        list_media_lists,
        create_media_list,
        get_media_list,
        delete_media_list,
        add_media_to_list,
        remove_media_from_list,
        get_timezone,
        set_timezone,
    ),
    components(
        schemas(
            CreateTaskDto,
            UpdateTaskDto,
            CreateWorkspaceDto,
            SwitchWorkspaceDto,
            CreateMediaListDto,
            AddMediaItemDto,
            CreateMediaDto,
            SetTimezoneDto,
            TaskDto,
            WorkspaceDto,
            MediaDto,
            MediaListDto,
            MediaListDetailDto,
            TimerSnapshotDto,
            TimezoneDto,
            HealthDto,
            ErrorDetailDto,
            ErrorEnvelopeDto,
        )
    ),
    tags(
        (name = "Tasks", description = "Task management and search"),
        (name = "Lifecycle", description = "Task timer lifecycle operations"),
        (name = "Timer", description = "Global active timer controls"),
        (name = "Workspaces", description = "Workspace scoping and switching"),
        (name = "Medias", description = "Media library management"),
        (name = "MediaLists", description = "Curated media list management"),
        (name = "Settings", description = "User & server settings"),
    ),
    info(
        title = "Let-timer REST API",
        version = "1.0.0",
        description = "RESTful HTTP API and OpenAPI specification for Let-timer daemon & web app"
    )
)]
pub struct ApiDoc;
