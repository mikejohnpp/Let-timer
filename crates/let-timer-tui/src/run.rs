//! Starting the interface: the one place that knows how the parts go together.
//!
//! Everything else in this crate is handed its parts. [`App`] is given a
//! component and a keymap; the terminal is set up by [`Screen::enter`]; events
//! come from [`Events::new`]. This module is the piece that reads the config
//! files, picks those parts, and hands them over, so that the binary calling in
//! from `main` has one job and does not have to know the order.
//!
//! There is one thing to start here, not a choice between several: the
//! interface takes the whole terminal. A panel drawn in the flow of somebody's
//! shell used to be a second way in, and everything it needed -- a second
//! viewport, a second keymap context with the letters left free, a height from
//! a settings file -- existed only to serve it. What the shell runs now is a
//! question with a printed answer, which is this crate's job elsewhere.

use crate::action::Component;
use crate::app::{App, AppError};
use crate::config::{ConfigError, keymap_file, settings};
use crate::event::Events;
use crate::terminal::Screen;
use crate::ui;

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
pub async fn run(component: Component) -> Result<(), RunError> {
    let keymap = keymap_file::load()?;
    let settings = settings::load()?;

    let mut screen = Screen::enter()?;
    let mut events = Events::new(settings.poll_interval());
    let app = App::new(component, keymap);

    let result = app.run(&mut screen, &mut events, ui::draw).await;
    screen.close();

    result.map_err(RunError::App)
}

#[cfg(test)]
mod tests {
    use super::*;

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
