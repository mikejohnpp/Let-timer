//! `let-timer`: the interface, and the questions that can be asked of it from a
//! shell.
//!
//! Every run is a question with an answer that gets printed: the terminal is
//! left exactly as it was found, which is what makes these lines the ones a
//! script can run. A command that leaves a field out asks the user for it,
//! unless `--no-interactive` was passed.
//!
//! The component is read first, because it is what the question acts on.
//! `tasks find` and `workspaces list` reach different lists, and there is
//! no line that reaches either one without saying which it meant.
//!
//! A component has to be named: clap refuses a line that names nothing before
//! any of this program's code runs.

mod cli;
mod config;
mod interactive;
mod render;

use std::process::ExitCode;

use crate::cli::{
    Cli, Commands, DaemonCommand, MediaCommand, MediaListCommand, ServerCommand, TaskCommand,
    WorkspaceCommand,
};
use crate::interactive::{
    resolve_description, resolve_estimated_mins, resolve_media_list, resolve_name,
    resolve_priority, resolve_scheduled_on, resolve_workspace,
};
use clap::Parser;
use let_timer_core::{
    Client, Command, ListQuery, MediaType, NewMedia, NewMediaList, NewTask, NewWorkspace, Priority,
    Response, SortOrder, TaskStatus, UpdateTask, WorkspaceScope, WorkspaceTarget, date_on,
};

/// Anything that stopped the program from doing what it was asked.
#[derive(Debug)]
enum Failure {
    /// The daemon could not be reached at all.
    Daemon(String),
    /// The daemon was reached and said no.
    Refused(String),
    /// What was asked for was not usable: a bad date, a missing name.
    Usage(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Daemon(message) => write!(f, "cannot reach the daemon: {message}"),
            Failure::Refused(message) => write!(f, "{message}"),
            Failure::Usage(message) => write!(f, "{message}"),
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli).await {
        Ok(()) => ExitCode::SUCCESS,
        // Everything a person can do something about is said to them; the
        // daemon's own complaints have already been printed by `render`.
        Err(failure) => {
            render::complain(&failure.to_string());
            ExitCode::FAILURE
        }
    }
}

/// Show the interface, or do what the command says.
///
/// The component is read first and decides everything: nothing reaches the
/// daemon without one having been named, because naming it is what decides
/// which commands exist.
/// Answer the question.
///
/// The component was read first and decides everything: nothing reaches the
/// daemon without one having been named, because naming it is what decides
/// which commands exist.
async fn run(cli: &Cli) -> Result<(), Failure> {
    once(cli, &cli.component.command()).await
}

