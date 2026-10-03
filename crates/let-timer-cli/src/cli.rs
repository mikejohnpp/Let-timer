//! The command line, as written down.
//!
//! One rule runs through all of it: a command that does something and exits is
//! one shape, and showing the interface is another. Only the first one talks to
//! the daemon once. Everything else is the interface, which is started instead
//! of a request being sent -- so the terminal belongs to the user for as long as
//! they are looking at it.
//!
//! That is also why the view is a positional argument and not a flag: it is what
//! to look at, and it is the only thing said when there is no command to run.

use clap::{Parser, Subcommand, ValueEnum};

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

    /// What to show when there is no command to run.
    #[arg(value_name = "VIEW", default_value = "tasks", value_enum)]
    pub view: View,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// The three kinds of thing there are.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Tasks,
    Workspaces,
    MediaLists,
}

impl From<View> for let_timer_tui::action::Component {
    fn from(view: View) -> Self {
        match view {
            View::Tasks => Self::Task,
            View::Workspaces => Self::Workspace,
            View::MediaLists => Self::MediaList,
        }
    }
}

/// One thing to do and print the answer for.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn nothing_to_run_shows_the_task_list() {
        let cli = Cli::parse_from(["let-timer"]);

        assert_eq!(cli.command, None);
        assert_eq!(cli.view, View::Tasks);
    }

    #[test]
    fn a_view_with_no_command_still_has_no_command() {
        // This is the whole point of the positional: saying what to look at is
        // not asking the daemon to do anything.
        let cli = Cli::parse_from(["let-timer", "workspaces"]);

        assert_eq!(cli.command, None);
        assert_eq!(cli.view, View::Workspaces);
        assert_eq!(
            let_timer_tui::action::Component::from(cli.view),
            let_timer_tui::action::Component::Workspace
        );
    }

    #[test]
    fn a_command_wins_over_a_view_that_looks_like_one() {
        // `list` is not a view, so this is the command. Were it a view too, the
        // command would have to lose and the user could not run it at all.
        let cli = Cli::parse_from(["let-timer", "list"]);

        assert!(matches!(cli.command, Some(Commands::List { .. })));
        assert_eq!(cli.view, View::Tasks);
    }

    #[test]
    fn json_can_be_asked_for_with_or_without_a_command() {
        assert!(Cli::parse_from(["let-timer", "--json", "current"]).json);
        assert!(Cli::parse_from(["let-timer", "--json"]).json);
        assert!(!Cli::parse_from(["let-timer", "current"]).json);
    }

    #[test]
    fn a_creating_task_is_a_command_and_not_a_view() {
        let cli = Cli::parse_from(["let-timer", "create", "--name", "wash up"]);

        assert_eq!(
            cli.command,
            Some(Commands::Create {
                name: Some("wash up".into()),
                workspace_id: None,
                media_list_id: None,
                description: None,
                priority: None,
                estimated_mins: None,
                on: None,
            })
        );
    }

    #[test]
    fn an_unknown_view_is_refused_with_the_three_that_exist() {
        let error = Cli::try_parse_from(["let-timer", "sprints"]).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("tasks"), "{message}");
        assert!(message.contains("workspaces"), "{message}");
        assert!(message.contains("media-lists"), "{message}");
    }

    #[test]
    fn the_command_line_is_well_formed() {
        // clap can be told to check itself, which catches the mistakes that
        // only show up as a panic in somebody else's argument parsing.
        Cli::command().debug_assert();
    }
}
