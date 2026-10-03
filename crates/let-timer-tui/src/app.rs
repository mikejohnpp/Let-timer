//! The event loop: everything that happens between a key press and a redraw.
//!
//! The loop does three things and nothing else. It turns what arrives into an
//! [`Action`], hands the action to the [`Dispatcher`], and carries out whatever
//! [`Effect`]s come back. It makes no decisions of its own: what a key means is
//! [`App::action_for_key`], and what an action changes is the stores' business.
//!
//! Three things can interrupt the loop, and all three arrive through
//! [`Input`]: a key, a tick, and a reply from the daemon. A fourth, a redraw
//! on its own, is a resize. A key the user pressed and a list the daemon
//! finished sending are the same to the loop, and the one the user is looking
//! at is a new one, so a redraw is a good idea every time round.

use let_timer_core::Task;
use ratatui::Frame;
use tokio::sync::mpsc;

use crate::action::{Action, Component};
use crate::config::keymap::{Context, KeyMap, Target, typed_char};
use crate::dispatcher::Dispatcher;
use crate::effect::Effect;
use crate::event::{AppEvent, EventError, Events};
use crate::ipc::IpcActor;
use crate::store::{FieldKind, Popup};
use crate::terminal::{Screen, TerminalError};

/// Something that arrived while the loop was waiting.
#[derive(Debug, Clone)]
enum Input {
    /// From the terminal or the clock.
    Event(AppEvent),
    /// A reply, or the failure of one, from the daemon.
    FromDaemon(Action),
}

/// Why the loop stopped.
#[derive(Debug)]
pub enum AppError {
    /// Events stopped arriving, or failed.
    Events(EventError),
    /// The terminal could not be drawn on.
    Terminal(TerminalError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Events(error) => write!(f, "{error}"),
            AppError::Terminal(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Events(error) => Some(error),
            AppError::Terminal(error) => Some(error),
        }
    }
}

impl From<EventError> for AppError {
    fn from(error: EventError) -> Self {
        AppError::Events(error)
    }
}

impl From<TerminalError> for AppError {
    fn from(error: TerminalError) -> Self {
        AppError::Terminal(error)
    }
}

/// The application: the stores, the keymap, and what is on screen.
pub struct App {
    dispatcher: Dispatcher,
    keymap: KeyMap,
    component: Component,
    context: Context,
}

impl App {
    /// An application showing `component`, reading keys with `keymap`.
    pub fn new(component: Component, keymap: KeyMap) -> Self {
        Self {
            dispatcher: Dispatcher::new(),
            keymap,
            component,
            context: Context::Normal,
        }
    }

    /// The stores, for drawing from.
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// The record type on screen.
    pub fn component(&self) -> Component {
        self.component
    }

    /// The key bindings in force.
    pub fn keymap(&self) -> &KeyMap {
        &self.keymap
    }

    /// Which set of bindings the next key press will be read against.
    ///
    /// The calendar is a modal of its own with keys of its own, and the arrows
    /// that move a list move the day inside it, so the context follows whatever
    /// is on top rather than being fixed at startup.
    pub fn context(&self) -> Context {
        if self.dispatcher.ui().popup() == Popup::Calendar {
            return Context::Calendar;
        }
        self.context
    }

    /// What this key press means right now.
    ///
    /// "Right now" is the whole difficulty. The same key can be a command, a
    /// letter in a form, or nothing at all, and which one depends on what is
    /// open and what the bindings say. Three layers get a say, outermost first:
    /// an overlay, then the form, then the list.
    pub fn action_for_key(&self, key: crossterm::event::KeyEvent) -> Option<Action> {
        let target = self.keymap.resolve(self.context(), &key);

        // An overlay is a modal: the keys underneath it are not the user's
        // business right now, and letting them through would move a list
        // selection behind a help popup nobody can see.
        if self.dispatcher.ui().has_popup() {
            return match (target, self.dispatcher.ui().popup()) {
                (Some(Target::Cancel), _) => Some(Action::Cancel),
                // In the calendar, submitting means picking the day it has
                // highlighted. The key cannot name a date, so it is looked up.
                (Some(Target::Submit), Popup::Calendar) => {
                    self.dispatcher.ui().calendar().map(Action::PickDate)
                }
                (Some(Target::Submit), _) => Some(Action::Submit),
                // The calendar is a modal that has keys of its own; every other
                // overlay swallows everything it does not use, because there is
                // nothing behind it worth acting on.
                (target, Popup::Calendar) => {
                    target.and_then(|target| target.into_action(self.component))
                }
                _ => None,
            };
        }

        // A form swallows most keys before the list ever hears about them.
        if self.dispatcher.form().is_open() {
            if let Some(action) = self.action_for_form_key(target, &key) {
                return Some(action);
            }
            // The form had no use for this key. Most commands are dropped: one
            // pressed mid-sentence is almost never what was meant.
            if !reaches_the_list_behind_a_form(target) {
                return None;
            }
        }

        match target {
            // These two need to know which row is selected, which is something
            // the keymap cannot know.
            Some(Target::OpenEdit) => self
                .selected_task()
                .map(|task| Action::OpenEdit(Box::new(task))),
            Some(Target::ConfirmDelete) => self
                .selected_task()
                .map(|task| Action::ConfirmDelete(Box::new(task))),
            // A date picker with no form has no date field to fill in, so the
            // key does nothing rather than opening a calendar over a list.
            Some(Target::OpenCalendar) => None,
            other => other.and_then(|target| target.into_action(self.component)),
        }
    }

