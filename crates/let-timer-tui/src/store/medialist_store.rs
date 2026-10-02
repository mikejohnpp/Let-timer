//! The media list.
//!
//! Same shape as the workspace list: rows the user can move a highlight
//! through, refreshed on every tick. Media lists only support listing and
//! creating, so there is no edit or delete path here yet.

use let_timer_core::{Command, MediaList};

use crate::action::Action;
use crate::effect::Effect;
use crate::store::list_state::{Identified, ListState};
use crate::store::{EffectQueue, Store};

impl Identified for MediaList {
    fn id(&self) -> i64 {
        self.id
    }
}

/// A list of media lists plus the highlighted row.
#[derive(Debug, Default)]
pub struct MediaListStore {
    list: ListState<MediaList>,
    /// Whether a list request is still in flight, so ticks do not pile up
    /// another one behind a slow daemon.
    pending: bool,
    effects: EffectQueue,
}

impl MediaListStore {
    /// An empty media list with the first row highlighted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every media list currently loaded, in the order the daemon returned them.
    pub fn media_lists(&self) -> &[MediaList] {
        self.list.rows()
    }

    /// Index of the highlighted row.
    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    /// The highlighted media list, or `None` when the list is empty.
    pub fn selected_media_list(&self) -> Option<&MediaList> {
        self.list.selected_row()
    }

    /// How many media lists are loaded.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no media list has loaded yet.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Whether a list request is waiting for a reply.
    pub fn is_pending(&self) -> bool {
        self.pending
    }
}

impl Store for MediaListStore {
    fn update(&mut self, action: Action) {
        match action {
            Action::MediaListLoaded(media_lists) => {
                self.pending = false;
                self.list.replace(media_lists);
            }

            // Creating a media list answers with the created row.
            Action::MediaListSaved(media_list) => self.list.save(media_list),

            // Both outcomes of a list request clear `pending`: a failure that
            // left it set would stall polling for the rest of the session.
            Action::IpcFailed(_) => self.pending = false,

            Action::Tick => self.request_list(),

            Action::Select(index) => self.list.select_row(index),
            Action::MoveSelection(delta) => self.list.move_selection(delta),

            _ => {}
        }
    }

    fn take_effects(&mut self) -> Vec<Effect> {
        self.effects.drain()
    }
}

impl MediaListStore {
    /// Queue a list request unless one is already in flight.
    fn request_list(&mut self) {
        if self.pending {
            return;
        }
        self.pending = true;
        self.effects.push(Effect::Send(Command::ListMediaLists));
    }
}

#[cfg(test)]
mod tests {
    use crate::store::test_util::{media_list_with_id, media_lists};

    use super::*;

    fn send(store: &mut MediaListStore, action: Action) -> Vec<Effect> {
        store.update(action);
        store.take_effects()
    }

    #[test]
    fn starts_empty() {
        let store = MediaListStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.selected(), 0);
        assert!(store.selected_media_list().is_none());
    }

    #[test]
    fn a_loaded_list_fills_the_store() {
        let mut store = MediaListStore::new();
        send(&mut store, Action::MediaListLoaded(media_lists(3)));

        assert_eq!(store.len(), 3);
        assert_eq!(store.selected(), 0);
        assert_eq!(store.selected_media_list().map(|list| list.id), Some(1));
    }

    #[test]
    fn a_tick_asks_the_daemon_for_the_media_lists() {
        let mut store = MediaListStore::new();
        let effects = send(&mut store, Action::Tick);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Send(Command::ListMediaLists)]
        ));
    }

    #[test]
    fn a_tick_while_a_request_is_out_does_not_send_another() {
        let mut store = MediaListStore::new();
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
        let mut store = MediaListStore::new();
        send(&mut store, Action::Tick);
        assert!(store.is_pending());

        send(&mut store, Action::MediaListLoaded(media_lists(2)));
        assert!(!store.is_pending());

        assert!(matches!(
            send(&mut store, Action::Tick).as_slice(),
            [Effect::Send(Command::ListMediaLists)]
        ));
    }

    #[test]
    fn a_failure_clears_pending_so_polling_recovers() {
        let mut store = MediaListStore::new();
        send(&mut store, Action::Tick);

        send(&mut store, Action::IpcFailed("boom".to_string()));
        assert!(!store.is_pending(), "otherwise polling would stall forever");

        assert!(matches!(
            send(&mut store, Action::Tick).as_slice(),
            [Effect::Send(Command::ListMediaLists)]
        ));
    }

    #[test]
    fn the_highlight_follows_the_same_media_list_across_a_refresh() {
        let mut store = MediaListStore::new();
        send(&mut store, Action::MediaListLoaded(media_lists(4)));
        send(&mut store, Action::Select(2));

        send(
            &mut store,
            Action::MediaListLoaded(media_lists(4).into_iter().rev().collect()),
        );

        assert_eq!(store.selected_media_list().map(|list| list.id), Some(3));
        assert_eq!(store.selected(), 1, "media list 3 now sits at row 1");
    }

    #[test]
    fn moving_stops_at_both_ends() {
        let mut store = MediaListStore::new();
        send(&mut store, Action::MediaListLoaded(media_lists(3)));

        send(&mut store, Action::MoveSelection(-5));
        assert_eq!(store.selected(), 0);

        send(&mut store, Action::MoveSelection(99));
        assert_eq!(store.selected(), 2);
    }

    #[test]
    fn a_created_media_list_is_added_and_selected() {
        let mut store = MediaListStore::new();
        send(&mut store, Action::MediaListLoaded(media_lists(2)));

        let created = media_list_with_id(99);
        send(&mut store, Action::MediaListSaved(created.clone()));

        assert_eq!(store.len(), 3);
        assert_eq!(
            store.selected_media_list().map(|list| list.id),
            Some(created.id)
        );
    }

    #[test]
    fn workspace_actions_are_ignored() {
        let mut store = MediaListStore::new();
        assert!(send(&mut store, Action::WorkspaceListLoaded(vec![])).is_empty());
        assert!(store.is_empty());
    }
}
