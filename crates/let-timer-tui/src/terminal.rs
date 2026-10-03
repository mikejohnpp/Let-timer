//! Taking the terminal over, and giving it back.
//!
//! The hard part of this file is not drawing. It is the way back: a terminal
//! left in raw mode, or left switched to the alternate screen, is a shell the
//! user has to kill. So [`Screen`] puts the terminal back when it is dropped,
//! which covers the ordinary exit, an error on the way out, and a panic, since
//! dropping still happens while the panic unwinds.
//!
//! Inline mode deliberately does not use the alternate screen. The alternate
//! screen is a scratch buffer that vanishes on exit, and an inline panel that
//! vanished would take the user's scrollback with it.

use std::io::{self, stdout};

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::DefaultTerminal;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::{TerminalOptions, Viewport};

use crate::store::Mode;

/// Whether this mode takes over the whole terminal.
///
/// Raw mode and a hidden cursor are wanted either way: both modes read single
/// key presses without the terminal echoing them or waiting for a newline.
/// The alternate screen is not, because only fullscreen is allowed to throw
/// away what was on screen before.
pub fn takes_the_screen(mode: Mode) -> bool {
    matches!(mode, Mode::Fullscreen)
}

/// A terminal set up for one mode, and put back when it goes out of scope.
pub struct Screen {
    terminal: DefaultTerminal,
    alternate_screen: bool,
    restored: bool,
}

impl Screen {
    /// Take the terminal over for `mode`.
    pub fn enter(mode: Mode) -> Result<Self, TerminalError> {
        let alternate_screen = takes_the_screen(mode);

        enable_raw_mode().map_err(|source| TerminalError::Enter {
            mode,
            stage: "raw mode",
            source,
        })?;

        // Every failure from here on has to put back what has already been
        // done, or the user is left with a shell they cannot type into.
        let built = Self::build(mode, alternate_screen);
        let mut terminal = match built {
            Ok(terminal) => terminal,
            Err(error) => {
                Self::undo(alternate_screen);
                return Err(error);
            }
        };

        if let Err(source) = terminal.hide_cursor() {
            Self::undo(alternate_screen);
            return Err(TerminalError::Enter {
                mode,
                stage: "the cursor",
                source,
            });
        }

        Ok(Self {
            terminal,
            alternate_screen,
            restored: false,
        })
    }

    /// Make the terminal, having already turned raw mode on.
    fn build(mode: Mode, alternate_screen: bool) -> Result<DefaultTerminal, TerminalError> {
        if alternate_screen {
            let mut backend = CrosstermBackend::new(stdout());
            execute!(backend, EnterAlternateScreen).map_err(|source| TerminalError::Enter {
                mode,
                stage: "the alternate screen",
                source,
            })?;

            return Terminal::new(backend).map_err(|source| TerminalError::Enter {
                mode,
                stage: "the screen",
                source,
            });
        }

        // An inline viewport of the panel's own height, so that ratatui keeps a
        // buffer the size of the panel rather than one the size of the terminal,
        // and so that the panel sits in the flow of the terminal rather than
        // over the top of it.
        let max_height = match mode {
            Mode::Inline { max_height } => max_height,
            Mode::Fullscreen => unreachable!("fullscreen is handled above"),
        };
        let options = TerminalOptions {
            viewport: Viewport::Inline(max_height),
        };
        Terminal::with_options(CrosstermBackend::new(stdout()), options).map_err(|source| {
            TerminalError::Enter {
                mode,
                stage: "the screen",
                source,
            }
        })
    }

    /// Put back whatever has been done so far.
    ///
    /// Best effort: it runs on the way out of a failure, where there is nothing
    /// useful left to do with a second error.
    fn undo(alternate_screen: bool) {
        if alternate_screen {
            let mut backend = CrosstermBackend::new(stdout());
            let _ = execute!(backend, LeaveAlternateScreen);
        }
        let _ = disable_raw_mode();
    }

    /// The terminal to draw into.
    pub fn terminal(&mut self) -> &mut DefaultTerminal {
        &mut self.terminal
    }

    /// Whether this screen took over the whole terminal.
    pub fn is_fullscreen(&self) -> bool {
        self.alternate_screen
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
        if self.alternate_screen {
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        }
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
        f.debug_struct("Screen")
            .field("alternate_screen", &self.alternate_screen)
            .finish()
    }
}

/// Something went wrong with the terminal itself.
#[derive(Debug)]
pub enum TerminalError {
    /// The terminal could not be set up.
    Enter {
        mode: Mode,
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
            TerminalError::Enter {
                mode,
                stage,
                source,
            } => write!(
                f,
                "cannot set the terminal up for {}: {stage}: {source}",
                match mode {
                    Mode::Fullscreen => "the fullscreen interface",
                    Mode::Inline { .. } => "the inline panel",
                }
            ),
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

    // ── which screen a mode gets ───────────────────────────────────────

    #[test]
    fn fullscreen_takes_the_whole_terminal() {
        assert!(takes_the_screen(Mode::Fullscreen));
    }

    #[test]
    fn inline_leaves_the_screen_alone() {
        // The alternate screen discards what was on screen when the program
        // exits. An inline panel that did that would take the user's scrollback
        // with it.
        assert!(!takes_the_screen(Mode::Inline { max_height: 10 }));
        assert!(!takes_the_screen(Mode::Inline { max_height: 40 }));
    }

    // ── without a terminal ─────────────────────────────────────────────

    #[test]
    fn asking_for_a_terminal_where_there_is_none_fails_instead_of_hanging() {
        // There is no tty under the test runner. What matters is that this
        // returns rather than blocking or aborting, so that the caller can
        // report it and carry on.
        let result = Screen::enter(Mode::Fullscreen);
        assert!(result.is_err(), "expected failure without a terminal");
    }

    // ── errors ─────────────────────────────────────────────────────────

    #[test]
    fn an_enter_error_names_the_mode_and_the_part_that_failed() {
        let error = TerminalError::Enter {
            mode: Mode::Fullscreen,
            stage: "raw mode",
            source: io::Error::other("no tty"),
        };
        let message = error.to_string();

        assert!(message.contains("fullscreen"), "{message}");
        assert!(message.contains("raw mode"), "{message}");
        assert!(message.contains("no tty"), "{message}");
    }

    #[test]
    fn an_enter_error_says_inline_for_an_inline_panel() {
        let error = TerminalError::Enter {
            mode: Mode::Inline { max_height: 8 },
            stage: "the cursor",
            source: io::Error::other("no tty"),
        };
        assert!(error.to_string().contains("inline panel"), "{error}");
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