/// Ask the daemon to do one thing, print the answer, and stop.
async fn once(cli: &Cli, command: &Commands) -> Result<(), Failure> {
    // Special handling for local `server` commands that don't need a daemon connection
    if let Commands::Server(server_cmd) = command {
        return handle_server_command(server_cmd, cli.json).await;
    }

    let (mut client, current_profile) =
        match connect(cli.server.as_deref(), cli.no_interactive).await {
            Ok(c) => c,
            Err(e) => {
                if let Commands::Task(TaskCommand::Current { format }) = command
                    && format == "short"
                {
                    return Ok(());
                }
                return Err(e);
            }
        };

    // Special handling for `workspaces switch` on remote profile: updates profile pin
    if let Commands::Workspace(WorkspaceCommand::Switch { target }) = command
        && let Some(ref profile_name) = current_profile
        && let Some(target_val) = target
    {
        let mut cfg = config::Config::load();
        if let Some(prof) = cfg.servers.get_mut(profile_name) {
            prof.workspace = Some(target_val.clone());
            cfg.save().map_err(|e| Failure::Usage(e.to_string()))?;
            println!("pinned workspace '{target_val}' for server '{profile_name}'");
            return Ok(());
        }
    }

    // Special handling for `medias create --file <PATH>`: uploads directly (local copy or HTTP multipart)
    if let Commands::Media(MediaCommand::Create {
        file: Some(file_path),
        name,
        media_type,
        description,
        ..
    }) = command
    {
        let res = client
            .upload_media(
                std::path::Path::new(file_path),
                name.as_deref(),
                media_type.as_deref(),
                description.as_deref(),
            )
            .await
            .map_err(|e| Failure::Daemon(e.to_string()))?;
        if let Response::Error { message } = &res {
            return Err(Failure::Refused(message.clone()));
        }
        render::print(&res, cli.json).map_err(|error| Failure::Usage(error.to_string()))?;
        return Ok(());
    }

    let command_proto = match build_command(command, &mut client, cli.no_interactive).await {
        Ok(c) => c,
        Err(error) => {
            if let Commands::Task(TaskCommand::Current { format }) = command
                && format == "short"
            {
                return Ok(());
            }
            return Err(Failure::Usage(error.to_string()));
        }
    };

    let response = match client.request(&command_proto).await {
        Ok(r) => r,
        Err(error) => {
            if let Commands::Task(TaskCommand::Current { format }) = command
                && format == "short"
            {
                return Ok(());
            }
            return Err(Failure::Daemon(error.to_string()));
        }
    };

    // The daemon's own complaint is the answer, and the exit code has to say so:
    // a script that cannot tell a failure from an empty list is a script that
    // will act on a failure.
    if let Response::Error { message } = &response {
        return Err(Failure::Refused(message.clone()));
    }

    if cli.json {
        return render::print(&response, true).map_err(|error| Failure::Usage(error.to_string()));
    }

    match (command, &response) {
        (Commands::Task(TaskCommand::Current { format }), _) => match format.as_str() {
            "short" => {
                let snap = match &response {
                    Response::Timer(snap) => Some(snap),
                    _ => None,
                };
                let line = render::render_current_short(snap);
                if !line.is_empty() {
                    println!("{line}");
                }
            }
            "json" => {
                render::print(&response, true)
                    .map_err(|error| Failure::Usage(error.to_string()))?;
            }
            _ => {
                render::print(&response, false)
                    .map_err(|error| Failure::Usage(error.to_string()))?;
            }
        },
        (Commands::Task(TaskCommand::Start { .. }), Response::Timer(snap)) => {
            println!("{}", render::render_lifecycle_confirmation("Started", snap));
        }
        (Commands::Task(TaskCommand::Pause { .. }), Response::Timer(snap)) => {
            println!("{}", render::render_lifecycle_confirmation("Paused", snap));
        }
        (Commands::Task(TaskCommand::Resume { .. }), Response::Timer(snap)) => {
            println!("{}", render::render_lifecycle_confirmation("Resumed", snap));
        }
        (Commands::Task(TaskCommand::Stop { .. }), Response::Timer(snap)) => {
            println!("{}", render::render_lifecycle_confirmation("Stopped", snap));
        }
        (Commands::Task(TaskCommand::Restart { .. }), Response::Timer(snap)) => {
            println!(
                "{}",
                render::render_lifecycle_confirmation("Restarted", snap)
            );
        }
        (Commands::Task(TaskCommand::Done { .. }), Response::Timer(snap)) => {
            println!(
                "{}",
                render::render_lifecycle_confirmation("Completed", snap)
            );
        }
        (Commands::Task(TaskCommand::Cancel { .. }), Response::Timer(snap)) => {
            println!(
                "{}",
                render::render_lifecycle_confirmation("Cancelled", snap)
            );
        }
        (Commands::Task(TaskCommand::Reopen { .. }), Response::Timer(snap)) => {
            println!(
                "{}",
                render::render_lifecycle_confirmation("Reopened", snap)
            );
        }
        _ => {
            render::print(&response, false).map_err(|error| Failure::Usage(error.to_string()))?;
        }
    }

    Ok(())
}

/// The daemon, or an explanation of why there isn't one.
async fn connect(
    server_flag: Option<&str>,
    no_interactive: bool,
) -> Result<(Client, Option<String>), Failure> {
    // 1. Resolve server profile name or URL
    // Order: --server flag -> LET_TIMER_SERVER env -> config.current -> "local"
    let config = config::Config::load();
    let target = server_flag
        .map(|s| s.to_string())
        .or_else(|| std::env::var("LET_TIMER_SERVER").ok())
        .or_else(|| config.current.clone())
        .unwrap_or_else(|| "local".to_string());

    if target == "local" || target.is_empty() {
        let path = let_timer_core::ipc::socket_path();
        let client = Client::connect_unix(&path)
            .await
            .map_err(|error| Failure::Daemon(error.to_string()))?;
        return Ok((client, None));
    }

    // Look up target in config or treat as direct URL
    let (profile_name, profile) = if let Some(p) = config.servers.get(&target) {
        (target.clone(), p.clone())
    } else if target.starts_with("http://") || target.starts_with("https://") {
        (
            target.clone(),
            config::ServerProfile {
                url: target.clone(),
                token: None,
                token_file: None,
                token_env: None,
                workspace: None,
            },
        )
    } else {
        return Err(Failure::Usage(format!("unknown server profile '{target}'")));
    };

    let token = config::Config::resolve_token(&profile);

    // Stderr banner in interactive mode when talking to non-local server
    if !no_interactive {
        eprintln!("→ {} ({})", profile_name, profile.url);
    }

    let client =
        Client::http(profile.url.clone(), token, profile.workspace.clone()).map_err(|e| {
            Failure::Daemon(format!(
                "failed to initialize http client for server {profile_name}: {e}"
            ))
        })?;

    Ok((client, Some(profile_name)))
}

