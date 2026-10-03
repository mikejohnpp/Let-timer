//! Taking the terminal over, and giving it back.
//!
//! The hard part of this file is not drawing. It is the way back: a terminal
//! left in raw mode, or left switched to the alternate screen, is a shell the
//! user has to kill. So [`Screen`] puts the terminal back when it is dropped,
//! which covers the ordinary exit, an error on the way out, and a panic, since
//! dropping still happens while the panic unwinds.
//!
//! The alternate screen is a scratch buffer that vanishes on exit, which is
//! why it is right for a program that owns the terminal while it runs and
//! hands it back whole when it stops.

use std::io::{self, stdout};

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::DefaultTerminal;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// A terminal set up for the interface, and put back when it goes out of scope.
pub struct Screen {
    terminal: DefaultTerminal,
    restored: bool,
}

impl Screen {
    /// Take the terminal over.
    pub fn enter() -> Result<Self, TerminalError> {
        enable_raw_mode().map_err(|source| TerminalError::Enter {
            stage: "raw mode",
            source,
        })?;

        // Every failure from here on has to put back what has already been
        // done, or the user is left with a shell they cannot type into.
        let mut terminal = match Self::build() {
            Ok(terminal) => terminal,
            Err(error) => {
                Self::undo();
                return Err(error);
            }
        };

        if let Err(source) = terminal.hide_cursor() {
            Self::undo();
            return Err(TerminalError::Enter {
                stage: "the cursor",
                source,
            });
        }

        Ok(Self {
            terminal,
            restored: false,
        })
    }

    /// Make the terminal, having already turned raw mode on.
    fn build() -> Result<DefaultTerminal, TerminalError> {
        let mut backend = CrosstermBackend::new(stdout());
        execute!(backend, EnterAlternateScreen).map_err(|source| TerminalError::Enter {
            stage: "the alternate screen",
            source,
        })?;

        Terminal::new(backend).map_err(|source| TerminalError::Enter {
            stage: "the screen",
            source,
        })
    }

    /// Put back whatever has been done so far.
    ///
    /// Best effort: it runs on the way out of a failure, where there is nothing
    /// useful left to do with a second error.
    fn undo() {
        let mut backend = CrosstermBackend::new(stdout());
        let _ = execute!(backend, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }

    /// The terminal to draw into.
    pub fn terminal(&mut self) -> &mut DefaultTerminal {
        &mut self.terminal
    }

    /// Draw one frame.
    pub fn draw<F>(&mut self, render: F) -> Result<(), TerminalError>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal
            .draw(render)
            .map(|_| ())
            .map_err(|source| TerminalError::Draw { source })
    }

    /// Put the terminal back now instead of waiting to be dropped.
    ///
    /// Dropping does the same thing, so this only makes the order visible at
    /// the end of `main`, where it is easier to notice missing. Calling it
    /// twice does nothing the second time.
    pub fn close(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;

        // The cursor first, or it is left blinking somewhere off screen.
        let _ = self.terminal.show_cursor();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        // This is what makes a panic survivable: dropping still happens while
        // the panic unwinds, so the terminal comes back even when the
        // application does not.
        self.close();
    }
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Screen").finish()
    }
}

/// Something went wrong with the terminal itself.
#[derive(Debug)]
pub enum TerminalError {
    /// The terminal could not be set up.
    Enter {
        /// Which part of the setup gave up.
        stage: &'static str,
        source: io::Error,
    },
    /// A frame could not be drawn.
    Draw { source: io::Error },
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::Enter { stage, source } => {
                write!(f, "cannot set the terminal up: {stage}: {source}")
            }
            TerminalError::Draw { source } => write!(f, "cannot draw: {source}"),
        }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TerminalError::Enter { source, .. } | TerminalError::Draw { source } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── without a terminal ─────────────────────────────────────────────

    #[test]
    fn asking_for_a_terminal_where_there_is_none_fails_instead_of_hanging() {
        // There is no tty under the test runner. What matters is that this
        // returns rather than blocking or aborting, so that the caller can
        // report it and carry on.
        let result = Screen::enter();
        assert!(result.is_err(), "expected failure without a terminal");
    }

    // ── errors ─────────────────────────────────────────────────────────

    #[test]
    fn an_enter_error_names_the_part_that_failed() {
        let error = TerminalError::Enter {
            stage: "raw mode",
            source: io::Error::other("no tty"),
        };
        let message = error.to_string();

        assert!(message.contains("raw mode"), "{message}");
        assert!(message.contains("no tty"), "{message}");
    }

    #[test]
    fn a_draw_error_says_what_it_was() {
        let error = TerminalError::Draw {
            source: io::Error::other("device is gone"),
        };

        assert!(error.to_string().contains("cannot draw"), "{error}");
        assert!(error.to_string().contains("device is gone"), "{error}");
    }

    #[test]
    fn errors_keep_their_cause() {
        use std::error::Error;

        let error = TerminalError::Draw {
            source: io::Error::other("device is gone"),
        };
        assert!(error.source().is_some());
    }
}
