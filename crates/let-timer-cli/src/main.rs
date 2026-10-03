//! `let-timer`: the interface, and the questions that can be asked of it from a
//! shell.
//!
//! There are two kinds of run here. With no component named at all, the
//! terminal belongs to the interface for as long as the user is looking at it,
//! and the daemon is only ever spoken to through it. With one named, this is a
//! question with an answer that gets printed: the terminal is left exactly as
//! it was found, which is what makes these lines the ones a script can run.
//!
//! The component is read first in both, because it is what the question acts
//! on. `tasks find` and `workspaces list` reach different lists, and there is
//! no line that reaches either one without saying which it meant.

mod cli;
mod render;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use let_timer_core::{Command, IpcClient, Response, SortOrder, TaskStatus};
// The one thing the shell hands to the interface is which kind of record to
// look at; named apart from `cli::Commands`, which is what the shell asks
// instead.
use let_timer_tui::action::Component as TuiComponent;

use crate::cli::{Cli, Commands, MediaListCommand, TaskCommand, WorkspaceCommand};

/// Anything that stopped the program from doing what it was asked.
#[derive(Debug)]
enum Failure {
    /// The daemon could not be reached at all.
    Daemon(String),
    /// The daemon was reached and said no.
    Refused(String),
    /// What was asked for was not usable: a sort order, a status.
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
                "let-timer needs a terminal to draw in; run `let-timer tasks` for the same list as text"
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

/// Show the interface, or answer the question.
///
/// Nothing named is the only run that draws: it is the task list, which is
/// what this program is for. A component on its own is already a question, and
/// `command` turns a component with nothing under it into the same question as
/// its list subcommand, so there is no branch here for a run that would draw
/// less than the whole screen -- there is no such run.
async fn run(cli: &Cli) -> Result<(), Failure> {
    let Some(component) = &cli.component else {
        return look(TuiComponent::Task).await;
    };

    once(cli, &component.command()).await
}

/// Hand the terminal to the interface.
async fn look(component: TuiComponent) -> Result<(), Failure> {
    if !std::io::stdout().is_terminal() {
        return Err(Failure::NoTerminal);
    }

    let_timer_tui::run::run(component)
        .await
        .map_err(|error| Failure::Usage(error.to_string()))
}

/// Ask the daemon to do one thing, print the answer, and stop.
async fn once(cli: &Cli, command: &Commands) -> Result<(), Failure> {
    // Built before the daemon is looked for, so that a word that cannot be
    // understood is said in those words rather than behind a socket error.
    let command = build_command(command).map_err(|error| Failure::Usage(error.to_string()))?;

    let mut client = connect().await?;

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

/// Turn a parsed command into the protocol's.
///
/// Nothing is asked of the user and no list has to be read to answer: every
/// command left on the line already says all it can say. That leaves one thing
/// to refuse, which is a value this program was told how to understand and
/// does not understand -- the word is given back as it was typed rather than
/// replaced by the words that would have worked.
fn build_command(command: &Commands) -> Result<Command, Box<dyn std::error::Error>> {
    match command {
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

        // The other two components have nothing asked of them but their list,
        // so there is no word here that could be a bad one.
        Commands::Workspace(WorkspaceCommand::List) => Ok(Command::ListWorkspaces),
        Commands::MediaList(MediaListCommand::List) => Ok(Command::ListMediaLists),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sort_order_that_was_not_understood_is_given_back_as_it_was_typed() {
        // The two words that would have worked are not printed, because the
        // user typed something and not something near it.
        let error = build_command(&Commands::Task(TaskCommand::List {
            priority: Some("sideways".to_string()),
            status: None,
        }))
        .unwrap_err();

        assert_eq!(error.to_string(), "invalid sort order: sideways");
    }

    #[test]
    fn a_status_that_was_not_understood_is_given_back_too() {
        let error = build_command(&Commands::Task(TaskCommand::List {
            priority: None,
            status: Some("nearly".to_string()),
        }))
        .unwrap_err();

        assert_eq!(error.to_string(), "invalid status: nearly");
    }

    #[test]
    fn a_list_asked_for_by_name_is_a_list() {
        // The run that replaced the inline panel: same command, no terminal,
        // and the two of them are asked for in different ways. `Command` is
        // compared by shape rather than by value because it carries records
        // that have no `PartialEq` of their own.
        let commands = Cli::parse_from(["let-timer", "tasks"])
            .component
            .map(|component| component.command())
            .expect("a component was named");

        let command = build_command(&commands).expect("a list needs no more than that");

        assert!(
            matches!(
                command,
                Command::List {
                    sort_priority: None,
                    filter_status: None,
                }
            ),
            "{command:?}"
        );
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

        // The suggestion has to be a line this program still accepts that needs
        // no terminal of its own, which is now the bare component rather than
        // its list subcommand.
        assert!(message.contains("let-timer tasks`"), "{message}");
    }
}
