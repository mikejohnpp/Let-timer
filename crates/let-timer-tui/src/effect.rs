//! Things a store asks the outside world to do.
//!
//! Stores are pure state machines: they never touch the terminal, the IPC
//! client or the filesystem themselves. Instead they push an [`Effect`] and
//! the event loop carries it out. This keeps every store unit-testable.

use let_timer_core::Command;

/// An instruction for the event loop to carry out.
///
/// `Command` carries no `PartialEq`, so `Effect` does not either. Tests match
/// on the variant instead of comparing whole values.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Send a command to the daemon over IPC.
    Send(Command),
    /// Leave the TUI.
    Quit,
    /// Show a short-lived message to the user.
    Toast(String),
}
