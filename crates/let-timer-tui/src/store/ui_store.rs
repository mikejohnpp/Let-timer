//! State about the chrome around the data: which overlay is open, what the
//! user was last told, and whether the daemon is reachable.

use chrono::{Local, NaiveDate, TimeDelta};

use crate::action::Action;
use crate::effect::Effect;
use crate::store::{EffectQueue, Store};

/// How many ticks a toast stays on screen before it disappears.
///
/// Counting ticks instead of wall-clock seconds keeps the behaviour testable
/// and independent of how fast the terminal is.
const TOAST_TICKS: u8 = 4;

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
    /// Asking whether a task should really be deleted.
    Confirm,
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

/// Which part of the screen the next key press is aimed at.
///
/// The application draws more than one pane, so a key has to mean something
/// different depending on which pane is being looked at. Focus is the answer,
/// and it is held here rather than worked out from what is on screen so that
/// "where am I" is a fact with one owner instead of a question every view asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The list of records and the panel beside it.
    #[default]
    List,
    /// The list of workspaces down the left.
    Sidebar,
}

impl Focus {
    /// The other pane, for the key that moves between them.
    ///
    /// `tab` walks forwards through the panes and `shift-tab` backwards, so
    /// with two panes both keys land on the same place and neither can get
    /// stuck: there is nowhere further to go.
    pub fn other(self) -> Focus {
        match self {
            Focus::List => Focus::Sidebar,
            Focus::Sidebar => Focus::List,
        }
    }
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
    popup: Popup,
    toast: Option<Toast>,
    connection: Connection,
    /// The day the calendar has highlighted, while the calendar is open.
    calendar: Option<NaiveDate>,
    /// Which pane the next key press is aimed at.
    focus: Focus,
    effects: EffectQueue,
}

impl Default for UiStore {
    fn default() -> Self {
        Self::new()
    }
}

impl UiStore {
    /// A UI store with nothing overlaid.
    pub fn new() -> Self {
        Self {
            popup: Popup::None,
            toast: None,
            connection: Connection::Connecting,
            calendar: None,
            focus: Focus::List,
            effects: EffectQueue::default(),
        }
    }

    /// The overlay currently open.
    pub fn popup(&self) -> Popup {
        self.popup
    }

    /// The pane the next key press is aimed at.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// The message on screen, if any.
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_ref().map(|toast| toast.message.as_str())
    }

    /// Whether the daemon answered the last command.
    pub fn connection(&self) -> Connection {
        self.connection
    }

    /// The day the calendar has highlighted, if the calendar is open.
    ///
    /// This is where a key press asking to pick a date gets the date from: the
    /// loop cannot know which day the user left the highlight on.
    pub fn calendar(&self) -> Option<NaiveDate> {
        self.calendar
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

            // Deleting is the one thing here that cannot be undone from the
            // keyboard, so it asks first. The task itself waits in the store
            // that owns it; this store only remembers that a question is open.
            Action::ConfirmDelete(_) => self.popup = Popup::Confirm,

            // The picker opens on today rather than on whatever the field
            // already says. Seeding it from the field would need the field, and
            // a store cannot read another store; the user can walk to it with
            // the arrows in two presses rather than in thirty.
            Action::OpenCalendar => {
                self.calendar = Some(Local::now().date_naive());
                self.popup = Popup::Calendar;
            }

            Action::CalendarMove(days) => {
                if let Some(day) = self.calendar
                    && let Some(moved) = day.checked_add_signed(TimeDelta::days(i64::from(days)))
                {
                    self.calendar = Some(moved);
                }
            }

            // Answering the question closes it, whichever way it was answered.
            // The store holding the task reads the same action and does the work.
            Action::Submit if self.popup == Popup::Confirm => self.popup = Popup::None,

            // A day cannot be picked without the picker having been open, so the
            // picker is the one thing that could be asked: submitting in the
            // calendar means the day it has highlighted.
            Action::Submit | Action::PickDate(_) if self.popup == Popup::Calendar => {
                self.popup = Popup::Calendar.close();
                self.calendar = None;
            }

            // Cancel unwinds one layer at a time. Once no overlay is open
            // there is nothing left to unwind and nothing to close either:
            // the interface is the whole screen, and it is left up. Quitting
            // is `q` or `ctrl-c`, which say so on the way out.
            Action::Cancel => match self.popup {
                Popup::None => {}
                other => {
                    self.popup = other.close();
                    // The highlight was only ever meant for the picker that is
                    // now gone, and a stale one would answer the next open.
                    self.calendar = None;
                }
            },

            Action::DismissToast => self.toast = None,

            // Moving focus is the one thing a key does that has no business
            // being undone by closing an overlay, so it is answered here and
            // nowhere else.
            Action::FocusNextPane | Action::FocusPrevPane => self.focus = self.focus.other(),

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

            Action::Toast(ref message) => self.show_toast(message),

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
        // Nothing to say is not worth a box on the screen saying nothing.
        if message.is_empty() {
            return;
        }
        self.toast = Some(Toast {
            message: message.to_string(),
            ticks_left: TOAST_TICKS,
        });
    }
}

#[cfg(test)]
mod toast_tests {
    use super::*;

