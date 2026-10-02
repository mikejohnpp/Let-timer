//! State about the chrome around the data: which screen is showing, which
//! overlay is open, what the user was last told, and whether the daemon is
//! reachable.

use crate::action::Action;
use crate::effect::Effect;
use crate::store::{EffectQueue, Store};

/// How many ticks a toast stays on screen before it disappears.
///
/// Counting ticks instead of wall-clock seconds keeps the behaviour testable
/// and independent of how fast the terminal is.
const TOAST_TICKS: u8 = 4;

/// Which screen the TUI is running as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Takes over the whole terminal. Used when no subcommand was given.
    Fullscreen,
    /// A panel drawn inline in the terminal flow, sized to its content.
    /// `max_height` comes from the config file.
    Inline { max_height: u16 },
}

/// An overlay drawn on top of the current screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Popup {
    /// Nothing on top.
    #[default]
    None,
    /// The date picker, opened on the focused form field.
    Calendar,
    /// The key bindings list.
    Help,
}

/// State of the connection to the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Connection {
    /// Spawned, but no reply has come back yet.
    #[default]
    Connecting,
    /// The last command came back.
    Connected,
    /// The last command failed or the connection dropped.
    Disconnected,
}

/// A message shown to the user for a few ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Toast {
    message: String,
    ticks_left: u8,
}

/// UI chrome state.
#[derive(Debug)]
pub struct UiStore {
    mode: Mode,
    popup: Popup,
    toast: Option<Toast>,
    connection: Connection,
    effects: EffectQueue,
}

impl Default for UiStore {
    fn default() -> Self {
        Self::new(Mode::Fullscreen)
    }
}

impl UiStore {
    /// A UI store showing `mode`, with nothing overlaid.
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            popup: Popup::None,
            toast: None,
            connection: Connection::Connecting,
            effects: EffectQueue::default(),
        }
    }

    /// Which screen the TUI is running as.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The overlay currently open.
    pub fn popup(&self) -> Popup {
        self.popup
    }

    /// The message on screen, if any.
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_ref().map(|toast| toast.message.as_str())
    }

    /// Whether the daemon answered the last command.
    pub fn connection(&self) -> Connection {
        self.connection
    }

    /// Whether an overlay is open. Views use this to decide whether to
    /// swallow plain keys.
    pub fn has_popup(&self) -> bool {
        self.popup != Popup::None
    }
}

impl Store for UiStore {
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.effects.push(Effect::Quit),

            Action::Help => {
                self.popup = if self.popup == Popup::Help {
                    Popup::None
                } else {
                    Popup::Help
                };
            }

            // Cancel unwinds one layer at a time. In inline mode there is
            // nothing left to unwind once no overlay is open, so the panel
            // closes; in fullscreen there is no panel to close, so nothing
            // happens.
            Action::Cancel => match self.popup {
                Popup::None if matches!(self.mode, Mode::Inline { .. }) => {
                    self.effects.push(Effect::Quit);
                }
                Popup::None => {}
                other => self.popup = other.close(),
            },

            Action::DismissToast => self.toast = None,

            Action::Tick => {
                if let Some(toast) = &mut self.toast {
                    toast.ticks_left -= 1;
                    if toast.ticks_left == 0 {
                        self.toast = None;
                    }
                }
            }

            // Any reply at all proves the daemon is reachable.
            Action::TaskListLoaded(_)
            | Action::WorkspaceListLoaded(_)
            | Action::MediaListLoaded(_)
            | Action::TaskSaved(_) => self.connection = Connection::Connected,

            // An empty message means the daemon answered with nothing to
            // report, which is not an error and not worth a toast.
            Action::IpcFailed(ref message) => {
                if message.is_empty() {
                    return;
                }
                self.connection = Connection::Disconnected;
                self.show_toast(message);
            }

            _ => {}
        }
    }

    fn take_effects(&mut self) -> Vec<Effect> {
        self.effects.drain()
    }
}

impl UiStore {
    fn show_toast(&mut self, message: &str) {
        self.toast = Some(Toast {
            message: message.to_string(),
            ticks_left: TOAST_TICKS,
        });
    }
}

