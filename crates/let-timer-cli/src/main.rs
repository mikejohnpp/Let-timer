//! `let-timer`: the interface, and the one-shot commands that go with it.
//!
//! There are two kinds of run here, and they are told apart by what was named
//! under the component. With a command, the daemon is asked to do a thing and
//! the answer is printed and the program exits. Without one, the terminal
//! belongs to the interface for as long as the user is looking at it, and the
//! daemon is only ever spoken to through it.
//!
//! The component is read first in both, because it is what the command acts on:
//! `tasks create` and `workspaces create` reach different builders, and there
//! is no line that reaches either one without saying which it meant.

mod cli;
mod interactive;
mod render;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use let_timer_core::{
    Command, IpcClient, NewMediaList, NewTask, NewWorkspace, Priority, Response, SortOrder,
    TaskStatus, UpdateTask, date_on,
};
use let_timer_tui::run::Launch;

use crate::cli::{Cli, Commands, MediaListCommand, TaskCommand, View, WorkspaceCommand};
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
                "let-timer needs a terminal to draw in; run `let-timer tasks list` for the same list as text"
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
/// The component is read first and decides everything: nothing reaches the
/// daemon without one having been named, because naming it is what decides
/// which commands exist.
async fn run(cli: &Cli) -> Result<(), Failure> {
    let Some(component) = &cli.component else {
        // Nothing named at all: the task list is what this program is for, and
        // it is the one run that takes the whole screen.
        return look(Launch::fullscreen(View::Task.into())).await;
    };

    let view = component.kind();
    match component.command() {
        // Named, with nothing asked of it: show that component, inline. Asking
        // for `let-timer tasks` is a choice about what to look at rather than a
        // longer way of saying `let-timer`, so it does not take the screen.
        None => {
            println!("Jump to look\nwith component {:?}", component);
            look(Launch::inline(view.into())).await
        }
        Some(commands) => {
            println!("Jump to once\nwith component {:?}", component);
            once(cli, &commands).await
        }
    }
}

/// Hand the terminal to the interface.
///
/// The launch already says which component and whether it is inline, because
/// that was decided by whether a component was named at all -- repeating it
/// here would be a second place to get it wrong.
async fn look(launch: Launch) -> Result<(), Failure> {
    if !std::io::stdout().is_terminal() {
        return Err(Failure::NoTerminal);
    }

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
            on,
        }) => Ok(Command::Edit {
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

        Commands::Task(TaskCommand::Find { query }) => Ok(Command::Find {
            query: query.clone(),
        }),

        Commands::Task(TaskCommand::List { priority, status }) => Ok(Command::List {
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

        Commands::Task(TaskCommand::Current) => Ok(Command::Current),
        Commands::Task(TaskCommand::Start { id }) => Ok(Command::Start { id: *id }),
        Commands::Task(TaskCommand::Stop) => Ok(Command::Stop),
        Commands::Task(TaskCommand::Done) => Ok(Command::Done),

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
        let mut client = match connect().await {
            Ok(client) => client,
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
        let mut client = match connect().await {
            Ok(client) => client,
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

    #[test]
    fn no_terminal_says_what_to_run_instead() {
        let message = Failure::NoTerminal.to_string();

        // The suggestion has to be a line this program still accepts, which is
        // why it names the component now.
        assert!(message.contains("let-timer tasks list"), "{message}");
    }
}
