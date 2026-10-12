//! The command line, as written down.
//!
//! One rule runs through all of it: a component is the first thing named, and
//! everything after it acts on that component and nothing else. `tasks find`
//! looks for a task, `workspaces list` lists workspaces, and there is no way to
//! write one of them without saying which it meant.
//!
//! That is why a component is a subcommand rather than a flag or a positional.
//! A positional with a default value is always filled in, so it can never tell
//! `let-timer` from `let-timer tasks`, and a command beside it can quietly win.
//! As a subcommand the component has to be there: clap refuses the line before
//! any of this program's code runs, and says what was expected instead.
//!
//! A component with nothing under it is that component's list, so
//! `let-timer tasks` is a longer way of saying `let-timer tasks list`. A
//! component has to be named: there is no bare run, so a line that names
//! nothing is refused by clap before any of this program's code runs.
//!
//! Every command answers one question, prints it, and is done -- which is also
//! why nothing here needs a terminal for itself. A command that leaves a field
//! out asks the user for it instead, unless `--no-interactive` was passed, in
//! which case the missing field is an error naming the flag.

use clap::{Parser, Subcommand};

/// Show a list, or report on one item.
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
    /// Required: a line that names nothing is refused before any of this
    /// program's code runs, and clap says what was expected instead.
    #[command(subcommand)]
    pub component: Component,
}

/// The three kinds of thing there are, and what can be asked of each.
///
/// Each variant carries its own commands, which is the point: nothing is
/// offered here that the daemon has no command to answer, and nothing is
/// offered that the interface already does better. A workspace cannot be edited
/// because there is no protocol command that would answer one, and no component
/// can be created from here because that is a form with a field list in it,
/// not something to be typed on one line.
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
    /// Media items (image, music, poem).
    Medias {
        #[command(subcommand)]
        command: Option<MediaCommand>,
    },
    /// Daemon management commands.
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
}