async fn handle_server_command(cmd: &ServerCommand, json: bool) -> Result<(), Failure> {
    let mut config = config::Config::load();

    match cmd {
        ServerCommand::List => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&config.servers)
                        .map_err(|e| Failure::Usage(e.to_string()))?
                );
            } else {
                let current = config.current.as_deref().unwrap_or("local");
                println!("Active profile: {current}");
                println!("Configured servers:");
                println!("  * local (Unix domain socket)");
                for (name, prof) in &config.servers {
                    let pin = prof.workspace.as_deref().unwrap_or("(none)");
                    let marker = if name == current { ">" } else { " " };
                    println!("  {marker} {name}: {} [workspace: {pin}]", prof.url);
                }
            }
        }
        ServerCommand::Current => {
            let current = config.current.as_deref().unwrap_or("local");
            if json {
                println!("{{\"current\":\"{current}\"}}");
            } else {
                println!("{current}");
            }
        }
        ServerCommand::Use { name } => {
            if name != "local" && !config.servers.contains_key(name) {
                return Err(Failure::Usage(format!("unknown server profile '{name}'")));
            }
            config.current = if name == "local" {
                None
            } else {
                Some(name.clone())
            };
            config.save().map_err(|e| Failure::Usage(e.to_string()))?;
            println!("switched active server to '{name}'");
        }
        ServerCommand::Remove { name } => {
            if config.servers.remove(name).is_none() {
                return Err(Failure::Usage(format!("unknown server profile '{name}'")));
            }
            if config.current.as_deref() == Some(name) {
                config.current = None;
            }
            config.save().map_err(|e| Failure::Usage(e.to_string()))?;
            println!("removed server profile '{name}'");
        }
        ServerCommand::Add {
            name,
            url,
            token_file,
            token_env,
            token_stdin,
            workspace,
        } => {
            if name == "local" {
                return Err(Failure::Usage("cannot name a profile 'local'".to_string()));
            }

            let token = if *token_stdin {
                let prompt = inquire::Password::new("Enter API token:")
                    .without_confirmation()
                    .prompt()
                    .map_err(|e| Failure::Usage(e.to_string()))?;
                Some(prompt)
            } else {
                None
            };

            let profile = config::ServerProfile {
                url: url.clone(),
                token,
                token_file: token_file.clone(),
                token_env: token_env.clone(),
                workspace: workspace.clone(),
            };

            let resolved_token = config::Config::resolve_token(&profile);

            // Validate connection and health before saving
            let test_client = Client::http(url.clone(), resolved_token, None)
                .map_err(|e| Failure::Daemon(format!("failed to connect to {url}: {e}")))?;

            let mut test_c = test_client;
            let ping_res = test_c
                .request(&Command::Ping)
                .await
                .map_err(|e| Failure::Daemon(format!("server validation failed: {e}")))?;

            match ping_res {
                Response::Pong => {}
                _ => {
                    return Err(Failure::Daemon(
                        "unexpected response from server".to_string(),
                    ));
                }
            }

            config.servers.insert(name.clone(), profile);
            config.save().map_err(|e| Failure::Usage(e.to_string()))?;
            println!("successfully added server profile '{name}' ({url})");
        }
        ServerCommand::Status { name } => {
            let target_name = name
                .as_deref()
                .or(config.current.as_deref())
                .unwrap_or("local");

            if target_name == "local" {
                println!("Local server: Unix domain socket");
                let path = let_timer_core::ipc::socket_path();
                let mut client = Client::connect_unix(&path)
                    .await
                    .map_err(|e| Failure::Daemon(e.to_string()))?;
                let start = std::time::Instant::now();
                let res = client
                    .request(&Command::DaemonStatus)
                    .await
                    .map_err(|e| Failure::Daemon(e.to_string()))?;
                let latency = start.elapsed();
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&res).map_err(|e| Failure::Usage(e.to_string()))?
                    );
                } else {
                    println!("Status: OK, latency: {:?}", latency);
                }
            } else if let Some(prof) = config.servers.get(target_name) {
                let token = config::Config::resolve_token(prof);
                let mut client = Client::http(prof.url.clone(), token, None)
                    .map_err(|e| Failure::Daemon(e.to_string()))?;
                let start = std::time::Instant::now();
                let res = client
                    .request(&Command::Ping)
                    .await
                    .map_err(|e| Failure::Daemon(e.to_string()))?;
                let latency = start.elapsed();
                if json {
                    println!(
                        "{{\"server\":\"{target_name}\",\"status\":\"ok\",\"latency_ms\":{}}}",
                        latency.as_millis()
                    );
                } else {
                    println!(
                        "Server '{}' ({}) - Status: {:?}, Latency: {:?}",
                        target_name, prof.url, res, latency
                    );
                }
            } else {
                return Err(Failure::Usage(format!(
                    "unknown server profile '{target_name}'"
                )));
            }
        }
    }

    Ok(())
}

