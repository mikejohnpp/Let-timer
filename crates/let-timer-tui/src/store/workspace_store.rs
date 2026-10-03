//! The workspace list.
//!
//! Same shape as the task list: rows the user can move a highlight through,
//! refreshed on every tick. Workspaces only support listing and creating, so
//! there is no edit or delete path here yet.

use let_timer_core::{Command, Workspace};

use crate::action::Action;
use crate::effect::Effect;
use crate::store::list_state::{Identified, ListState};
use crate::store::{EffectQueue, Store};

impl Identified for Workspace {
    fn id(&self) -> i64 {
        self.id
    }
}

/// A list of workspaces plus the highlighted row.
#[derive(Debug, Default)]
pub struct WorkspaceStore {
    list: ListState<Workspace>,
    /// Whether a list request is still in flight, so ticks do not pile up
    /// another one behind a slow daemon.
    pending: bool,
    effects: EffectQueue,
}

impl WorkspaceStore {
    /// An empty workspace list with the first row highlighted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every workspace currently loaded, in the order the daemon returned them.
    pub fn workspaces(&self) -> &[Workspace] {
        self.list.rows()
    }

    /// Index of the highlighted row.
    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    /// The highlighted workspace, or `None` when the list is empty.
    pub fn selected_workspace(&self) -> Option<&Workspace> {
        self.list.selected_row()
    }

    /// How many workspaces are loaded.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no workspace has loaded yet.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Whether a list request is waiting for a reply.
    pub fn is_pending(&self) -> bool {
        self.pending
    }
}

impl Store for WorkspaceStore {
    fn update(&mut self, action: Action) {
        match action {
            Action::WorkspaceListLoaded(workspaces) => {
                self.pending = false;
                self.list.replace(workspaces);
            }

            // Creating a workspace answers with the created row.
            Action::WorkspaceSaved(workspace) => self.list.save(workspace),

            // Both outcomes of a list request clear `pending`: a failure that
            // left it set would stall polling for the rest of the session.
            Action::IpcFailed(_) => self.pending = false,

            Action::Tick => self.request_list(),

            // Only the sidebar's own actions move this highlight. `Select` and
            // `MoveSelection` belong to the task list, and answering them here
            // would mean a keystroke in the list walked the sidebar too.
            Action::SelectWorkspace(index) => self.list.select_row(index),
            Action::MoveWorkspaceSelection(delta) => self.list.move_selection(delta),

            // Which workspace the sidebar is sitting on says nothing about what
            // the task list is showing, so nothing changes here. The task store
            // is the one that reads the filter.
            Action::SetWorkspaceFilter(_) => {}

            _ => {}
        }
    }

    fn take_effects(&mut self) -> Vec<Effect> {
        self.effects.drain()
    }
}

impl WorkspaceStore {
    /// Queue a list request unless one is already in flight.
    fn request_list(&mut self) {
        if self.pending {
            return;
        }
        self.pending = true;
        self.effects.push(Effect::Send(Command::ListWorkspaces));
    }
}

#[cfg(test)]
mod tests {
    use crate::store::test_util::{workspace_with_id, workspaces};

    use super::*;

    fn send(store: &mut WorkspaceStore, action: Action) -> Vec<Effect> {
        store.update(action);
        store.take_effects()
    }

    #[test]
    fn starts_empty() {
        let store = WorkspaceStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.selected(), 0);
        assert!(store.selected_workspace().is_none());
    }

    #[test]
    fn a_loaded_list_fills_the_store() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::WorkspaceListLoaded(workspaces(3)));

        assert_eq!(store.len(), 3);
        assert_eq!(store.selected(), 0);
        assert_eq!(store.selected_workspace().map(|w| w.id), Some(1));
    }

    #[test]
    fn a_tick_asks_the_daemon_for_the_workspaces() {
        let mut store = WorkspaceStore::new();
        let effects = send(&mut store, Action::Tick);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Send(Command::ListWorkspaces)]
        ));
    }

    #[test]
    fn a_tick_while_a_request_is_out_does_not_send_another() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::Tick);
        assert!(store.is_pending());

        send(&mut store, Action::Tick);
        assert!(
            send(&mut store, Action::Tick).is_empty(),
            "a slow daemon must not get one request per tick"
        );
    }

    #[test]
    fn a_reply_lets_the_next_tick_refresh_again() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::Tick);
        assert!(store.is_pending());

        send(&mut store, Action::WorkspaceListLoaded(workspaces(2)));
        assert!(!store.is_pending());

        assert!(matches!(
            send(&mut store, Action::Tick).as_slice(),
            [Effect::Send(Command::ListWorkspaces)]
        ));
    }

    #[test]
    fn a_failure_clears_pending_so_polling_recovers() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::Tick);

        send(&mut store, Action::IpcFailed("boom".to_string()));
        assert!(!store.is_pending(), "otherwise polling would stall forever");

        assert!(matches!(
            send(&mut store, Action::Tick).as_slice(),
            [Effect::Send(Command::ListWorkspaces)]
        ));
    }

    #[test]
    fn the_highlight_follows_the_same_workspace_across_a_refresh() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::WorkspaceListLoaded(workspaces(4)));
        send(&mut store, Action::SelectWorkspace(2));

        send(
            &mut store,
            Action::WorkspaceListLoaded(workspaces(4).into_iter().rev().collect()),
        );

        assert_eq!(store.selected_workspace().map(|w| w.id), Some(3));
        assert_eq!(store.selected(), 1, "workspace 3 now sits at row 1");
    }

    #[test]
    fn moving_stops_at_both_ends() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::WorkspaceListLoaded(workspaces(3)));

        send(&mut store, Action::MoveWorkspaceSelection(-5));
        assert_eq!(store.selected(), 0);

        send(&mut store, Action::MoveWorkspaceSelection(99));
        assert_eq!(store.selected(), 2);
    }

    #[test]
    fn a_created_workspace_is_added_and_selected() {
        let mut store = WorkspaceStore::new();
        send(&mut store, Action::WorkspaceListLoaded(workspaces(2)));

        let created = workspace_with_id(99);
        send(&mut store, Action::WorkspaceSaved(created.clone()));

        assert_eq!(store.len(), 3);
        assert_eq!(store.selected_workspace().map(|w| w.id), Some(created.id));
    }

    #[test]
    fn task_actions_are_ignored() {
        let mut store = WorkspaceStore::new();
        assert!(send(&mut store, Action::TaskListLoaded(vec![])).is_empty());
        assert!(store.is_empty());
    }
}
