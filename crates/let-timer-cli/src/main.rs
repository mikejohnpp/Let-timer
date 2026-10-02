mod interactive;

use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use clap::{Parser, Subcommand};
use let_timer_core::{Command, IpcClient, NewTask, Priority, Response, SortOrder, UpdateTask};

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

fn parse_priority(value: &str) -> Result<Priority, Box<dyn std::error::Error>> {
    match value {
        "urgent" => Ok(Priority::Urgent),
        "immediate" => Ok(Priority::Immediate),
        "not-yet" => Ok(Priority::NotYet),
        other => Err(format!("invalid priority value: {other}").into()),
    }
}

/// Resolve the weekday shortcut used by `--on`: `mon`..`sun`.
fn weekday_from_alias(alias: &str) -> Option<Weekday> {
    match alias {
        "mon" | "monday" | "thu-2" => Some(Weekday::Mon),
        "tue" | "tuesday" | "thu-3" => Some(Weekday::Tue),
        "wed" | "wednesday" | "thu-4" => Some(Weekday::Wed),
        "thu" | "thursday" | "thu-5" => Some(Weekday::Thu),
        "fri" | "friday" | "thu-6" => Some(Weekday::Fri),
        "sat" | "saturday" | "thu-7" => Some(Weekday::Sat),
        "sun" | "sunday" | "chu-nhat" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parse a `--on` value into a day.
///
/// Accepted: `none`/empty (unscheduled), an ISO date (`2026-10-01`),
/// `today`, `tomorrow`, or a weekday (`mon`..`sun`) resolving to the next
/// occurrence, today included.
fn parse_date_on(value: &str) -> Result<Option<NaiveDate>, Box<dyn std::error::Error>> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }

    let today = Local::now().date_naive();

    if value.eq_ignore_ascii_case("today") {
        return Ok(Some(today));
    }
    if value.eq_ignore_ascii_case("tomorrow") {
        return Ok(Some(today + Days::new(1)));
    }

    if let Some(weekday) = weekday_from_alias(&value.to_ascii_lowercase()) {
        let days_ahead =
            (weekday.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
        return Ok(Some(today + Days::new(days_ahead as u64)));
    }

    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            format!(
                "invalid date: {value} (expected YYYY-MM-DD, today, tomorrow, mon..sun, or none)"
            )
            .into()
        })
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
        Some(value) => parse_date_on(value)?,
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
        Some(p) => parse_priority(p)?,
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
                    Some(p) => Some(parse_priority(p)?),
                    None => None,
                },
                scheduled_on: match on {
                    Some(value) => Some(parse_date_on(value)?),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_on_accepts_none_and_empty() {
        assert_eq!(parse_date_on("none").unwrap(), None);
        assert_eq!(parse_date_on("NONE").unwrap(), None);
        assert_eq!(parse_date_on("  ").unwrap(), None);
    }

    #[test]
    fn parse_on_accepts_iso_date() {
        let expected = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        assert_eq!(parse_date_on("2026-10-01").unwrap(), Some(expected));
    }

    #[test]
    fn parse_on_accepts_relative_days() {
        let today = Local::now().date_naive();
        assert_eq!(parse_date_on("today").unwrap(), Some(today));
        assert_eq!(
            parse_date_on("tomorrow").unwrap(),
            Some(today + Days::new(1))
        );
    }

    #[test]
    fn parse_on_resolves_weekday_to_next_occurrence() {
        let today = Local::now().date_naive();
        let same_day = today.weekday();
        let alias = match same_day {
            Weekday::Mon => "mon",
            Weekday::Tue => "tue",
            Weekday::Wed => "wed",
            Weekday::Thu => "thu",
            Weekday::Fri => "fri",
            Weekday::Sat => "sat",
            Weekday::Sun => "sun",
        };

        // Today's weekday resolves to today, never to next week.
        assert_eq!(parse_date_on(alias).unwrap(), Some(today));

        // The day after tomorrow resolves to tomorrow.
        let next_alias = match same_day {
            Weekday::Mon => "tue",
            Weekday::Tue => "wed",
            Weekday::Wed => "thu",
            Weekday::Thu => "fri",
            Weekday::Fri => "sat",
            Weekday::Sat => "sun",
            Weekday::Sun => "mon",
        };
        assert_eq!(
            parse_date_on(next_alias).unwrap(),
            Some(today + Days::new(1))
        );
    }

    #[test]
    fn parse_on_rejects_garbage() {
        assert!(parse_date_on("01/10/2026").is_err());
        assert!(parse_date_on("2026-13-01").is_err());
        assert!(parse_date_on("funday").is_err());
    }

    #[test]
    fn parse_priority_rejects_unknown_value() {
        assert!(parse_priority("high").is_err());
        assert_eq!(parse_priority("urgent").unwrap(), Priority::Urgent);
    }
}