    /// What this key means to an open form.
    ///
    /// `None` means the form has no use for the key, so it goes on to the list.
    fn action_for_form_key(
        &self,
        target: Option<Target>,
        key: &crossterm::event::KeyEvent,
    ) -> Option<Action> {
        // These mean the same thing whatever field has focus.
        match target {
            Some(Target::Cancel) => return Some(Action::Cancel),
            Some(Target::Submit) => return Some(Action::Submit),
            Some(Target::NextField) => return Some(Action::FormNextField),
            Some(Target::PreviousField) => return Some(Action::FormPrevField),
            Some(Target::OpenCalendar) => return Some(Action::OpenCalendar),
            Some(Target::ClearDate) => return Some(Action::ClearDate),
            _ => {}
        }

        // A field chosen by cycling rather than typing. Arrows drive the choice
        // once the form is drawn; until then the field keeps whatever it was
        // given instead of filling up with letters that cannot parse.
        let draft = self.dispatcher.form().draft()?;
        if !accepts_typing(draft.focused_kind()) {
            return None;
        }

        if key.code == crossterm::event::KeyCode::Backspace {
            return Some(Action::FormBackspace);
        }
        typed_char(key).map(Action::FormInput)
    }

    /// The task on the highlighted row, if there is one.
    ///
    /// Only tasks: the workspace and media-list forms have no edit form yet,
    /// and offering one that cannot be filled in would be worse than not
    /// offering it.
    fn selected_task(&self) -> Option<Task> {
        match self.component {
            Component::Task => self.dispatcher.tasks().selected_task().cloned(),
            Component::Workspace | Component::MediaList => None,
        }
    }

    /// Feed one action in and get whatever should happen because of it.
    ///
    /// An [`Effect::Quit`] in the answer means the application is finished.
    pub fn react(&mut self, action: Action) -> Vec<Effect> {
        self.dispatcher.dispatch(action)
    }

    /// Run until the user leaves, the input ends, or something breaks.
    ///
    /// `render` draws the current state; it is passed in rather than named here
    /// so that this file stays about the loop and not about how things look.
    pub async fn run<F>(
        mut self,
        screen: &mut Screen,
        events: &mut Events,
        render: F,
    ) -> Result<(), AppError>
    where
        F: Fn(&mut Frame, &App),
    {
        let actor = IpcActor::spawn(let_timer_core::ipc::socket_path());
        let (outbox, mut inbox) = mpsc::unbounded_channel::<Action>();

        // Ask for the list straight away rather than waiting out a whole period
        // on a screen with nothing in it.
        let effects = self.react(Action::Tick);
        self.carry_out(effects, &actor, &outbox);
        screen.draw(|frame| render(frame, &self))?;

        loop {
            let Some(input) = next_input(events, &mut inbox).await? else {
                // The input ended. The user closed it, or the terminal went
                // away; either way there is nothing left to wait for.
                return Ok(());
            };

            let action = match input {
                Input::Event(AppEvent::Key(key)) => self.action_for_key(key),
                Input::Event(AppEvent::Tick) => Some(Action::Tick),
                // A resize has no action: the next draw picks up the new size.
                Input::Event(AppEvent::Resize) => None,
                Input::FromDaemon(action) => Some(action),
            };

            let Some(action) = action else {
                screen.draw(|frame| render(frame, &self))?;
                continue;
            };

            let effects = self.react(action);
            let finished = self.carry_out(effects, &actor, &outbox);

            screen.draw(|frame| render(frame, &self))?;

            if finished {
                return Ok(());
            }
        }
    }

