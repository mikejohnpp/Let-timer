//! `let-timer`: the interface, and the one-shot commands that go with it.
//!
//! There are two kinds of run here, and they are told apart by whether a command
//! was given. With one, the daemon is asked to do a thing and the answer is
//! printed and the program exits. Without one, the terminal belongs to the
//! interface for as long as the user is looking at it, and the daemon is only
//! ever spoken to through it.
//!
//! That is also why a view is a positional argument rather than a flag: saying
//! what to look at is not asking the daemon to do anything.

mod cli;
mod interactive;
mod render;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use let_timer_core::{
    Command, IpcClient, NewTask, Priority, Response, SortOrder, TaskStatus, UpdateTask, date_on,
};
use let_timer_tui::run::Launch;

use crate::cli::{Cli, Commands};
use crate::interactive::{
    resolve_description, resolve_estimated_mins, resolve_media_list, resolve_name,
    resolve_priority, resolve_scheduled_on, resolve_workspace,
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
    /// There is no terminal to draw on.
    NoTerminal,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Daemon(message) => write!(f, "cannot reach the daemon: {message}"),
            Failure::Refused(message) => write!(f, "{message}"),
            Failure::Usage(message) => write!(f, "{message}"),
            // Said the way a person would: what to run instead is in the
            // message, because the alternative is a shell that appears frozen.
            Failure::NoTerminal => write!(
                f,
                "let-timer needs a terminal to draw in; run `let-timer list` for the same list as text"
            ),
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
/// The one branch in the program: a command is something to do once, and no
/// command is something to look at until the user stops looking.
async fn run(cli: &Cli) -> Result<(), Failure> {
    match &cli.command {
        Some(command) => once(cli, command).await,
        None => look(cli).await,
    }
}

/// Hand the terminal to the interface.
///
/// Which view is a choice the user made by typing it; whether it takes the
/// whole screen is a choice they made by naming one, and asking for
/// `let-timer tasks` should not be the way to say `let-timer`. That is the one
/// rule here worth stating out loud, because both readings are reasonable and
/// only one of them can be right.
async fn look(cli: &Cli) -> Result<(), Failure> {
    if !std::io::stdout().is_terminal() {
        return Err(Failure::NoTerminal);
    }

    let component = cli.view.into();
    let launch = match cli.view {
        // Only one view is not a choice: the task list is what this program is
        // for, so it is what an unqualified `let-timer` shows, full screen.
        cli::View::Tasks => Launch::fullscreen(component),
        cli::View::Workspaces | cli::View::MediaLists => Launch::inline(component),
    };

    let_timer_tui::run::run(launch)
        .await
        .map_err(|error| Failure::Usage(error.to_string()))
}

/// Ask the daemon to do one thing, print the answer, and stop.
async fn once(cli: &Cli, command: &Commands) -> Result<(), Failure> {
    let mut client = connect().await?;

    let command = build_command(command, &mut client, cli.no_interactive)
        .await
        .map_err(|error| Failure::Usage(error.to_string()))?;

    let response = client
        .request(&command)
        .await
        .map_err(|error| Failure::Daemon(error.to_string()))?;

    // The daemon's own complaint is the answer, and the exit code has to say so:
    // a script that cannot tell a failure from an empty list is a script that
    // will act on a failure.
    if let Response::Error { message } = &response {
        return Err(Failure::Refused(message.clone()));
    }

    render::print(&response, cli.json).map_err(|error| Failure::Usage(error.to_string()))
}

/// The daemon, or an explanation of why there isn't one.
async fn connect() -> Result<IpcClient, Failure> {
    let path = let_timer_core::ipc::socket_path();
    IpcClient::connect(&path)
        .await
        .map_err(|error| Failure::Daemon(error.to_string()))
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
        } => Ok(Command::Create(
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

        Commands::Delete { id } => Ok(Command::Delete { id: *id }),

        Commands::Edit {
            id,
            name,
            description,
            priority,
            on,
        } => Ok(Command::Edit {
            id: *id,
            update: UpdateTask {
                name: name.clone(),
                // An empty string means "clear it", which is a different thing
                // from not having been asked to change it at all. Only the first
                // is worth sending.
                description: match description {
                    Some(description) if description.is_empty() => Some(None),
                    Some(description) => Some(Some(description.clone())),
                    None => None,
                },
                priority: match priority {
                    Some(priority) => Some(priority.parse()?),
                    None => None,
                },
                scheduled_on: match on {
                    Some(value) => Some(date_on(value)?),
                    None => None,
                },
                ..Default::default()
            },
        }),

        Commands::Find { query } => Ok(Command::Find {
            query: query.clone(),
        }),

        Commands::List { priority, status } => Ok(Command::List {
            sort_priority: match priority.as_deref() {
                Some("ascending") => Some(SortOrder::Ascending),
                Some("descending") => Some(SortOrder::Descending),
                Some(other) => return Err(format!("invalid sort order: {other}").into()),
                None => None,
            },
            filter_status: match status.as_deref() {
                Some("pending") => Some(TaskStatus::Pending),
                Some("in-progress") => Some(TaskStatus::InProgress),
                Some("completed") => Some(TaskStatus::Completed),
                Some("cancelled") => Some(TaskStatus::Cancelled),
                Some(other) => return Err(format!("invalid status: {other}").into()),
                None => None,
            },
        }),

        Commands::Current => Ok(Command::Current),
        Commands::Start { id } => Ok(Command::Start { id: *id }),
        Commands::Stop => Ok(Command::Stop),
        Commands::Done => Ok(Command::Done),
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

/// A field a task cannot do without.
///
/// With nobody to ask, the answer is an error naming the flag: "required" on its
/// own would leave the user to guess which of the eleven flags they had left
/// out.
fn required<'a>(value: &'a Option<String>, flag: &str, no_interactive: bool) -> Asked<'a> {
    match Given::of(value) {
        Given::Yes(value) => Asked::Given(value),
        Given::Blank if no_interactive => Asked::Refuse(format!("{flag} cannot be blank")),
        Given::Missing if no_interactive => Asked::Refuse(format!(
            "a task needs a {what}: pass {flag} <VALUE>",
            what = flag.trim_start_matches("--")
        )),
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
    let name = match required(name, "--name", no_interactive) {
        Asked::Given(name) => name.to_string(),
        Asked::Refuse(message) => return Err(message.into()),
        Asked::Ask => resolve_name()?,
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
        Some(mins) => Some(*mins),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_command_that_cannot_be_built_says_what_was_wrong() {
        // No name and no terminal to prompt on: the error has to name the flag
        // that would have supplied it, because this is the whole message the
        // user gets.
        let mut client = match connect().await {
            Ok(client) => client,
            // No daemon on this machine, which is the normal case in a test:
            // the prompts would have needed one, and nothing was reached.
            Err(_) => return,
        };

        let error = build_command(
            &Commands::Create {
                name: None,
                workspace_id: Some(1),
                media_list_id: None,
                description: None,
                priority: None,
                estimated_mins: None,
                on: None,
            },
            &mut client,
            true,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("--name"), "{error}");
    }

    #[test]
    fn a_blank_name_is_a_name_that_was_not_given() {
        // Whitespace is what an unquoted shell argument turns into, and it is
        // not a task name.
        let error = blank("   ", true);
        assert!(error.contains("--name"), "{error}");

        let error = blank("", true);
        assert!(error.contains("--name"), "{error}");
    }

    /// The name half of `build_new_task`, without a client or a terminal.
    fn blank(name: &str, no_interactive: bool) -> String {
        let name = Some(name.to_string());
        match name {
            Some(name) if !name.trim().is_empty() => "named".to_string(),
            Some(_) if no_interactive => "--name cannot be blank".to_string(),
            _ if no_interactive => "a task needs a name: pass --name <NAME>".to_string(),
            _ => "prompted".to_string(),
        }
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

    #[test]
    fn no_terminal_says_what_to_run_instead() {
        let message = Failure::NoTerminal.to_string();

        assert!(message.contains("let-timer list"), "{message}");
    }
}
