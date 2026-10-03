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
//! `let-timer tasks` is a longer way of saying `let-timer tasks list`. Only a
//! run that is asked for no component at all hands over the terminal, because
//! the task list is what this program is for. Everything else answers one
//! question, prints it, and is done -- which is also why nothing here needs a
//! terminal, and nothing here asks the user a question.

use clap::{Parser, Subcommand};

/// Show a list, or report on one item.
#[derive(Parser, Debug)]
#[command(
    name = "let-timer",
    about = "Tasks, workspaces and media lists",
    version
)]
pub struct Cli {
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
                    priority: None,
                    status: None,
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
///
/// Nothing here creates, edits or deletes: those are the interface's forms, and
/// a line of flags is not a worse way of filling one in so much as a different
/// thing to maintain twice.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum TaskCommand {
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
/// `create`, `edit` and `delete` are all missing on purpose: the first belongs
/// to a form, and the other two have no protocol command to answer them with.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceCommand {
    /// Show every workspace.
    List,
}

/// What can be asked of a media list.
///
/// Missing `create`, `edit` and `delete` for the same reasons as a workspace's.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum MediaListCommand {
    /// Show every media list.
    List,
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
    fn a_component_with_no_subcommand_is_its_list() {
        // Naming what to look at is not a longer way of naming the screen. It
        // is the same answer as asking for the list by name, and it does not
        // take the terminal to get it.
        let cli = Cli::parse_from(["let-timer", "workspaces"]);
        let component = cli.component.expect("a component was named");

        assert_eq!(
            component,
            Component::Workspaces { command: None },
            "a component with nothing under it is its list"
        );
        assert_eq!(
            component.command(),
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
                    priority: None,
                    status: None,
                }),
            ),
            ("workspaces", Commands::Workspace(WorkspaceCommand::List)),
            ("media-lists", Commands::MediaList(MediaListCommand::List)),
        ];

        for (name, expected) in cases {
            let cli = Cli::parse_from(["let-timer", name]);

            assert_eq!(
                cli.component.as_ref().map(Component::command),
                Some(expected),
                "{name}"
            );
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
    fn each_component_only_offers_what_is_still_on_offer() {
        // Refused here rather than at the daemon, so the user is told what to
        // type instead of hearing that a workspace cannot be edited. The
        // removed three are in this list too: a line that used to work has to
        // stop with a message rather than quietly meaning something else.
        let refused = [
            ["let-timer", "tasks", "create"],
            ["let-timer", "tasks", "delete"],
            ["let-timer", "tasks", "edit"],
            ["let-timer", "workspaces", "create"],
            ["let-timer", "workspaces", "edit"],
            ["let-timer", "workspaces", "start"],
            ["let-timer", "media-lists", "create"],
            ["let-timer", "media-lists", "delete"],
            ["let-timer", "media-lists", "list-extra"],
        ];

        for argv in refused {
            let error = Cli::try_parse_from(argv).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidSubcommand, "{argv:?}");
        }
    }

    #[test]
    fn the_inline_flags_are_gone_from_the_line() {
        // Nothing here takes over the terminal, so nothing here needs a
        // terminal, and there is no field left that a prompt could fill in.
        let help = Cli::command().render_long_help().to_string();

        assert!(!help.contains("--no-interactive"), "{help}");

        let error =
            Cli::try_parse_from(["let-timer", "tasks", "list", "--no-interactive"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn json_is_global_through_the_nesting() {
        // `global = true` has to reach past a subcommand inside a subcommand,
        // because a script should not have to know where it put the flag.
        assert!(Cli::parse_from(["let-timer", "--json", "tasks", "current"]).json);
        assert!(Cli::parse_from(["let-timer", "tasks", "current", "--json"]).json);
        assert!(Cli::parse_from(["let-timer", "--json"]).json);
        assert!(!Cli::parse_from(["let-timer", "tasks", "current"]).json);
        // And it works on the run that needs no subcommand at all, which is
        // the one a script is most likely to reach for.
        assert!(Cli::parse_from(["let-timer", "--json", "tasks"]).json);
    }

    #[test]
    fn every_command_that_is_left_is_reachable() {
        // One row per subcommand in this file. A refactor that drops one, or
        // files it under the wrong component, fails here.
        let reachable: [(&[&str], Commands); 8] = [
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
                &["media-lists", "list"],
                Commands::MediaList(MediaListCommand::List),
            ),
        ];

        for (tail, expected) in reachable {
            let mut argv = vec!["let-timer"];
            argv.extend_from_slice(tail);
            let cli = Cli::parse_from(&argv);

            assert_eq!(
                cli.component.as_ref().map(Component::command),
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