    /// Do what the stores asked, and say whether the application is finished.
    fn carry_out(
        &self,
        effects: Vec<Effect>,
        actor: &IpcActor,
        outbox: &mpsc::UnboundedSender<Action>,
    ) -> bool {
        let mut finished = false;

        for effect in effects {
            match effect {
                Effect::Send(command) => {
                    let actor = actor.clone();
                    let outbox = outbox.clone();
                    tokio::spawn(async move {
                        let action = match actor.send(command).await {
                            Ok(response) => Action::from_response(response),
                            Err(message) => Some(Action::IpcFailed(message)),
                        };
                        if let Some(action) = action {
                            // Sending to an application that has already gone is
                            // not worth an error.
                            let _ = outbox.send(action);
                        }
                    });
                }
                // A toast cannot write to a store, so it goes back in as an
                // action and is routed like any other.
                Effect::Toast(message) => {
                    let _ = outbox.send(Action::Toast(message));
                }
                Effect::Quit => finished = true,
            }
        }

        finished
    }
}

/// Wait for the next thing to arrive, wherever it comes from.
async fn next_input(
    events: &mut Events,
    inbox: &mut mpsc::UnboundedReceiver<Action>,
) -> Result<Option<Input>, EventError> {
    tokio::select! {
        event = events.next() => event.map(|event| event.map(Input::Event)),
        action = inbox.recv() => Ok(action.map(Input::FromDaemon)),
    }
}

/// Whether a key the form had no use for may still act on the list.
///
/// Moving the highlight is harmless, and quitting has to get through: a user
/// stuck in a form they cannot finish needs ctrl-c to work exactly as it does
/// everywhere else. Everything else is a command aimed at whatever is behind the
/// form, which is not what somebody typing a name meant to do.
fn reaches_the_list_behind_a_form(target: Option<Target>) -> bool {
    target.is_some_and(|target| target.is_navigation() || matches!(target, Target::Quit))
}

