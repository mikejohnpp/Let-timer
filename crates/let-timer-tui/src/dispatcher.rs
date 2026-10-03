//! The dispatcher: the one route an [`Action`] takes through the application.
//!
//! It holds no application logic. It hands the action to the stores, then
//! collects the [`Effect`]s they asked for, which is what makes the flow
//! unidirectional: input → action → store → effect → event loop.

use crate::action::Action;
use crate::effect::Effect;
use crate::store::ui_store::Mode;
use crate::store::{FormStore, MediaListStore, Store, TaskStore, UiStore, WorkspaceStore};

/// Routes actions to the stores and gathers the resulting effects.
#[derive(Debug)]
pub struct Dispatcher {
    ui: UiStore,
    tasks: TaskStore,
    workspaces: WorkspaceStore,
    media_lists: MediaListStore,
    form: FormStore,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new(Mode::Fullscreen)
    }
}

impl Dispatcher {
    /// A dispatcher with empty stores, running as `mode`.
    pub fn new(mode: Mode) -> Self {
        Self {
            ui: UiStore::new(mode),
            tasks: TaskStore::new(),
            workspaces: WorkspaceStore::new(),
            media_lists: MediaListStore::new(),
            form: FormStore::new(),
        }
    }

    /// The UI chrome store.
    pub fn ui(&self) -> &UiStore {
        &self.ui
    }

    /// The task list store.
    pub fn tasks(&self) -> &TaskStore {
        &self.tasks
    }

    /// The workspace list store.
    pub fn workspaces(&self) -> &WorkspaceStore {
        &self.workspaces
    }

    /// The media list store.
    pub fn media_lists(&self) -> &MediaListStore {
        &self.media_lists
    }

    /// The open form, if any.
    pub fn form(&self) -> &FormStore {
        &self.form
    }

    /// Hand `action` to every store and return what they want done.
    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        // Cancel is the one action two layers both want, so it is unwound one
        // layer at a time instead of being broadcast: an overlay on top, then
        // the form, then the panel itself. Broadcasting it would close the
        // form and the panel on the same key press.
        if matches!(action, Action::Cancel) {
            if self.ui.has_popup() {
                let mut effects = Self::collect_from(&mut self.ui, action.clone());
                // An overlay can be asking about something a store is holding,
                // and closing it has to reach that store too, or the store keeps
                // a pending delete that nothing will ever confirm. The form is
                // left out on purpose: the overlay was on top of it, so it is not
                // the layer being closed.
                effects.extend(Self::collect_from(&mut self.tasks, action.clone()));
                effects.extend(Self::collect_from(&mut self.workspaces, action.clone()));
                effects.extend(Self::collect_from(&mut self.media_lists, action));
                return effects;
            }
            if self.form.is_open() {
                return Self::collect_from(&mut self.form, action);
            }
        }

        let mut effects = Vec::new();
        effects.extend(Self::collect_from(&mut self.ui, action.clone()));
        effects.extend(Self::collect_from(&mut self.tasks, action.clone()));
        effects.extend(Self::collect_from(&mut self.workspaces, action.clone()));
        effects.extend(Self::collect_from(&mut self.media_lists, action.clone()));
        effects.extend(Self::collect_from(&mut self.form, action));
        effects
    }

    /// Give one action to one store and take whatever it asks for.
    ///
    /// Not a method on `&mut self`: the caller already holds a borrow of one
    /// of the stores.
    fn collect_from(store: &mut impl Store, action: Action) -> Vec<Effect> {
        store.update(action);
        store.take_effects()
    }
}

#[cfg(test)]
mod tests {
    use let_timer_core::Command;

    use crate::action::Component;
    use crate::store::Mode;
    use crate::store::test_util::{task_with_id, tasks};

    use super::*;

    fn dispatcher(mode: Mode) -> Dispatcher {
        Dispatcher::new(mode)
    }

    #[test]
    fn a_fresh_dispatcher_has_empty_stores() {
        let dispatcher = dispatcher(Mode::Fullscreen);

        assert!(dispatcher.tasks().is_empty());
        assert!(dispatcher.workspaces().is_empty());
        assert!(dispatcher.media_lists().is_empty());
        assert!(!dispatcher.form().is_open());
        assert_eq!(dispatcher.ui().popup(), crate::store::Popup::None);
    }