impl Popup {
    /// The popup underneath this one, for unwinding on cancel.
    fn close(self) -> Popup {
        Popup::None
    }
}

#[cfg(test)]
mod tests {
    use crate::store::test_util::task_with_id;

    use super::*;

    fn send(store: &mut UiStore, action: Action) -> Vec<Effect> {
        store.update(action);
        store.take_effects()
    }

    #[test]
    fn starts_in_fullscreen_with_nothing_overlaid() {
        let store = UiStore::default();
        assert_eq!(store.mode(), Mode::Fullscreen);
        assert_eq!(store.popup(), Popup::None);
        assert_eq!(store.toast(), None);
        assert_eq!(store.connection(), Connection::Connecting);
    }

    #[test]
    fn quit_produces_the_quit_effect() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::Quit);
        assert!(matches!(effects.as_slice(), [Effect::Quit]));
    }

    #[test]
    fn a_reply_marks_the_connection_connected() {
        let mut store = UiStore::default();
        send(&mut store, Action::TaskListLoaded(vec![task_with_id(1)]));
        assert_eq!(store.connection(), Connection::Connected);
        assert_eq!(store.toast(), None);
    }

    #[test]
    fn a_single_task_reply_also_marks_connected() {
        let mut store = UiStore::default();
        send(&mut store, Action::TaskSaved(task_with_id(1)));
        assert_eq!(store.connection(), Connection::Connected);
    }

    #[test]
    fn a_failure_shows_a_toast_and_marks_disconnected() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::IpcFailed("boom".to_string()));
        assert!(effects.is_empty());
        assert_eq!(store.toast(), Some("boom"));
        assert_eq!(store.connection(), Connection::Disconnected);
    }

    #[test]
    fn an_empty_failure_message_is_not_an_error() {
        let mut store = UiStore::default();
        send(&mut store, Action::IpcFailed(String::new()));
        assert_eq!(store.toast(), None);
        assert_eq!(store.connection(), Connection::Connecting);
    }

    #[test]
    fn a_toast_disappears_after_a_few_ticks() {
        let mut store = UiStore::default();
        send(&mut store, Action::IpcFailed("boom".to_string()));
        assert_eq!(store.toast(), Some("boom"));

        for _ in 0..TOAST_TICKS - 1 {
            send(&mut store, Action::Tick);
            assert_eq!(store.toast(), Some("boom"), "toast expired too early");
        }

        send(&mut store, Action::Tick);
        assert_eq!(store.toast(), None);
    }

    #[test]
    fn a_toast_can_be_dismissed_early() {
        let mut store = UiStore::default();
        send(&mut store, Action::IpcFailed("boom".to_string()));
        send(&mut store, Action::DismissToast);
        assert_eq!(store.toast(), None);
    }

    #[test]
    fn help_toggles_open_and_closed() {
        let mut store = UiStore::default();

        send(&mut store, Action::Help);
        assert_eq!(store.popup(), Popup::Help);
        assert!(store.has_popup());

        send(&mut store, Action::Help);
        assert_eq!(store.popup(), Popup::None);
    }

    #[test]
    fn cancel_closes_the_overlay_without_quitting() {
        let mut store = UiStore::default();
        send(&mut store, Action::Help);
        let effects = send(&mut store, Action::Cancel);
        assert_eq!(store.popup(), Popup::None);
        assert!(
            effects.is_empty(),
            "cancel should not quit with an overlay open"
        );
    }

    #[test]
    fn cancel_with_nothing_open_closes_an_inline_panel() {
        let mut store = UiStore::new(Mode::Inline { max_height: 20 });
        let effects = send(&mut store, Action::Cancel);
        assert!(matches!(effects.as_slice(), [Effect::Quit]));
    }

    #[test]
    fn cancel_with_nothing_open_does_nothing_in_fullscreen() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::Cancel);
        assert!(effects.is_empty());
        assert_eq!(store.connection(), Connection::Connecting);
    }

    #[test]
    fn actions_for_other_stores_are_ignored() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::Select(3));
        assert!(effects.is_empty());
        assert_eq!(store.popup(), Popup::None);
    }
}