/// Whether this field is typed into rather than picked from a list.
fn accepts_typing(kind: FieldKind) -> bool {
    !matches!(kind, FieldKind::Priority | FieldKind::Status)
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use let_timer_core::Command;

    use crate::store::{Draft, Popup};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn app() -> App {
        App::new(Component::Task, KeyMap::defaults())
    }

    /// One task with the given id, for actions that carry a record.
    fn test_tasks(count: i64) -> Vec<Task> {
        crate::store::test_util::tasks(count)
    }

    fn with_tasks(app: &mut App, count: i64) {
        let effects = app.react(Action::TaskListLoaded(crate::store::test_util::tasks(
            count,
        )));
        assert!(
            effects.is_empty(),
            "a list that just arrived has nothing left to ask the daemon for"
        );
    }

    // ── a key on a plain list ──────────────────────────────────────────

    #[test]
    fn a_command_key_does_what_it_says() {
        let app = app();

        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('j'))),
            Some(Action::MoveSelection(1))
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('q'))),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn an_arrow_key_and_its_letter_agree() {
        let app = app();

        assert!(matches!(
            app.action_for_key(key(KeyCode::Down)),
            Some(Action::MoveSelection(1))
        ));
    }

    #[test]
    fn ctrl_c_quits() {
        let app = app();
        assert!(matches!(
            app.action_for_key(ctrl(KeyCode::Char('c'))),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn a_bound_key_the_user_pressed_becomes_the_components_create_form() {
        let app = App::new(Component::Workspace, KeyMap::defaults());

        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('n'))),
            Some(Action::OpenCreate(Component::Workspace))
        ));
    }

    #[test]
    fn an_unbound_key_does_nothing() {
        let app = app();
        assert!(app.action_for_key(key(KeyCode::F(9))).is_none());
    }

    // ── keys that need a selected row ──────────────────────────────────

    #[test]
    fn edit_with_nothing_selected_does_nothing() {
        let app = app();

        assert!(app.action_for_key(key(KeyCode::Char('e'))).is_none());
    }

    #[test]
    fn edit_opens_the_selected_task() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::Select(1));

        let Some(Action::OpenEdit(task)) = app.action_for_key(key(KeyCode::Char('e'))) else {
            panic!("expected an edit form for the selected task");
        };
        assert_eq!(task.id, 2);
    }

    #[test]
    fn delete_carries_the_task_not_a_row_number() {
        // A refresh between asking to delete and saying yes would otherwise
        // leave a row number pointing at a different task.
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::Select(2));

        let Some(Action::ConfirmDelete(task)) = app.action_for_key(key(KeyCode::Char('d'))) else {
            panic!("expected a delete confirmation for the selected task");
        };
        assert_eq!(task.id, 3);
    }

    #[test]
    fn delete_with_nothing_selected_does_nothing() {
        let app = app();
        assert!(app.action_for_key(key(KeyCode::Char('d'))).is_none());
    }

    #[test]
    fn only_tasks_offer_an_edit_form() {
        // The workspace form has nothing to fill in yet, so offering one would
        // be a dead end.
        for component in [Component::Workspace, Component::MediaList] {
            let app = App::new(component, KeyMap::defaults());
            assert!(
                app.action_for_key(key(KeyCode::Char('e'))).is_none(),
                "{component:?}"
            );
            assert!(
                app.action_for_key(key(KeyCode::Char('d'))).is_none(),
                "{component:?}"
            );
        }
    }

    // ── a form is open ─────────────────────────────────────────────────

    #[test]
    fn a_letter_types_into_an_open_form() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        for letter in "hi".chars() {
            assert!(matches!(
                app.action_for_key(key(KeyCode::Char(letter))),
                Some(Action::FormInput(c)) if c == letter
            ));
        }
    }

    #[test]
    fn a_letter_becomes_a_command_once_the_form_is_closed() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));
        assert!(
            matches!(
                app.action_for_key(key(KeyCode::Char('n'))),
                Some(Action::FormInput('n'))
            ),
            "the form has the letter first"
        );

        app.react(Action::Cancel);

        assert!(
            matches!(
                app.action_for_key(key(KeyCode::Char('n'))),
                Some(Action::OpenCreate(_))
            ),
            "and the list has it once the form is gone"
        );
    }

    #[test]
    fn space_types_a_space() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Char(' '))),
            Some(Action::FormInput(' '))
        ));
    }

    #[test]
    fn backspace_deletes_rather_than_types() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Backspace)),
            Some(Action::FormBackspace)
        ));
    }

    #[test]
    fn ctrl_a_does_not_type_an_a() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(app.action_for_key(ctrl(KeyCode::Char('a'))).is_none());
    }

    #[test]
    fn tab_moves_between_fields_instead_of_typing() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Tab)),
            Some(Action::FormNextField)
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::BackTab)),
            Some(Action::FormPrevField)
        ));
    }

    #[test]
    fn enter_submits_and_escape_leaves() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Enter)),
            Some(Action::Submit)
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Esc)),
            Some(Action::Cancel)
        ));
    }

    #[test]
    fn the_letter_q_is_not_quit_while_a_form_is_open() {
        // Otherwise nobody could type a q into a task name.
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('q'))),
            Some(Action::FormInput('q'))
        ));
    }

    /// The open draft, for tests that walk the fields.
    fn form(app: &App) -> &Draft {
        app.dispatcher()
            .form()
            .draft()
            .expect("these tests all open a form first")
    }

    #[test]
    fn pressing_d_then_enter_deletes_the_task_the_dialog_named() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::MoveSelection(1));

        let asking = app.react(app.action_for_key(key(KeyCode::Char('d'))).unwrap());
        assert!(
            asking.is_empty(),
            "asking a question needs nothing from the daemon yet"
        );
        assert!(
            app.dispatcher().ui().has_popup(),
            "the question is on screen"
        );

        let effects = app.react(Action::Submit);

        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::Delete { id: 2 }))),
            "found {effects:?}"
        );
        assert!(
            !app.dispatcher().ui().has_popup(),
            "the question is answered, whichever way"
        );
    }

    #[test]
    fn pressing_d_then_escape_deletes_nothing() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::ConfirmDelete(Box::new(test_tasks(1).remove(0))));

        let effects = app.react(app.action_for_key(key(KeyCode::Esc)).unwrap());

        assert!(effects.is_empty(), "found {effects:?}");
        assert!(!app.dispatcher().ui().has_popup());
        assert!(
            app.dispatcher().tasks().pending_delete().is_none(),
            "nothing may be left waiting for an answer that is not coming"
        );
    }

    #[test]
    fn a_key_press_while_the_question_is_open_does_not_reach_the_list() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::ConfirmDelete(Box::new(test_tasks(1).remove(0))));

        assert!(app.action_for_key(key(KeyCode::Char('j'))).is_none());
        assert!(app.action_for_key(key(KeyCode::Down)).is_none());
        assert_eq!(
            app.dispatcher().tasks().selected(),
            0,
            "the highlight behind a dialog is not the user's business"
        );
    }

    #[test]
    fn ctrl_c_is_not_a_command_aimed_at_the_list() {
        assert!(reaches_the_list_behind_a_form(Some(Target::MoveDown)));
        assert!(reaches_the_list_behind_a_form(Some(Target::Quit)));
        assert!(!reaches_the_list_behind_a_form(Some(Target::ConfirmDelete)));
        assert!(!reaches_the_list_behind_a_form(Some(Target::OpenCreate)));
        assert!(!reaches_the_list_behind_a_form(None));
    }

    #[test]
    fn a_command_pressed_on_a_field_that_cannot_be_typed_into_is_dropped() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        // Walk the focus onto the priority field, which is picked by cycling
        // rather than typed into.
        while form(&app).focused_kind() != FieldKind::Priority {
            app.react(Action::FormNextField);
        }

        assert!(app.action_for_key(key(KeyCode::Char('d'))).is_none());
        assert!(app.action_for_key(key(KeyCode::Char('n'))).is_none());
        assert!(
            !app.dispatcher().ui().has_popup(),
            "a delete dialog over an unfinished name is the worst possible reading of d"
        );
    }

    #[test]
    fn ctrl_c_still_quits_while_a_form_is_open() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(ctrl(KeyCode::Char('c'))),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn a_field_that_is_picked_rather_than_typed_swallows_nothing() {
        // Priority is chosen by cycling. Filling it with letters would only
        // produce a value the daemon refuses.
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        // Focus lands on the name first; move to priority.
        while form(&app).focused_kind() != FieldKind::Priority {
            app.react(Action::FormNextField);
        }

        assert!(app.action_for_key(key(KeyCode::Char('h'))).is_none());
        assert!(app.action_for_key(key(KeyCode::Backspace)).is_none());
    }

    #[test]
    fn a_choice_field_still_lets_tab_through() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        // The new-task form picks priority by cycling, so this is the one
        // field that cannot be typed into.
        while form(&app).focused_kind() != FieldKind::Priority {
            app.react(Action::FormNextField);
        }

        assert!(matches!(
            app.action_for_key(key(KeyCode::Tab)),
            Some(Action::FormNextField),
        ));
    }

    #[test]
    fn arrows_reach_the_list_while_a_form_is_open() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));

        assert!(matches!(
            app.action_for_key(key(KeyCode::Down)),
            Some(Action::MoveSelection(1))
        ));
    }

    // ── an overlay is open ─────────────────────────────────────────────

    #[test]
    fn an_overlay_keeps_the_keys_underneath_off() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::Help);
        assert_eq!(app.dispatcher().ui().popup(), Popup::Help);

        // Moving the list behind a popup nobody can see is not what was meant.
        assert!(app.action_for_key(key(KeyCode::Char('j'))).is_none());
        assert!(app.action_for_key(key(KeyCode::Down)).is_none());
    }

    #[test]
    fn an_overlay_still_closes() {
        let mut app = app();
        app.react(Action::Help);

        assert!(matches!(
            app.action_for_key(key(KeyCode::Esc)),
            Some(Action::Cancel)
        ));
    }

    #[test]
    fn an_overlay_on_top_of_a_form_swallows_typing() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Workspace));
        app.react(Action::Help);

        assert!(app.action_for_key(key(KeyCode::Char('x'))).is_none());
    }

    #[test]
    fn an_overlay_leaves_the_keys_working_once_it_is_closed() {
        let mut app = app();
        app.react(Action::Help);
        app.react(Action::Cancel);
        assert_eq!(app.dispatcher().ui().popup(), Popup::None);

        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('j'))),
            Some(Action::MoveSelection(1))
        ));
    }

    // ── the date picker ────────────────────────────────────────────────

    #[test]
    fn the_picker_opens_only_from_a_form_with_a_date_to_fill_in() {
        let mut app = app();
        with_tasks(&mut app, 3);

        // No form, so there is no date field: the key does nothing rather than
        // opening a picker over a list.
        assert!(app.action_for_key(key(KeyCode::Char('c'))).is_none());

        app.react(Action::OpenCreate(Component::Task));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Char('c'))),
            Some(Action::OpenCalendar)
        ));
    }

    #[test]
    fn the_picker_becomes_the_context_the_next_key_is_read_against() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::OpenCalendar);

        assert_eq!(app.context(), Context::Calendar);

        app.react(Action::Cancel);
        assert_eq!(
            app.context(),
            Context::Normal,
            "closing the picker goes back to the screen behind it"
        );
    }

    #[test]
    fn inside_the_picker_the_arrows_move_the_day_and_not_the_list() {
        let mut app = app();
        with_tasks(&mut app, 3);
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::OpenCalendar);

        assert!(matches!(
            app.action_for_key(key(KeyCode::Right)),
            Some(Action::CalendarMove(1))
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Left)),
            Some(Action::CalendarMove(-1))
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Down)),
            Some(Action::CalendarMove(7))
        ));
        assert!(matches!(
            app.action_for_key(key(KeyCode::Up)),
            Some(Action::CalendarMove(-7))
        ));
        // The list's own bindings would move a selection the form is covering.
        assert!(app.action_for_key(key(KeyCode::Char('j'))).is_none());
    }

    #[test]
    fn pressing_enter_in_the_picker_picks_the_day_it_has_highlighted() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::OpenCalendar);
        app.react(Action::CalendarMove(1));
        let highlighted = app.dispatcher().ui().calendar().unwrap();

        let action = app
            .action_for_key(key(KeyCode::Enter))
            .expect("enter picks a day");

        assert!(
            matches!(action, Action::PickDate(day) if day == highlighted),
            "found {action:?}"
        );
    }

    #[test]
    fn the_day_the_picker_gave_the_form_is_in_the_date_field() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::OpenCalendar);
        let day = app.dispatcher().ui().calendar().unwrap();
        app.react(Action::CalendarMove(-2));

        // Enter is the only key needed: picking closes the picker on its own,
        // and a second escape here would close the form with it.
        app.react(app.action_for_key(key(KeyCode::Enter)).unwrap());

        assert_eq!(app.dispatcher().ui().calendar(), None, "the picker is done");
        let picked = day.pred_opt().unwrap().pred_opt().unwrap();
        assert_eq!(
            app.dispatcher()
                .form()
                .draft()
                .expect("the form is still open")
                .value(FieldKind::ScheduledOn),
            Some(picked.format("%Y-%m-%d").to_string().as_str()),
            "the two days the arrows walked, written as the daemon will read them"
        );
    }

    #[test]
    fn the_picker_closes_on_escape_without_touching_the_field() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::OpenCalendar);

        assert!(matches!(
            app.action_for_key(key(KeyCode::Esc)),
            Some(Action::Cancel)
        ));
        app.react(Action::Cancel);

        assert_eq!(app.dispatcher().ui().popup(), Popup::None);
        assert_eq!(
            app.dispatcher()
                .form()
                .draft()
                .unwrap()
                .value(FieldKind::ScheduledOn),
            Some(""),
            "escaping out of the picker leaves the field as it was"
        );
    }

    // ── reacting ───────────────────────────────────────────────────────

    #[test]
    fn reacting_to_quit_says_so() {
        let mut app = app();

        let effects = app.react(Action::Quit);

        assert!(effects.iter().any(|effect| matches!(effect, Effect::Quit)));
    }

    #[test]
    fn reacting_to_a_tick_asks_the_daemon_for_the_list() {
        let mut app = app();

        let effects = app.react(Action::Tick);

        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::List { .. })))
        );
    }

    #[test]
    fn reacting_to_a_reply_keeps_it() {
        let mut app = app();
        let tasks = crate::store::test_util::tasks(2);

        let effects = app.react(Action::TaskListLoaded(tasks));

        assert!(effects.is_empty());
        assert_eq!(app.dispatcher().tasks().len(), 2);
    }

    // ── the parts worth testing on their own ───────────────────────────

    #[test]
    fn typed_fields_are_typed_and_choice_fields_are_not() {
        assert!(accepts_typing(FieldKind::Name));
        assert!(accepts_typing(FieldKind::Description));
        assert!(accepts_typing(FieldKind::WorkspaceId));
        assert!(!accepts_typing(FieldKind::Priority));
        assert!(!accepts_typing(FieldKind::Status));
    }
}
