//! The command line, as written down.
//!
//! One rule runs through all of it: a component is the first thing named, and
//! everything after it acts on that component and nothing else. `tasks create`
//! makes a task, `workspaces create` makes a workspace, and there is no way to
//! write one of them without saying which.
//!
//! That is why a component is a subcommand rather than a flag or a positional.
//! A positional with a default value is always filled in, so it can never tell
//! `let-timer` from `let-timer tasks`, and a command beside it can quietly win.
//! As a subcommand the component has to be there: clap refuses the line before
//! any of this program's code runs, and says what was expected instead.
//!
//! Two kinds of run fall out of that. A component with nothing under it is the
//! interface, and the terminal belongs to the user for as long as they are
//! looking at it. A component with a command under it is one request, one
//! printed answer, and the process is done.

use clap::{Parser, Subcommand};

/// Show a list, or manage one item, and print the answer.
#[derive(Parser, Debug)]
#[command(
    name = "let-timer",
    about = "Tasks, workspaces and media lists",
    version
)]
pub struct Cli {
    /// Never prompt: use defaults for missing fields, error on missing required ones.
    #[arg(long, global = true)]
    pub no_interactive: bool,

    /// Print the answer as JSON, for a script to read.
    #[arg(long, global = true)]
    pub json: bool,

    /// What this run is about, and what any command under it acts on.
    ///
    /// Left out, the task list is what is shown: it is what this program is
    /// for, and it is the one run that takes the whole screen.
    #[command(subcommand)]
    pub component: Option<Component>,
}

/// The three kinds of thing there are, and the commands each of them has.
///
/// Each variant carries its own commands, which is the point: a workspace has no
/// `edit` because the daemon has no command to answer one with, so clap will
/// not offer it.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Component {
    /// Tasks, in a schedule.
    Tasks {
        #[command(subcommand)]
        command: Option<TaskCommand>,
    },
    /// The projects tasks are filed under.
    Workspaces {
        #[command(subcommand)]
        command: Option<WorkspaceCommand>,
    },
    /// The lists a task can point at for the media it needs.
    MediaLists {
        #[command(subcommand)]
        command: Option<MediaListCommand>,
    },
}

impl Component {
    /// The kind of record this run is about, whatever was asked of it.
    pub fn kind(&self) -> View {
        match self {
            Component::Tasks { .. } => View::Task,
            Component::Workspaces { .. } => View::Workspace,
            Component::MediaLists { .. } => View::MediaList,
        }
    }

    /// What to do about it, with the component's own nesting flattened away.
    ///
    /// `None` means the component was named and nothing else, which is the
    /// interface and not a command.
    pub fn command(&self) -> Option<Commands> {
        match self {
            Component::Tasks { command } => command.clone().map(Commands::Task),
            Component::Workspaces { command } => command.clone().map(Commands::Workspace),
            Component::MediaLists { command } => command.clone().map(Commands::MediaList),
        }
    }
}

/// One kind of record, with nothing asked of it yet.
///
/// Singular because it is one kind at a time: the word on the command line is
/// plural (`tasks`), the thing on screen is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Task,
    Workspace,
    MediaList,
}

impl From<View> for let_timer_tui::action::Component {
    fn from(view: View) -> Self {
        match view {
            View::Task => Self::Task,
            View::Workspace => Self::Workspace,
            View::MediaList => Self::MediaList,
        }
    }
}

/// What to do, and to what, with the decision already made.
///
/// Not a clap type: this is the answer to "which command, on which kind of
/// record", which is the only thing the rest of the program has to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    Task(TaskCommand),
    Workspace(WorkspaceCommand),
    MediaList(MediaListCommand),
}