/// Turn a parsed command into the protocol's, prompting for whatever was left
/// out.
///
/// Order: name -> scheduled day -> workspace -> media list -> priority ->
/// description -> estimated mins. The order matters: the first two are the only
/// ones a task cannot do without, so asking about them first means a `no` to
/// anything else has cost the user nothing.
async fn build_command(
    command: &Commands,
    client: &mut Client,
    no_interactive: bool,
) -> Result<Command, Box<dyn std::error::Error>> {
    match command {
        Commands::Task(TaskCommand::Create {
            name,
            on,
            workspace_id,
            media_list_id,
            description,
            priority,
            estimated_mins,
        }) => Ok(Command::Create(
            build_new_task(
                client,
                no_interactive,
                name,
                on,
                workspace_id,
                media_list_id,
                description,
                priority,
                estimated_mins,
            )
            .await?,
        )),

        Commands::Task(TaskCommand::Delete { id }) => Ok(Command::Delete { id: *id }),

        Commands::Task(TaskCommand::Edit {
            id,
            name,
            description,
            priority,
            estimated_mins,
            clear_estimate,
            media_list_id,
            on,
        }) => Ok(Command::Edit {
            id: *id,
            update: UpdateTask {
                name: name.clone(),
                description: match description {
                    Some(description) if description.is_empty() => Some(None),
                    Some(description) => Some(Some(description.clone())),
                    None => None,
                },
                priority: match priority {
                    Some(priority) => Some(priority.parse()?),
                    None => None,
                },
                estimated_mins: if *clear_estimate {
                    Some(None)
                } else {
                    estimated_mins.map(Some)
                },
                media_list_id: match media_list_id.as_deref() {
                    Some("none") => Some(None),
                    Some(id_str) => match id_str.parse::<i64>() {
                        Ok(id) => Some(Some(id)),
                        Err(_) => return Err("invalid media list ID".into()),
                    },
                    None => None,
                },
                scheduled_on: match on.as_deref() {
                    Some("none") => Some(None),
                    Some(value) => Some(date_on(value)?),
                    None => None,
                },
                ..Default::default()
            },
        }),

        Commands::Task(TaskCommand::Find {
            query,
            workspace,
            all_workspaces,
        }) => {
            let ws_scope = if *all_workspaces {
                WorkspaceScope::All
            } else if let Some(ws) = workspace {
                if let Ok(id) = ws.parse::<i64>() {
                    WorkspaceScope::Id(id)
                } else {
                    WorkspaceScope::Name(ws.clone())
                }
            } else {
                WorkspaceScope::Active
            };
            Ok(Command::Find {
                query: query.clone(),
                workspace: ws_scope,
            })
        }

        Commands::Task(TaskCommand::List {
            sort_priority,
            priority_label,
            status,
            today,
            date,
            workspace,
            all_workspaces,
        }) => {
            let ws_scope = if *all_workspaces {
                WorkspaceScope::All
            } else if let Some(ws) = workspace {
                if let Ok(id) = ws.parse::<i64>() {
                    WorkspaceScope::Id(id)
                } else {
                    WorkspaceScope::Name(ws.clone())
                }
            } else {
                WorkspaceScope::Active
            };

            let date = if *today {
                date_on("today")?
            } else if let Some(d) = date {
                date_on(d)?
            } else {
                None
            };

            Ok(Command::List(ListQuery {
                workspace: ws_scope,
                sort_priority: match sort_priority.as_deref() {
                    Some("ascending") => Some(SortOrder::Ascending),
                    Some("descending") => Some(SortOrder::Descending),
                    Some(other) => return Err(format!("invalid sort order: {other}").into()),
                    None => None,
                },
                status: match status.as_deref() {
                    Some("pending") => Some(TaskStatus::Pending),
                    Some("in-progress") => Some(TaskStatus::InProgress),
                    Some("paused") => Some(TaskStatus::Paused),
                    Some("completed") => Some(TaskStatus::Completed),
                    Some("cancelled") => Some(TaskStatus::Cancelled),
                    Some(other) => return Err(format!("invalid status: {other}").into()),
                    None => None,
                },
                priority: match priority_label.as_deref() {
                    Some(p) => Some(p.parse()?),
                    None => None,
                },
                date,
                overdue: None,
            }))
        }

        Commands::Task(TaskCommand::Current { .. }) => Ok(Command::Current),
        Commands::Task(TaskCommand::Start { id, next, switch }) => {
            let id = match id {
                Some(id) => Some(*id),
                None => {
                    if *next {
                        None
                    } else if no_interactive {
                        return Err(
                            "specify a task ID or use --next (--no-interactive requires id or --next)"
                                .into(),
                        );
                    } else {
                        let res = client
                            .request(&Command::StartCandidates {
                                workspace: WorkspaceScope::Active,
                            })
                            .await
                            .map_err(|e| format!("cannot list candidate tasks: {e}"))?;
                        match res {
                            Response::OkList(tasks) if !tasks.is_empty() => {
                                let options: Vec<String> = tasks
                                    .iter()
                                    .map(|t| {
                                        let est = t
                                            .estimated_mins
                                            .map(|m| format!(" [{m}m]"))
                                            .unwrap_or_default();
                                        let date = t
                                            .scheduled_on
                                            .map(|d| format!(" ({d})"))
                                            .unwrap_or_default();
                                        format!(
                                            "#{} {} [{}]{est}{date}",
                                            t.id,
                                            t.name,
                                            t.priority.as_str()
                                        )
                                    })
                                    .collect();
                                let ans = inquire::Select::new("Select task to start:", options)
                                    .prompt()
                                    .map_err(|e| e.to_string())?;
                                let id_str = ans
                                    .strip_prefix('#')
                                    .and_then(|s| s.split_whitespace().next())
                                    .ok_or_else(|| "failed to parse task id".to_string())?;
                                let id = id_str.parse::<i64>().map_err(|e| e.to_string())?;
                                Some(id)
                            }
                            Response::OkList(_) => {
                                return Err(
                                    "no pending or paused tasks found in active workspace".into()
                                );
                            }
                            Response::Error { message } => return Err(message.into()),
                            _ => return Err("unexpected response listing candidate tasks".into()),
                        }
                    }
                }
            };
            Ok(Command::Start {
                id,
                next: *next,
                switch: *switch,
            })
        }
        Commands::Task(TaskCommand::Pause { id }) => Ok(Command::Pause { id: *id }),
        Commands::Task(TaskCommand::Resume { id }) => Ok(Command::Resume { id: *id }),
        Commands::Task(TaskCommand::Stop { id }) => Ok(Command::Stop { id: *id }),
        Commands::Task(TaskCommand::Restart { id }) => Ok(Command::Restart { id: *id }),
        Commands::Task(TaskCommand::Done { id, force }) => Ok(Command::Done {
            id: *id,
            force: *force,
        }),
        Commands::Task(TaskCommand::Cancel { id }) => Ok(Command::Cancel { id: *id }),
        Commands::Task(TaskCommand::Reopen { id }) => Ok(Command::Reopen { id: *id }),
        Commands::Daemon(DaemonCommand::Status) => Ok(Command::DaemonStatus),

        // The other two components answer with their own records, so these
        // need nothing from the task builder.
        Commands::Workspace(WorkspaceCommand::List) => Ok(Command::ListWorkspaces),
        Commands::Workspace(WorkspaceCommand::Create { name, description }) => {
            let (name, description) = build_named(
                "workspace",
                "Workspace name",
                name,
                description,
                no_interactive,
            )?;
            Ok(Command::CreateWorkspace(NewWorkspace { name, description }))
        }
        Commands::Workspace(WorkspaceCommand::Switch { target }) => {
            let target = match target {
                Some(t) => {
                    if let Ok(id) = t.parse::<i64>() {
                        WorkspaceTarget::Id(id)
                    } else {
                        WorkspaceTarget::Name(t.clone())
                    }
                }
                None => {
                    if no_interactive {
                        return Err(
                            "missing workspace target (--no-interactive requires id or name)"
                                .into(),
                        );
                    }
                    let res = client
                        .request(&Command::ListWorkspaces)
                        .await
                        .map_err(|e| format!("cannot list workspaces: {e}"))?;
                    match res {
                        Response::WorkspaceList(workspaces) if !workspaces.is_empty() => {
                            let options: Vec<String> = workspaces
                                .iter()
                                .map(|w| format!("{} (#{})", w.name, w.id))
                                .collect();
                            let ans =
                                inquire::Select::new("Select workspace to switch to:", options)
                                    .prompt()
                                    .map_err(|e| e.to_string())?;
                            let id_str = ans
                                .rsplit("(#")
                                .next()
                                .and_then(|s| s.strip_suffix(')'))
                                .ok_or_else(|| "failed to parse selected workspace".to_string())?;
                            let id = id_str.parse::<i64>().map_err(|e| e.to_string())?;
                            WorkspaceTarget::Id(id)
                        }
                        Response::WorkspaceList(_) => {
                            return Err("no workspaces available".into());
                        }
                        Response::Error { message } => return Err(message.into()),
                        _ => return Err("unexpected response listing workspaces".into()),
                    }
                }
            };
            Ok(Command::SwitchWorkspace { target })
        }
        Commands::Workspace(WorkspaceCommand::Current) => Ok(Command::CurrentWorkspace),
        Commands::Workspace(WorkspaceCommand::Delete { id }) => {
            if !no_interactive {
                let confirmed =
                    inquire::Confirm::new(&format!("Delete workspace #{id} and all its tasks?"))
                        .with_default(false)
                        .prompt()
                        .map_err(|e| e.to_string())?;
                if !confirmed {
                    return Err("aborted by user".into());
                }
            }
            Ok(Command::DeleteWorkspace { id: *id })
        }
        Commands::MediaList(MediaListCommand::List) => Ok(Command::ListMediaLists),
        Commands::MediaList(MediaListCommand::Create { name, description }) => {
            let (name, description) = build_named(
                "media list",
                "Media list name",
                name,
                description,
                no_interactive,
            )?;
            Ok(Command::CreateMediaList(NewMediaList { name, description }))
        }
        Commands::MediaList(MediaListCommand::Show { id }) => Ok(Command::GetMediaList { id: *id }),
        Commands::MediaList(MediaListCommand::Delete { id }) => {
            if !no_interactive {
                let confirmed = inquire::Confirm::new(&format!("Delete media list #{id}?"))
                    .with_default(false)
                    .prompt()
                    .map_err(|e| e.to_string())?;
                if !confirmed {
                    return Err("aborted by user".into());
                }
            }
            Ok(Command::DeleteMediaList { id: *id })
        }
        Commands::MediaList(MediaListCommand::AddMedia { list_id, media_id }) => {
            Ok(Command::AddMediaToList {
                media_list_id: *list_id,
                media_id: *media_id,
            })
        }
        Commands::MediaList(MediaListCommand::RemoveMedia { list_id, media_id }) => {
            Ok(Command::RemoveMediaFromList {
                media_list_id: *list_id,
                media_id: *media_id,
            })
        }
        Commands::Media(MediaCommand::List { media_type }) => {
            let media_type = match media_type {
                Some(t) => Some(
                    t.parse::<MediaType>()
                        .map_err(|e| format!("invalid media type: {t} ({e})"))?,
                ),
                None => None,
            };
            Ok(Command::ListMedias { media_type })
        }
        Commands::Media(MediaCommand::Create {
            name,
            file: _,
            url,
            media_type,
            description,
            content,
        }) => {
            let name = match required(name, "media", "--name", no_interactive) {
                Asked::Given(n) => n.to_string(),
                Asked::Refuse(msg) => return Err(msg.into()),
                Asked::Ask => resolve_name("Media name", "e.g. Ambient Focus")?,
            };
            let url = match required(url, "media", "--url", no_interactive) {
                Asked::Given(u) => u.to_string(),
                Asked::Refuse(msg) => return Err(msg.into()),
                Asked::Ask => inquire::Text::new("Media URL")
                    .with_help_message("e.g. https://example.com/audio.mp3")
                    .prompt()
                    .map_err(|e| e.to_string())?,
            };
            let media_type: MediaType = match media_type {
                Some(t) => t
                    .parse()
                    .map_err(|e| format!("invalid media type: {t} ({e})"))?,
                None => {
                    if no_interactive {
                        return Err(
                            "a media needs a type: pass --media-type <image|music|poem>".into()
                        );
                    }
                    let chosen = inquire::Select::new("Media type", vec!["image", "music", "poem"])
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    chosen
                        .parse()
                        .map_err(|e: let_timer_core::DbError| e.to_string())?
                }
            };
            let description = match description {
                Some(d) if !d.trim().is_empty() => Some(d.trim().to_string()),
                Some(_) => None,
                None if no_interactive => None,
                None => resolve_description()?,
            };
            let content = match content {
                Some(c) if !c.trim().is_empty() => Some(c.trim().to_string()),
                _ => None,
            };
            Ok(Command::CreateMedia {
                media: NewMedia {
                    name,
                    url,
                    media_type,
                    description,
                    content,
                },
                import_file: None,
            })
        }
        Commands::Media(MediaCommand::Delete { id }) => {
            if !no_interactive {
                let confirmed = inquire::Confirm::new(&format!("Delete media #{id}?"))
                    .with_default(false)
                    .prompt()
                    .map_err(|e| e.to_string())?;
                if !confirmed {
                    return Err("aborted by user".into());
                }
            }
            Ok(Command::DeleteMedia { id: *id })
        }

        Commands::Server(_) => {
            Err("server commands are handled locally and do not dispatch to daemon".into())
        }
    }
}

