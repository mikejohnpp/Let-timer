mod interactive;

use clap::{Parser, Subcommand};
use let_timer_core::{
    Command, IpcClient, NewTask, Priority, Response, SortOrder, UpdateTask, date_on,
};

use crate::interactive::{
    resolve_description, resolve_estimated_mins, resolve_media_list, resolve_name,
    resolve_priority, resolve_scheduled_on, resolve_workspace,
};

#[derive(Parser)]
#[command(name = "let-timer", about = "CLI for let-timer daemon")]
struct Cli {
    /// Never prompt: use defaults for missing fields, error on missing required ones.
    #[arg(long, global = true)]
    no_interactive: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short = 'w', long)]
        workspace_id: Option<i64>,
        #[arg(short = 'm', long, value_name = "ID|none")]
        media_list_id: Option<String>, // "none" skips; numeric picks a list
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        priority: Option<String>, // "urgent" | "immediate" | "not-yet"
        #[arg(short = 't', long)]
        estimated_mins: Option<i64>,
        /// Day the task is planned for: `2026-10-01`, `today`, `tomorrow`,
        /// a weekday (`mon`..`sun`), or `none` to leave it unscheduled.
        #[arg(short = 'o', long, value_name = "DATE|none")]
        on: Option<String>,
    },
    Delete {
        #[arg(short, long)]
        id: i64,
    },
    Edit {
        #[arg(short, long)]
        id: i64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        priority: Option<String>,
        /// Reschedule: `2026-10-01`, `today`, `tomorrow`, `mon`..`sun`, or
        /// `none` to clear the scheduled day.
        #[arg(short = 'o', long, value_name = "DATE|none")]
        on: Option<String>,
    },
    Find {
        query: String,
    },
    List {
        #[arg(long)]
        priority: Option<String>, // "ascending" | "descending"
        #[arg(long)]
        status: Option<String>, // "pending" | "in-progress" | "completed" | "cancelled"
    },
    Current,
    Start {
        #[arg(short, long)]
        id: Option<i64>,
    },
    Stop,
    Done,
}

/// Build a `NewTask`, prompting for any field the user did not pass on the CLI.
///
/// Order: name -> scheduled day -> workspace -> media list -> priority ->
/// description -> estimated mins. When `no_interactive` is set, nothing is
/// prompted: optional fields fall back to `None` (priority to `not-yet`) and
/// missing required fields produce an error.
#[allow(clippy::too_many_arguments)]
async fn build_new_task(
    client: &mut IpcClient,
    no_interactive: bool,
    name: &Option<String>,
    on: &Option<String>,
    workspace_id: &Option<i64>,
    media_list_id: &Option<String>,
    description: &Option<String>,
    priority: &Option<String>,
    estimated_mins: &Option<i64>,
) -> Result<NewTask, Box<dyn std::error::Error>> {
    let name = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ if no_interactive => {
            return Err("task name is required; pass --name <NAME> or run interactively".into());
        }
        _ => resolve_name()?,
    };

    let scheduled_on = match on {
        Some(value) => date_on(value)?,
        None if no_interactive => None,
        None => resolve_scheduled_on()?,
    };

    let workspace_id = match workspace_id {
        Some(id) => *id,
        None if no_interactive => {
            return Err(
                "workspace is required; pass --workspace-id <ID> or run interactively".into(),
            );
        }
        None => resolve_workspace(client).await?,
    };

    let media_list_id = match media_list_id {
        Some(s) if s == "none" || s.trim().is_empty() => None,
        Some(s) => Some(
            s.trim()
                .parse::<i64>()
                .map_err(|_| format!("invalid media list id: {s}"))?,
        ),
        None if no_interactive => None,
        None => resolve_media_list(client).await?,
    };

    let priority = match priority {
        Some(p) => p.parse()?,
        None if no_interactive => Priority::NotYet,
        None => resolve_priority()?,
    };

    let description = match description {
        Some(d) if d.trim().is_empty() => None,
        Some(d) => Some(d.trim().to_string()),
        None if no_interactive => None,
        None => resolve_description()?,
    };

    let estimated_mins = match estimated_mins {
        Some(m) => Some(*m),
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

/// Turn a parsed CLI command into a protocol `Command`, prompting for any
/// missing selection (e.g. workspace/media list) interactively.
async fn parse_cli_command_to_protocol(
    command: &Commands,
    client: &mut IpcClient,
    no_interactive: bool,
) -> Result<Command, Box<dyn std::error::Error>> {
    match command {
        Commands::Create {
            name,
            on,
            workspace_id,
            media_list_id,
            description,
            priority,
            estimated_mins,
        } => {
            let new_task = build_new_task(
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
            .await?;
            Ok(Command::Create(new_task))
        }
        Commands::Delete { id } => Ok(Command::Delete { id: *id }),
        Commands::Edit {
            id,
            name,
            description,
            priority,
            on,
        } => {
            let update_task = UpdateTask {
                name: name.clone(),
                description: match description {
                    Some(desc) if desc.is_empty() => None,
                    _ => Some(description.clone()),
                },
                priority: match priority {
                    Some(p) => Some(p.parse()?),
                    None => None,
                },
                scheduled_on: match on {
                    Some(value) => Some(date_on(value)?),
                    None => None,
                },
                ..Default::default()
            };
            Ok(Command::Edit {
                id: *id,
                update: update_task,
            })
        }
        Commands::Find { query } => Ok(Command::Find {
            query: query.clone(),
        }),
        Commands::List { priority, status } => {
            let sort_priority = match priority {
                Some(p) => Some(match p.as_str() {
                    "ascending" => SortOrder::Ascending,
                    "descending" => SortOrder::Descending,
                    _ => return Err(format!("invalid sort order value: {p}").into()),
                }),
                None => None,
            };
            let filter_status = match status {
                Some(s) => Some(match s.as_str() {
                    "pending" => let_timer_core::TaskStatus::Pending,
                    "in-progress" => let_timer_core::TaskStatus::InProgress,
                    "completed" => let_timer_core::TaskStatus::Completed,
                    "cancelled" => let_timer_core::TaskStatus::Cancelled,
                    _ => return Err(format!("invalid status value: {s}").into()),
                }),
                None => None,
            };
            Ok(Command::List {
                sort_priority,
                filter_status,
            })
        }
        Commands::Current => Ok(Command::Current),
        Commands::Start { id } => Ok(Command::Start { id: *id }),
        Commands::Stop => Ok(Command::Stop),
        Commands::Done => Ok(Command::Done),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let path = std::path::Path::new("/tmp/let-timer.sock");
    let mut client = match IpcClient::connect(path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect daemon: {e}");
            std::process::exit(1);
        }
    };

    let cmd_to_protocol =
        match parse_cli_command_to_protocol(&cli.command, &mut client, cli.no_interactive).await {
            Ok(cmd) => cmd,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };

    match client.request(&cmd_to_protocol).await {
        Ok(resp) => match resp {
            Response::Ok(payload) => {
                println!("Success: {:?}", payload);
            }
            Response::OkList(tasks) => {
                println!("Tasks: {:?}", tasks);
            }
            Response::OkEmpty => {
                println!("Success: No data returned");
            }
            Response::WorkspaceList(workspaces) => {
                println!("Workspaces: {:?}", workspaces);
            }
            Response::MediaListList(media_lists) => {
                println!("Media lists: {:?}", media_lists);
            }
            Response::Workspace(workspace) => {
                println!("Workspace: {:?}", workspace);
            }
            Response::MediaList(media_list) => {
                println!("Media list: {:?}", media_list);
            }
            Response::Error { message } => {
                eprintln!("Error from daemon: {message}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