impl Component {
    /// What to do about it, with the component's own nesting flattened away.
    ///
    /// Nothing under the component is not a third kind of run: it is the list,
    /// which is the one thing asked of every component and the only thing left
    /// to ask of them. Deciding that here means `run` has one answer rather
    /// than two, and no way to reach a screen that has been taken away.
    pub fn command(&self) -> Commands {
        match self {
            Component::Tasks { command } => match command {
                Some(command) => Commands::Task(command.clone()),
                None => Commands::Task(TaskCommand::List {
                    sort_priority: None,
                    priority_label: None,
                    status: None,
                    today: false,
                    date: None,
                    workspace: None,
                    all_workspaces: false,
                }),
            },
            Component::Workspaces { command } => match command {
                Some(command) => Commands::Workspace(command.clone()),
                None => Commands::Workspace(WorkspaceCommand::List),
            },
            Component::MediaLists { command } => match command {
                Some(command) => Commands::MediaList(command.clone()),
                None => Commands::MediaList(MediaListCommand::List),
            },
            Component::Medias { command } => match command {
                Some(command) => Commands::Media(command.clone()),
                None => Commands::Media(MediaCommand::List { media_type: None }),
            },
            Component::Daemon { command } => match command {
                Some(command) => Commands::Daemon(command.clone()),
                None => Commands::Daemon(DaemonCommand::Status),
            },
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
    Media(MediaCommand),
    Daemon(DaemonCommand),
}

/// What can be asked of a task.
///
/// A command that leaves a field out asks the user for it, unless
/// `--no-interactive` was passed: prompting is what fills one line of flags
/// into a whole record without making the user type every flag.
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
        #[arg(short = 't', long, value_name = "MINUTES")]
        estimated_mins: Option<i64>,
        #[arg(long, default_value_t = false)]
        clear_estimate: bool,
        #[arg(short = 'm', long, value_name = "ID|none")]
        media_list_id: Option<String>,
        /// Reschedule: `2026-10-01`, `today`, `tomorrow`, `mon`..`sun`, or
        /// `none` to clear the scheduled day.
        #[arg(short = 'o', long, value_name = "DATE|none")]
        on: Option<String>,
    },
    /// Search tasks by name.
    Find {
        query: String,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
        #[arg(long, default_value_t = false)]
        all_workspaces: bool,
    },
    /// Show the schedule.
    List {
        /// Sort tasks by priority: `ascending` | `descending`.
        #[arg(long, alias = "priority")]
        sort_priority: Option<String>,
        /// Filter by priority label: `urgent` | `immediate` | `not-yet`.
        #[arg(long)]
        priority_label: Option<String>,
        /// Filter by status: `pending` | `in-progress` | `paused` | `completed` | `cancelled`.
        #[arg(long)]
        status: Option<String>,
        /// Filter by today's date.
        #[arg(long, default_value_t = false)]
        today: bool,
        /// Filter by scheduled date: `2026-10-01`, `today`, `tomorrow`, `mon`..`sun`.
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
        #[arg(long, default_value_t = false)]
        all_workspaces: bool,
    },
    /// Show the work in hand right now.
    Current {
        #[arg(long, default_value = "text", value_name = "text|short|json")]
        format: String,
    },
    /// Start a task, or the next one if there is no id.
    Start {
        #[arg(short, long)]
        id: Option<i64>,
        #[arg(long, default_value_t = false)]
        next: bool,
        #[arg(long, default_value_t = false)]
        switch: bool,
    },
    /// Pause a task (defaults to currently running task).
    Pause {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Resume a task (defaults to most recently paused task).
    Resume {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Stop the current task.
    Stop {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Restart a task (defaults to currently running task).
    Restart {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Mark the current task done.
    Done {
        #[arg(short, long)]
        id: Option<i64>,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Cancel a task (defaults to currently running task).
    Cancel {
        #[arg(short, long)]
        id: Option<i64>,
    },
    /// Reopen a completed or cancelled task.
    Reopen {
        #[arg(short, long)]
        id: i64,
    },
}

/// What can be asked of the daemon.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum DaemonCommand {
    /// Show daemon status (ping).
    Status,
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
    /// Switch active workspace by id or name.
    Switch {
        /// Target workspace id or name (omit for interactive picker).
        target: Option<String>,
    },
    /// Show current active workspace.
    Current,
    /// Delete a workspace by id.
    Delete {
        #[arg(short, long)]
        id: i64,
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
    /// Show a media list and its contents.
    Show {
        #[arg(short, long)]
        id: i64,
    },
    /// Delete a media list by id.
    Delete {
        #[arg(short, long)]
        id: i64,
    },
    /// Add a media item to a list.
    AddMedia {
        #[arg(short = 'l', long)]
        list_id: i64,
        #[arg(short = 'm', long)]
        media_id: i64,
    },
    /// Remove a media item from a list.
    RemoveMedia {
        #[arg(short = 'l', long)]
        list_id: i64,
        #[arg(short = 'm', long)]
        media_id: i64,
    },
}

/// What can be asked of media items.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum MediaCommand {
    /// Show every media item, optionally filtered by type.
    List {
        #[arg(short = 't', long, value_name = "image|music|poem")]
        media_type: Option<String>,
    },
    /// Create a media item.
    Create {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        url: Option<String>,
        #[arg(short = 't', long, value_name = "image|music|poem")]
        media_type: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a media item by id.
    Delete {
        #[arg(short, long)]
        id: i64,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn nothing_named_is_refused_with_help() {
        // There is no bare run: a line that names nothing is refused before
        // any of this program's code runs, and clap says what to type instead.
        let error = Cli::try_parse_from(["let-timer"]).unwrap_err();

        assert_eq!(
            error.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn a_component_with_no_subcommand_is_its_list() {
        // Naming what to look at is a choice about what to print. It is the
        // same answer as asking for the list by name.
        let cli = Cli::parse_from(["let-timer", "workspaces"]);

        assert_eq!(
            cli.component,
            Component::Workspaces { command: None },
            "a component with nothing under it is its list"
        );
        assert_eq!(
            cli.component.command(),
            Commands::Workspace(WorkspaceCommand::List)
        );
    }

    #[test]
    fn every_component_answers_with_its_own_list() {
        // The same decision three times: without this, naming a component and
        // naming its list would be two different questions.
        let cases = [
            (
                "tasks",
                Commands::Task(TaskCommand::List {
                    sort_priority: None,
                    priority_label: None,
                    status: None,
                    today: false,
                    date: None,
                    workspace: None,
                    all_workspaces: false,
                }),
            ),
            ("workspaces", Commands::Workspace(WorkspaceCommand::List)),
            ("media-lists", Commands::MediaList(MediaListCommand::List)),
            (
                "medias",
                Commands::Media(MediaCommand::List { media_type: None }),
            ),
        ];

        for (name, expected) in cases {
            let cli = Cli::parse_from(["let-timer", name]);

            assert_eq!(cli.component.command(), expected, "{name}");
        }
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
    fn the_four_components_are_the_ones_on_offer() {
        // A line that names no component gets told where to look rather than
        // just that the word was wrong.
        let help = Cli::command().render_long_help().to_string();

        for component in ["tasks", "workspaces", "media-lists", "medias"] {
            assert!(
                help.contains(component),
                "{component} missing from:\n{help}"
            );
        }
    }

    #[test]
    fn each_component_only_offers_what_the_daemon_has() {
        // Refused here rather than at the daemon, so the user is told what to
        // type instead of hearing that a workspace cannot be edited.
        let refused = [
            ["let-timer", "workspaces", "edit"],
            ["let-timer", "workspaces", "start"],
            ["let-timer", "media-lists", "edit"],
            ["let-timer", "medias", "edit"],
        ];

        for argv in refused {
            let error = Cli::try_parse_from(argv).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidSubcommand, "{argv:?}");
        }
    }

    #[test]
    fn a_creating_task_lives_under_tasks() {
        let cli = Cli::parse_from(["let-timer", "tasks", "create", "--name", "wash up"]);

        assert_eq!(
            cli.component,
            Component::Tasks {
                command: Some(TaskCommand::Create {
                    name: Some("wash up".into()),
                    workspace_id: None,
                    media_list_id: None,
                    description: None,
                    priority: None,
                    estimated_mins: None,
                    on: None,
                })
            }
        );
    }

    #[test]
    fn a_workspace_create_is_not_a_task_create() {
        // The line this shape exists to stop: with the component beside the
        // command instead of above it, `workspaces create` reached the task
        // builder and made a task.
        let cli = Cli::parse_from(["let-timer", "workspaces", "create", "-n", "side"]);

        assert_eq!(
            cli.component.command(),
            Commands::Workspace(WorkspaceCommand::Create {
                name: Some("side".into()),
                description: None,
            })
        );
    }

    #[test]
    fn no_interactive_is_global_through_the_nesting() {
        // A script should not have to know where to put the flag, and an
        // interactive terminal should not have to avoid it.
        assert!(
            Cli::parse_from(["let-timer", "--no-interactive", "tasks", "create"]).no_interactive
        );
        assert!(
            Cli::parse_from(["let-timer", "tasks", "create", "--no-interactive"]).no_interactive
        );
        assert!(!Cli::parse_from(["let-timer", "tasks", "create"]).no_interactive);
    }

    #[test]
    fn json_is_global_through_the_nesting() {
        // `global = true` has to reach past a subcommand inside a subcommand,
        // because a script should not have to know where it put the flag.
        assert!(Cli::parse_from(["let-timer", "--json", "tasks", "current"]).json);
        assert!(Cli::parse_from(["let-timer", "tasks", "current", "--json"]).json);
        assert!(!Cli::parse_from(["let-timer", "tasks", "current"]).json);
        // And it works on the run that needs no subcommand at all, which is
        // the one a script is most likely to reach for.
        assert!(Cli::parse_from(["let-timer", "--json", "tasks"]).json);
    }

    #[test]
    fn every_command_the_daemon_has_is_reachable() {
        // One row per `Command` in the protocol. A refactor that drops one, or
        // files it under the wrong component, fails here.
        let reachable: [(&[&str], Commands); 29] = [
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
                    estimated_mins: None,
                    clear_estimate: false,
                    media_list_id: None,
                    on: None,
                }),
            ),
            (
                &["tasks", "find", "needle"],
                Commands::Task(TaskCommand::Find {
                    query: "needle".into(),
                    workspace: None,
                    all_workspaces: false,
                }),
            ),
            (
                &["tasks", "list"],
                Commands::Task(TaskCommand::List {
                    sort_priority: None,
                    priority_label: None,
                    status: None,
                    today: false,
                    date: None,
                    workspace: None,
                    all_workspaces: false,
                }),
            ),
            (
                &["tasks", "current"],
                Commands::Task(TaskCommand::Current {
                    format: "text".into(),
                }),
            ),
            (
                &["tasks", "start"],
                Commands::Task(TaskCommand::Start {
                    id: None,
                    next: false,
                    switch: false,
                }),
            ),
            (
                &["tasks", "pause"],
                Commands::Task(TaskCommand::Pause { id: None }),
            ),
            (
                &["tasks", "resume"],
                Commands::Task(TaskCommand::Resume { id: None }),
            ),
            (
                &["tasks", "stop"],
                Commands::Task(TaskCommand::Stop { id: None }),
            ),
            (
                &["tasks", "restart"],
                Commands::Task(TaskCommand::Restart { id: None }),
            ),
            (
                &["tasks", "done"],
                Commands::Task(TaskCommand::Done {
                    id: None,
                    force: false,
                }),
            ),
            (
                &["tasks", "cancel"],
                Commands::Task(TaskCommand::Cancel { id: None }),
            ),
            (
                &["tasks", "reopen", "--id", "1"],
                Commands::Task(TaskCommand::Reopen { id: 1 }),
            ),
            (
                &["daemon", "status"],
                Commands::Daemon(DaemonCommand::Status),
            ),
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
                &["workspaces", "switch", "side"],
                Commands::Workspace(WorkspaceCommand::Switch {
                    target: Some("side".into()),
                }),
            ),
            (
                &["workspaces", "current"],
                Commands::Workspace(WorkspaceCommand::Current),
            ),
            (
                &["workspaces", "delete", "--id", "1"],
                Commands::Workspace(WorkspaceCommand::Delete { id: 1 }),
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
            (
                &["media-lists", "show", "--id", "1"],
                Commands::MediaList(MediaListCommand::Show { id: 1 }),
            ),
            (
                &["media-lists", "delete", "--id", "1"],
                Commands::MediaList(MediaListCommand::Delete { id: 1 }),
            ),
            (
                &["media-lists", "add-media", "-l", "1", "-m", "2"],
                Commands::MediaList(MediaListCommand::AddMedia {
                    list_id: 1,
                    media_id: 2,
                }),
            ),
            (
                &["media-lists", "remove-media", "-l", "1", "-m", "2"],
                Commands::MediaList(MediaListCommand::RemoveMedia {
                    list_id: 1,
                    media_id: 2,
                }),
            ),
            (
                &["medias", "list"],
                Commands::Media(MediaCommand::List { media_type: None }),
            ),
            (
                &[
                    "medias",
                    "create",
                    "-n",
                    "song",
                    "-u",
                    "http://example.com/a.mp3",
                    "-t",
                    "music",
                ],
                Commands::Media(MediaCommand::Create {
                    name: Some("song".into()),
                    url: Some("http://example.com/a.mp3".into()),
                    media_type: Some("music".into()),
                    description: None,
                }),
            ),
            (
                &["medias", "delete", "--id", "1"],
                Commands::Media(MediaCommand::Delete { id: 1 }),
            ),
        ];

        for (tail, expected) in reachable {
            let mut argv = vec!["let-timer"];
            argv.extend_from_slice(tail);
            let cli = Cli::parse_from(&argv);

            assert_eq!(cli.component.command(), expected, "{argv:?}");
        }
    }

    #[test]
    fn the_command_line_is_well_formed() {
        // clap can be told to check itself, which catches the mistakes that
        // only show up as a panic in somebody else's argument parsing.
        Cli::command().debug_assert();
    }
}