    #[test]
    fn a_tick_reaches_every_list_store() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        let effects = dispatcher.dispatch(Action::Tick);

        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::List { .. }))),
            "the task list should refresh"
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::ListWorkspaces))),
            "the workspace list should refresh"
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::ListMediaLists))),
            "the media list should refresh"
        );
    }

    #[test]
    fn an_ipc_reply_lands_in_the_right_store_and_in_the_ui_store() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        let effects = dispatcher.dispatch(Action::TaskListLoaded(tasks(3)));

        assert_eq!(
            dispatcher.tasks().len(),
            3,
            "the reply filled the task list"
        );
        assert!(dispatcher.workspaces().is_empty(), "and only the task list");
        assert_eq!(
            dispatcher.ui().connection(),
            crate::store::Connection::Connected
        );
        assert!(effects.is_empty(), "loading a list asks for nothing back");
    }

    #[test]
    fn a_failure_reaches_the_ui_store_and_every_pending_list() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);
        dispatcher.dispatch(Action::Tick);

        let effects = dispatcher.dispatch(Action::IpcFailed("boom".to_string()));

        assert_eq!(dispatcher.ui().toast(), Some("boom"));
        assert!(
            !dispatcher.tasks().is_pending(),
            "a failed list request must not stall the refresh"
        );
        assert!(!dispatcher.workspaces().is_pending());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_reply_lets_every_list_refresh_again() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);
        dispatcher.dispatch(Action::Tick);
        assert!(dispatcher.tasks().is_pending());

        dispatcher.dispatch(Action::TaskListLoaded(tasks(1)));

        assert!(!dispatcher.tasks().is_pending());
        assert!(
            dispatcher
                .dispatch(Action::Tick)
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::List { .. })))
        );
    }

    #[test]
    fn quitting_produces_exactly_one_quit_effect() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        let effects = dispatcher.dispatch(Action::Quit);

        assert_eq!(effects.len(), 1);
        assert!(matches!(effects.as_slice(), [Effect::Quit]));
    }

    #[test]
    fn a_task_reply_fills_the_list_and_the_selection_follows_it() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        dispatcher.dispatch(Action::TaskListLoaded(tasks(3)));
        dispatcher.dispatch(Action::Select(2));
        dispatcher.dispatch(Action::TaskSaved(task_with_id(99)));

        assert_eq!(dispatcher.tasks().len(), 4);
        assert_eq!(
            dispatcher.tasks().selected_task().map(|task| task.id),
            Some(99)
        );
    }

    #[test]
    fn selecting_moves_the_task_highlight() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);
        dispatcher.dispatch(Action::TaskListLoaded(tasks(4)));

        dispatcher.dispatch(Action::MoveSelection(2));

        assert_eq!(dispatcher.tasks().selected(), 2);
    }

    #[test]
    fn submitting_a_filled_form_sends_its_command() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);
        dispatcher.dispatch(Action::OpenCreate(Component::Workspace));
        for character in "Side quest".chars() {
            dispatcher.dispatch(Action::FormInput(character));
        }

        let effects = dispatcher.dispatch(Action::Submit);

        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::CreateWorkspace(_))))
        );
        assert!(!dispatcher.form().is_open(), "a sent form closes");
    }

    #[test]
    fn a_refused_form_stays_open_with_its_complaints() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);
        dispatcher.dispatch(Action::OpenCreate(Component::Workspace));

        let effects = dispatcher.dispatch(Action::Submit);

        assert!(effects.is_empty());
        assert!(dispatcher.form().is_open());
        assert!(!dispatcher.form().errors().is_empty());
    }

    #[test]
    fn esc_closes_the_form_before_it_closes_the_panel() {
        let mut dispatcher = dispatcher(Mode::Inline { max_height: 20 });
        dispatcher.dispatch(Action::OpenCreate(Component::Workspace));

        let effects = dispatcher.dispatch(Action::Cancel);

        assert!(
            effects.is_empty(),
            "the panel must survive while a form is open"
        );
        assert!(!dispatcher.form().is_open());
    }

    #[test]
    fn esc_closes_an_overlay_before_it_closes_the_form() {
        let mut dispatcher = dispatcher(Mode::Inline { max_height: 20 });
        dispatcher.dispatch(Action::OpenCreate(Component::Workspace));
        dispatcher.dispatch(Action::Help);

        dispatcher.dispatch(Action::Cancel);

        assert_eq!(dispatcher.ui().popup(), crate::store::Popup::None);
        assert!(
            dispatcher.form().is_open(),
            "the form is still under the overlay"
        );
    }

    #[test]
    fn esc_with_nothing_open_closes_an_inline_panel() {
        let mut dispatcher = dispatcher(Mode::Inline { max_height: 20 });

        let effects = dispatcher.dispatch(Action::Cancel);

        assert!(matches!(effects.as_slice(), [Effect::Quit]));
    }

    #[test]
    fn esc_with_nothing_open_does_nothing_in_fullscreen() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        let effects = dispatcher.dispatch(Action::Cancel);

        assert!(effects.is_empty());
    }

    #[test]
    fn effects_are_drained_between_dispatches() {
        let mut dispatcher = dispatcher(Mode::Fullscreen);

        assert!(!dispatcher.dispatch(Action::Quit).is_empty());
        assert!(
            dispatcher.dispatch(Action::Select(1)).is_empty(),
            "the queue must not hand out the same effect twice"
        );
    }
}