/// What was passed for a field, said as one of the three things that can happen
/// to it: it was given, it was given as nothing, or it was left out.
#[derive(Debug, PartialEq, Eq)]
enum Given<'a> {
    /// A usable value.
    Yes(&'a str),
    /// Passed, but empty: an unquoted shell argument, or a flag with no value.
    Blank,
    /// Not passed at all.
    Missing,
}

impl<'a> Given<'a> {
    /// What this field was given, with a blank counting as nothing.
    ///
    /// A name of three spaces is the same as no name: the task would be called
    /// nothing, and a list of nothing is not a list.
    fn of(value: &'a Option<String>) -> Self {
        match value {
            Some(value) if !value.trim().is_empty() => Given::Yes(value.trim()),
            Some(_) => Given::Blank,
            None => Given::Missing,
        }
    }
}

/// What to do with a field the user might have to be asked about.
#[derive(Debug, PartialEq, Eq)]
enum Asked<'a> {
    /// It was given; this is it.
    Given(&'a str),
    /// Nobody can be asked, so it cannot be left out.
    Refuse(String),
    /// Ask the user.
    Ask,
}

/// A field a record cannot do without.
///
/// With nobody to ask, the answer is an error naming the flag: "required" on its
/// own would leave the user to guess which of the eleven flags they had left
/// out. `noun` is what the record is called in that sentence, so a workspace is
/// never told it is short of a task name.
fn required<'a>(
    value: &'a Option<String>,
    noun: &str,
    flag: &str,
    no_interactive: bool,
) -> Asked<'a> {
    match Given::of(value) {
        Given::Yes(value) => Asked::Given(value),
        Given::Blank if no_interactive => Asked::Refuse(format!("{flag} cannot be blank")),
        Given::Missing if no_interactive => {
            // The placeholder is the flag's own name in capitals, so the refusal
            // can be typed straight back: `pass --name <NAME>`.
            let what = flag.trim_start_matches("--");
            let placeholder = what.replace('-', "_").to_uppercase();
            Asked::Refuse(format!(
                "a {noun} needs a {what}: pass {flag} <{placeholder}>"
            ))
        }
        _ => Asked::Ask,
    }
}

