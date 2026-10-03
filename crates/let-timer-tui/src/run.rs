//! Starting the interface: the one place that knows how the parts go together.
//!
//! Everything else in this crate is handed its parts. [`App`] is given a mode, a
//! component and a keymap; the terminal is set up by [`Screen::enter`]; events
//! come from [`Events::new`]. This module is the piece that reads the config
//! files, picks those parts, and hands them over, so that the binary calling in
//! from `main` has one job and does not have to know the order.

use crate::action::Component;
use crate::app::{App, AppError};
use crate::config::{ConfigError, keymap_file, settings};
use crate::event::Events;
use crate::store::Mode;
use crate::terminal::Screen;
use crate::ui;

/// What to show, and how much of the terminal to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Launch {
    component: Component,
    inline: bool,
}

impl Launch {
    /// Take over the terminal and show `component`.
    ///
    /// This is what running `let-timer` with nothing else to do should mean.
    pub fn fullscreen(component: Component) -> Self {
        Self {
            component,
            inline: false,
        }
    }

    /// Draw `component` in the flow of the terminal, where the command was
    /// typed and the shell is still there underneath it.
    ///
    /// How many rows that is comes from the settings file rather than from
    /// here: whoever types `let-timer tasks` has asked for a panel, not for a
    /// number of rows.
    pub fn inline(component: Component) -> Self {
        Self {
            component,
            inline: true,
        }
    }

    /// The component this launch shows.
    pub fn component(&self) -> Component {
        self.component
    }

    /// Whether this launch takes the whole terminal.
    pub fn is_fullscreen(&self) -> bool {
        !self.inline
    }
}

/// The mode a launch asks for, given the height the settings allow.
///
/// Only the height is asked about: fullscreen has no height to choose, and an
/// inline panel with no height cannot be drawn at all.
fn mode_for(launch: Launch, inline_max_height: u16) -> Mode {
    match launch.inline {
        false => Mode::Fullscreen,
        true => Mode::Inline {
            max_height: inline_max_height,
        },
    }
}

/// Why the interface did not start, or stopped early.
#[derive(Debug)]
pub enum RunError {
    /// A config file said something that could not be used.
    Config(ConfigError),
    /// The terminal could not be drawn on, or the input stopped.
    App(AppError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Config(error) => write!(f, "{error}"),
            RunError::App(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunError::Config(error) => Some(error),
            RunError::App(error) => Some(error),
        }
    }
}

impl From<ConfigError> for RunError {
    fn from(error: ConfigError) -> Self {
        RunError::Config(error)
    }
}

impl From<AppError> for RunError {
    fn from(error: AppError) -> Self {
        RunError::App(error)
    }
}

impl From<crate::terminal::TerminalError> for RunError {
    fn from(error: crate::terminal::TerminalError) -> Self {
        RunError::App(AppError::Terminal(error))
    }
}

/// Run the interface until the user leaves.
///
/// The config files are read first, before the terminal is touched: a keymap
/// that cannot be parsed is worth saying on a terminal that is still a terminal,
/// where the message can be read, and is not worth a screen that flashes up and
/// disappears again.
pub async fn run(launch: Launch) -> Result<(), RunError> {
    let keymap = keymap_file::load()?;
    let settings = settings::load()?;

    // `let-timer tasks` means "inline, please", and how tall that is belongs to
    // the settings file rather than to whoever typed the command.
    let mode = mode_for(launch, settings.inline_max_height());

    let mut screen = Screen::enter(mode)?;
    let mut events = Events::new(settings.poll_interval());
    let app = App::new(mode, launch.component, keymap);

    let result = app.run(&mut screen, &mut events, ui::draw).await;
    screen.close();

    result.map_err(RunError::App)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fullscreen_launch_is_fullscreen() {
        let launch = Launch::fullscreen(Component::Task);

        assert!(launch.is_fullscreen());
        assert_eq!(launch.component(), Component::Task);
    }

    #[test]
    fn an_inline_launch_is_not_fullscreen() {
        let launch = Launch::inline(Component::Workspace);

        assert!(!launch.is_fullscreen());
        assert_eq!(launch.component(), Component::Workspace);
    }

    #[test]
    fn the_settings_decide_how_tall_an_inline_panel_is() {
        // The height is not asked about in the launch, so it cannot be got
        // wrong by whoever is calling: there is only one place it comes from.
        assert_eq!(
            mode_for(Launch::inline(Component::Task), 9),
            Mode::Inline { max_height: 9 }
        );
    }

    #[test]
    fn the_settings_cannot_make_a_fullscreen_launch_inline() {
        assert_eq!(
            mode_for(Launch::fullscreen(Component::Task), 9),
            Mode::Fullscreen
        );
    }

    #[test]
    fn a_config_file_error_says_which_file_it_was() {
        // Written by hand rather than produced, because the point is what the
        // user is shown, not that the file is unreadable.
        let error = RunError::Config(ConfigError::Parse {
            path: "/home/somebody/.config/let-timer/keymap.toml".into(),
            message: "expected `[normal]`".into(),
        });

        assert!(
            error.to_string().contains("keymap.toml"),
            "found {:?}",
            error.to_string()
        );
    }

    #[test]
    fn a_terminal_failure_is_reported_as_an_app_failure() {
        let error = RunError::from(AppError::Terminal(crate::terminal::TerminalError::Enter {
            mode: Mode::Fullscreen,
            stage: "raw mode",
            source: std::io::Error::other("no terminal here"),
        }));

        assert!(matches!(error, RunError::App(AppError::Terminal(_))));
        assert!(
            error.to_string().contains("no terminal here"),
            "found {:?}",
            error.to_string()
        );
    }
}
