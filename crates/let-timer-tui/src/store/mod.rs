//! The stores: the only place application state lives.
//!
//! A store is a plain state machine. It receives every [`Action`] the
//! dispatcher routes to it, ignores the ones it does not care about, mutates
//! its own fields, and pushes an [`Effect`] when it needs the outside world to
//! do something. Stores never touch the terminal, the IPC client or the
//! filesystem, which is what lets them be tested without any of those.

pub mod form_store;
pub mod list_state;
pub mod medialist_store;
pub mod task_store;
pub mod ui_store;
pub mod workspace_store;

use crate::action::Action;
use crate::effect::Effect;

pub use form_store::{Draft, Field, FieldKind, FormKind, FormStore};
pub use list_state::{Identified, ListState};
pub use medialist_store::MediaListStore;
pub use task_store::TaskStore;
pub use ui_store::{Connection, Mode, Popup, UiStore};
pub use workspace_store::WorkspaceStore;

/// A state machine fed by actions.
pub trait Store {
    /// Apply one action. Actions this store does not handle are ignored.
    fn update(&mut self, action: Action);

    /// Take the effects queued by the last `update`, leaving the queue empty.
    fn take_effects(&mut self) -> Vec<Effect> {
        Vec::new()
    }
}

/// Shared buffer for effects, so each store only has to implement `update`.
#[derive(Debug, Default)]
pub(crate) struct EffectQueue {
    effects: Vec<Effect>,
}

impl EffectQueue {
    /// Queue one effect for the event loop to carry out.
    pub(crate) fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Hand over everything queued so far and reset.
    pub(crate) fn drain(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }
}

/// Fixtures shared by the store tests.
#[cfg(test)]
pub(crate) mod test_util {
    use let_timer_core::{MediaList, Priority, Task, TaskStatus, Workspace};

    /// A task with the given id, for building lists in tests.
    pub(crate) fn task_with_id(id: i64) -> Task {
        Task {
            id,
            workspace_id: 1,
            media_list_id: None,
            name: format!("Task {id}"),
            description: None,
            priority: Priority::NotYet,
            status: TaskStatus::Pending,
            estimated_mins: None,
            scheduled_on: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    /// A task list of `count` rows, ids 1..=count.
    pub(crate) fn tasks(count: i64) -> Vec<Task> {
        (1..=count).map(task_with_id).collect()
    }

    /// A workspace with the given id, for building lists in tests.
    pub(crate) fn workspace_with_id(id: i64) -> Workspace {
        Workspace {
            id,
            name: format!("Workspace {id}"),
            description: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    /// A workspace list of `count` rows, ids 1..=count.
    pub(crate) fn workspaces(count: i64) -> Vec<Workspace> {
        (1..=count).map(workspace_with_id).collect()
    }

    /// A media list with the given id, for building lists in tests.
    pub(crate) fn media_list_with_id(id: i64) -> MediaList {
        MediaList {
            id,
            name: format!("Media list {id}"),
            description: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    /// A media list of `count` rows, ids 1..=count.
    pub(crate) fn media_lists(count: i64) -> Vec<MediaList> {
        (1..=count).map(media_list_with_id).collect()
    }
}