/// Build a `NewTask`, prompting for any field the user did not pass on the CLI.
///
/// When `no_interactive` is set, nothing is prompted: optional fields fall back
/// to `None` (priority to `not-yet`) and missing required fields are an error
/// naming the flag that would have supplied them.
#[allow(clippy::too_many_arguments)]
async fn build_new_task(
    client: &mut Client,
    no_interactive: bool,
    name: &Option<String>,
    on: &Option<String>,
    workspace_id: &Option<i64>,
    media_list_id: &Option<String>,
    description: &Option<String>,
    priority: &Option<String>,
    estimated_mins: &Option<i64>,
) -> Result<NewTask, Box<dyn std::error::Error>> {
    let name = match required(name, "task", "--name", no_interactive) {
        Asked::Given(name) => name.to_string(),
        Asked::Refuse(message) => return Err(message.into()),
        Asked::Ask => resolve_name("Task name", "e.g. Write the quarterly report")?,
    };

    let scheduled_on = match on {
        // `none` is a day spelled out: it asks for the task to have no day,
        // which is different from not asking about the day at all.
        Some(value) => date_on(value)?,
        None if no_interactive => None,
        None => resolve_scheduled_on()?,
    };

    let workspace_id = match workspace_id {
        Some(id) => *id,
        None if no_interactive => {
            return Err("a task needs a workspace: pass --workspace-id <ID>".into());
        }
        None => resolve_workspace(client).await?,
    };

    let media_list_id = match media_list_id {
        Some(value) if value == "none" || value.trim().is_empty() => None,
        Some(value) => Some(
            value
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("invalid media list id: {value}"))?,
        ),
        None if no_interactive => None,
        None => resolve_media_list(client).await?,
    };

    let priority = match priority {
        Some(priority) => priority.parse()?,
        None if no_interactive => Priority::NotYet,
        None => resolve_priority()?,
    };

    // A description the user left as a space is not a description they meant.
    let description = match description {
        Some(description) if description.trim().is_empty() => None,
        Some(description) => Some(description.trim().to_string()),
        None if no_interactive => None,
        None => resolve_description()?,
    };

    let estimated_mins = match estimated_mins {
        Some(mins) => {
            if *mins <= 0 {
                return Err("estimated minutes must be greater than 0".into());
            }
            Some(*mins)
        }
        None if no_interactive => None,
        None => resolve_estimated_mins()?,
    };

    Ok(NewTask {
        workspace_id,
        media_list_id,
        name,
        description,
        priority,
        estimated_mins,
        scheduled_on,
    })
}