    #[test]
    fn a_toast_action_shows_a_message() {
        let mut ui = UiStore::new();

        ui.update(Action::Toast("saved".to_string()));

        assert_eq!(ui.toast(), Some("saved"));
    }

    #[test]
    fn an_empty_toast_is_worthless() {
        let mut ui = UiStore::new();

        ui.update(Action::Toast(String::new()));

        assert_eq!(ui.toast(), None);
    }

    #[test]
    fn a_toast_fades_like_any_other() {
        let mut ui = UiStore::new();
        ui.update(Action::Toast("saved".to_string()));

        for _ in 0..TOAST_TICKS {
            assert_eq!(ui.toast(), Some("saved"));
            ui.update(Action::Tick);
        }

        assert_eq!(ui.toast(), None);
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

    // ── focus ──────────────────────────────────────────────────────────

    #[test]
    fn starts_with_the_list_focused() {
        assert_eq!(UiStore::default().focus(), Focus::List);
    }

    #[test]
    fn focus_walks_between_the_list_and_the_sidebar() {
        let mut store = UiStore::default();

        send(&mut store, Action::FocusNextPane);
        assert_eq!(store.focus(), Focus::Sidebar);

        send(&mut store, Action::FocusNextPane);
        assert_eq!(store.focus(), Focus::List);
    }

    #[test]
    fn focus_walks_backwards_too() {
        let mut store = UiStore::default();
        send(&mut store, Action::FocusNextPane);

        send(&mut store, Action::FocusPrevPane);

        assert_eq!(
            store.focus(),
            Focus::List,
            "with two panes, walking back from the sidebar has to land somewhere"
        );
    }

    #[test]
    fn a_pane_with_nothing_in_it_still_takes_the_focus() {
        // The stores do not know how wide the terminal is, so a narrow window
        // cannot stop the focus here. What keeps `j` from jumping a list the
        // user cannot see is the sidebar being drawn out of the way instead.
        let mut store = UiStore::default();

        send(&mut store, Action::FocusNextPane);

        assert_eq!(store.focus(), Focus::Sidebar);
    }

    #[test]
    fn focusing_a_pane_is_not_undone_by_a_cancel() {
        let mut store = UiStore::default();
        send(&mut store, Action::FocusNextPane);

        send(&mut store, Action::Cancel);

        assert_eq!(
            store.focus(),
            Focus::Sidebar,
            "there is nothing open for cancel to close"
        );
    }

    #[test]
    fn starts_with_nothing_overlaid() {
        let store = UiStore::default();
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
    fn cancel_with_nothing_open_does_nothing() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::Cancel);
        assert!(effects.is_empty());
        assert_eq!(store.connection(), Connection::Connecting);
    }

    #[test]
    fn the_picker_opens_on_today_with_the_highlight_on_it() {
        let mut store = UiStore::default();

        send(&mut store, Action::OpenCalendar);

        assert_eq!(store.popup(), Popup::Calendar);
        assert_eq!(store.calendar(), Some(chrono::Local::now().date_naive()));
    }

    #[test]
    fn moving_the_highlight_keeps_the_picker_open() {
        let mut store = UiStore::default();
        send(&mut store, Action::OpenCalendar);
        let today = store.calendar().unwrap();

        send(&mut store, Action::CalendarMove(1));
        assert_eq!(store.calendar(), Some(today.succ_opt().unwrap()));
        assert_eq!(store.popup(), Popup::Calendar);

        // A week back from tomorrow: six days before today.
        send(&mut store, Action::CalendarMove(-7));
        assert_eq!(
            store.calendar(),
            today
                .succ_opt()
                .unwrap()
                .checked_sub_signed(TimeDelta::days(7)),
        );
    }

    #[test]
    fn a_highlight_that_would_leave_the_calendar_does_not_move() {
        let mut store = UiStore::default();
        send(&mut store, Action::OpenCalendar);
        let today = store.calendar().unwrap();

        // Two billion days is no day at all, and a highlight with nowhere to
        // stand is better than one wrapped round to the other end.
        send(&mut store, Action::CalendarMove(i32::MAX));

        assert_eq!(store.calendar(), Some(today));
    }

    #[test]
    fn moving_before_the_picker_is_open_does_nothing() {
        let mut store = UiStore::default();
        send(&mut store, Action::CalendarMove(3));
        assert_eq!(store.calendar(), None);
    }

    #[test]
    fn cancelling_the_picker_forgets_the_day() {
        let mut store = UiStore::default();
        send(&mut store, Action::OpenCalendar);

        send(&mut store, Action::Cancel);

        assert_eq!(store.popup(), Popup::None);
        assert_eq!(store.calendar(), None, "the highlight outlived the picker");
    }

    #[test]
    fn a_picked_day_also_closes_the_picker() {
        let mut store = UiStore::default();
        send(&mut store, Action::OpenCalendar);

        send(
            &mut store,
            Action::PickDate(NaiveDate::from_ymd_opt(2026, 3, 17).unwrap()),
        );

        assert_eq!(store.popup(), Popup::None);
        assert_eq!(store.calendar(), None);
    }

    #[test]
    fn actions_for_other_stores_are_ignored() {
        let mut store = UiStore::default();
        let effects = send(&mut store, Action::Select(3));
        assert!(effects.is_empty());
        assert_eq!(store.popup(), Popup::None);
    }
}