/// What can be asked of a task.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum TaskCommand {
    /// Create a task.
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
        #[arg(short = 't', long, value_name = "MINUTES")]
        estimated_mins: Option<i64>,
        /// Day the task is planned for: `2026-10-01`, `today`, `tomorrow`,
        /// a weekday (`mon`..`sun`), or `none` to leave it unscheduled.
        #[arg(short = 'o', long, value_name = "DATE|none")]
        on: Option<String>,
    },
    /// Delete a task by id.
    Delete {
        #[arg(short, long)]
        id: i64,
    },
    /// Edit one task by id. Fields left out are not changed.
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
    /// Search tasks by name.
    Find { query: String },
    /// Show the schedule.
    List {
        #[arg(long)]
        priority: Option<String>, // "ascending" | "descending"
        #[arg(long)]
        status: Option<String>, // "pending" | "in-progress" | "completed" | "cancelled"
    },
    /// Show the work in hand right now.
    Current,
    /// Start a task, or the next one if there is no id.
    Start {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Stop the current task.
    Stop,
    /// Mark the current task done.
    Done,
}

/// What can be asked of a workspace.
///
/// `edit` and `delete` are missing on purpose: the protocol has no command to
/// answer them with, so offering them would only move the failure.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceCommand {
    /// Show every workspace.
    List,
    /// Create a workspace.
    Create {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
}