/// Build the name and description of something simpler than a task.
///
/// A workspace or a media list has two fields and no schedule, so it shares the
/// task's name rules -- blank is not a name, and with nobody to ask the refusal
/// names the flag -- and nothing else. Returns the pair rather than either
/// record so both callers read the same way.
fn build_named(
    noun: &str,
    label: &str,
    name: &Option<String>,
    description: &Option<String>,
    no_interactive: bool,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let name = match required(name, noun, "--name", no_interactive) {
        Asked::Given(name) => name.to_string(),
        Asked::Refuse(message) => return Err(message.into()),
        Asked::Ask => resolve_name(label, &format!("e.g. the name of a {noun}"))?,
    };

    // A description the user left as a space is not one they meant.
    let description = match description {
        Some(description) if description.trim().is_empty() => None,
        Some(description) => Some(description.trim().to_string()),
        None if no_interactive => None,
        None => resolve_description()?,
    };

    Ok((name, description))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_command_that_cannot_be_built_says_what_was_wrong() {
        // No name and no terminal to prompt on: the error has to name the flag
        // that would have supplied it, because this is the whole message the
        // user gets.
        let (mut client, _) = match connect(None, true).await {
            Ok(pair) => pair,
            // No daemon on this machine, which is the normal case in a test:
            // the prompts would have needed one, and nothing was reached.
            Err(_) => return,
        };

        let error = build_command(
            &Commands::Task(TaskCommand::Create {
                name: None,
                workspace_id: Some(1),
                media_list_id: None,
                description: None,
                priority: None,
                estimated_mins: None,
                on: None,
            }),
            &mut client,
            true,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("--name"), "{error}");
    }

    #[tokio::test]
    async fn a_workspace_without_a_name_is_refused_in_its_own_words() {
        // The workspace builder used to be the task builder, so a missing name
        // here would have been told to pass a task flag.
        let (mut client, _) = match connect(None, true).await {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let error = build_command(
            &Commands::Workspace(WorkspaceCommand::Create {
                name: None,
                description: None,
            }),
            &mut client,
            true,
        )
        .await
        .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("a workspace needs a name"), "{message}");
        assert!(!message.contains("task"), "{message}");
    }

    #[test]
    fn a_blank_name_is_a_name_that_was_not_given() {
        // Whitespace is what an unquoted shell argument turns into, and it is
        // not a name. Tested against `required` itself rather than a copy of
        // it, which is what this used to be.
        let blank = Some("   ".to_string());

        assert_eq!(
            required(&blank, "task", "--name", true),
            Asked::Refuse("--name cannot be blank".into())
        );
        assert_eq!(
            required(&Some(String::new()), "task", "--name", true),
            Asked::Refuse("--name cannot be blank".into())
        );
        assert_eq!(
            required(&None, "task", "--name", true),
            Asked::Refuse("a task needs a name: pass --name <NAME>".into())
        );
    }

    #[test]
    fn a_record_is_named_in_its_own_words_when_it_has_no_name() {
        assert_eq!(
            required(&None, "media list", "--name", true),
            Asked::Refuse("a media list needs a name: pass --name <NAME>".into())
        );
    }

    #[test]
    fn a_name_that_was_given_is_never_a_question() {
        let given = Some("  side project  ".to_string());

        assert_eq!(
            required(&given, "workspace", "--name", true),
            Asked::Given("side project")
        );
        // Nobody to ask is only a refusal for a field that is missing.
        assert_eq!(required(&None, "workspace", "--name", false), Asked::Ask);
    }

    #[test]
    fn a_refusal_is_told_apart_from_a_crash() {
        // Both are failures, but only one of them is the daemon's answer, and
        // the exit code says which happened.
        assert!(
            Failure::Refused("no such task".into())
                .to_string()
                .contains("no such task")
        );
        assert!(
            !Failure::Daemon("no socket".into())
                .to_string()
                .contains("no such task")
        );
    }

    /// A daemon to build against, or nothing when there is none on this
    /// machine -- which is the normal case in a test. Kept beside the tests
    /// that need one so a missing daemon skips them rather than failing them.
    async fn client() -> Option<Client> {
        connect(None, true).await.ok().map(|(c, _)| c)
    }

    #[tokio::test]
    async fn a_sort_order_that_was_not_understood_is_given_back_as_it_was_typed() {
        // The two words that would have worked are not printed, because the
        // user typed something and not something near it.
        let Some(mut client) = client().await else {
            return;
        };

        let error = build_command(
            &Commands::Task(TaskCommand::List {
                sort_priority: Some("sideways".to_string()),
                priority_label: None,
                status: None,
                today: false,
                date: None,
                workspace: None,
                all_workspaces: false,
            }),
            &mut client,
            true,
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "invalid sort order: sideways");
    }

    #[tokio::test]
    async fn a_status_that_was_not_understood_is_given_back_too() {
        let Some(mut client) = client().await else {
            return;
        };

        let error = build_command(
            &Commands::Task(TaskCommand::List {
                sort_priority: None,
                priority_label: None,
                status: Some("nearly".to_string()),
                today: false,
                date: None,
                workspace: None,
                all_workspaces: false,
            }),
            &mut client,
            true,
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "invalid status: nearly");
    }

    #[tokio::test]
    async fn a_list_asked_for_by_name_is_a_list() {
        // `Command` is compared by shape rather than by value because it
        // carries records that have no `PartialEq` of their own.
        let Some(mut client) = client().await else {
            return;
        };

        let commands = Cli::parse_from(["let-timer", "tasks"]).component.command();

        let command = build_command(&commands, &mut client, true)
            .await
            .expect("a list needs no more than that");

        assert!(
            matches!(
                command,
                Command::List(ListQuery {
                    sort_priority: None,
                    status: None,
                    ..
                })
            ),
            "{command:?}"
        );
    }
}