/// What can be asked of a media list.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum MediaListCommand {
    /// Show every media list.
    List,
    /// Create a media list.
    Create {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn nothing_named_shows_the_task_list() {
        // No component at all is the one run that is not a choice: the task
        // list is what this program is for.
        let cli = Cli::parse_from(["let-timer"]);

        assert_eq!(cli.component, None);
    }

    #[test]
    fn a_component_with_no_subcommand_is_a_screen() {
        // This is the whole point of nesting: saying what to look at is not
        // asking the daemon to do anything, and it is still the thing that was
        // named.
        let cli = Cli::parse_from(["let-timer", "workspaces"]);
        let component = cli.component.expect("a component was named");

        assert_eq!(
            component,
            Component::Workspaces { command: None },
            "a component with nothing under it is a screen"
        );
        assert_eq!(component.kind(), View::Workspace);
        assert_eq!(component.command(), None);
    }

    #[test]
    fn the_component_cannot_be_left_out() {
        // `let-timer list` used to mean the task list. Now the command cannot
        // be reached without saying what it acts on, so the line is refused
        // before this program's code runs at all.
        let error = Cli::try_parse_from(["let-timer", "list"]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn the_three_components_are_the_ones_on_offer() {
        // A line that names no component gets told where to look rather than
        // just that the word was wrong.
        let help = Cli::command().render_long_help().to_string();

        for component in ["tasks", "workspaces", "media-lists"] {
            assert!(
                help.contains(component),
                "{component} missing from:\n{help}"
            );
        }
    }

    #[test]
    fn a_creating_task_lives_under_tasks() {
        let cli = Cli::parse_from(["let-timer", "tasks", "create", "--name", "wash up"]);

        assert_eq!(
            cli.component,
            Some(Component::Tasks {
                command: Some(TaskCommand::Create {
                    name: Some("wash up".into()),
                    workspace_id: None,
                    media_list_id: None,
                    description: None,
                    priority: None,
                    estimated_mins: None,
                    on: None,
                })
            })
        );
    }

    #[test]
    fn a_workspace_create_is_not_a_task_create() {
        // The line this shape exists to stop: with the component beside the
        // command instead of above it, `workspaces create` reached the task
        // builder and made a task.
        let cli = Cli::parse_from(["let-timer", "workspaces", "create", "-n", "side"]);

        assert_eq!(
            cli.component.and_then(|component| component.command()),
            Some(Commands::Workspace(WorkspaceCommand::Create {
                name: Some("side".into()),
                description: None,
            }))
        );
    }

    #[test]
    fn a_media_list_create_is_not_a_task_create() {
        let cli = Cli::parse_from([
            "let-timer",
            "media-lists",
            "create",
            "-n",
            "books",
            "-d",
            "to read this year",
        ]);

        assert_eq!(
            cli.component.and_then(|component| component.command()),
            Some(Commands::MediaList(MediaListCommand::Create {
                name: Some("books".into()),
                description: Some("to read this year".into()),
            }))
        );
    }

    #[test]
    fn each_component_only_offers_what_the_daemon_has() {
        // Refused here rather than at the daemon, so the user is told what to
        // type instead of hearing that a workspace cannot be edited.
        let refused = [
            ["let-timer", "workspaces", "edit"],
            ["let-timer", "workspaces", "start"],
            ["let-timer", "media-lists", "delete"],
            ["let-timer", "media-lists", "list-extra"],
        ];

        for argv in refused {
            let error = Cli::try_parse_from(argv).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidSubcommand, "{argv:?}");
        }
    }

    #[test]
    fn json_is_global_through_the_nesting() {
        // `global = true` has to reach past a subcommand inside a subcommand,
        // because a script should not have to know where it put the flag.
        assert!(Cli::parse_from(["let-timer", "--json", "tasks", "current"]).json);
        assert!(Cli::parse_from(["let-timer", "tasks", "current", "--json"]).json);
        assert!(Cli::parse_from(["let-timer", "--json"]).json);
        assert!(!Cli::parse_from(["let-timer", "tasks", "current"]).json);
    }

    #[test]
    fn every_command_the_daemon_has_is_reachable() {
        // One row per `Command` in the protocol. A refactor that drops one, or
        // files it under the wrong component, fails here.
        let reachable: [(&[&str], Commands); 13] = [
            (
                &["tasks", "create", "-n", "wash up"],
                Commands::Task(TaskCommand::Create {
                    name: Some("wash up".into()),
                    workspace_id: None,
                    media_list_id: None,
                    description: None,
                    priority: None,
                    estimated_mins: None,
                    on: None,
                }),
            ),
            (
                &["tasks", "delete", "--id", "1"],
                Commands::Task(TaskCommand::Delete { id: 1 }),
            ),
            (
                &["tasks", "edit", "--id", "1"],
                Commands::Task(TaskCommand::Edit {
                    id: 1,
                    name: None,
                    description: None,
                    priority: None,
                    on: None,
                }),
            ),
            (
                &["tasks", "find", "needle"],
                Commands::Task(TaskCommand::Find {
                    query: "needle".into(),
                }),
            ),
            (
                &["tasks", "list"],
                Commands::Task(TaskCommand::List {
                    priority: None,
                    status: None,
                }),
            ),
            (&["tasks", "current"], Commands::Task(TaskCommand::Current)),
            (
                &["tasks", "start"],
                Commands::Task(TaskCommand::Start { id: None }),
            ),
            (&["tasks", "stop"], Commands::Task(TaskCommand::Stop)),
            (&["tasks", "done"], Commands::Task(TaskCommand::Done)),
            (
                &["workspaces", "list"],
                Commands::Workspace(WorkspaceCommand::List),
            ),
            (
                &["workspaces", "create", "-n", "side"],
                Commands::Workspace(WorkspaceCommand::Create {
                    name: Some("side".into()),
                    description: None,
                }),
            ),
            (
                &["media-lists", "list"],
                Commands::MediaList(MediaListCommand::List),
            ),
            (
                &["media-lists", "create", "-n", "books"],
                Commands::MediaList(MediaListCommand::Create {
                    name: Some("books".into()),
                    description: None,
                }),
            ),
        ];

        for (tail, expected) in reachable {
            let mut argv = vec!["let-timer"];
            argv.extend_from_slice(tail);
            let cli = Cli::parse_from(&argv);

            assert_eq!(
                cli.component
                    .as_ref()
                    .and_then(|component| component.command()),
                Some(expected),
                "{argv:?}"
            );
        }
    }

    #[test]
    fn the_command_line_is_well_formed() {
        // clap can be told to check itself, which catches the mistakes that
        // only show up as a panic in somebody else's argument parsing.
        Cli::command().debug_assert();
    }
}
