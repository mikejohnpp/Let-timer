//! The task list: the rows on screen and which one is highlighted.

use std::collections::HashMap;

use let_timer_core::{Command, SortOrder, Task, TaskStatus};

use crate::action::Action;
use crate::effect::Effect;
use crate::store::list_state::{Identified, ListState};
use crate::store::{EffectQueue, Store};

impl Identified for Task {
    fn id(&self) -> i64 {
        self.id
    }
}

/// A list of tasks plus the highlighted row.
///
/// The tasks the daemon sent and the tasks on screen are kept apart. The
/// daemon is asked for the whole list on every refresh, because the sidebar
/// needs every workspace's tasks to count them; narrowing that answer down to
/// one workspace happens here rather than in the query, so switching workspaces
/// costs nothing and switching back does not have to wait for the daemon.
#[derive(Debug, Default)]
pub struct TaskStore {
    /// Every task loaded, in the order the daemon returned them.
    raw: Vec<Task>,
    /// The rows on screen: `raw` narrowed to the workspace filter.
    list: ListState<Task>,
    /// The workspace to show, or every workspace when there is no filter.
    workspace: Option<i64>,
    /// How many tasks each workspace holds, counted over `raw`.
    ///
    /// The sidebar shows these next to the workspace names, and counting here
    /// means it never has to be handed the tasks themselves.
    counts: HashMap<i64, usize>,
    sort_priority: Option<SortOrder>,
    filter_status: Option<TaskStatus>,
    /// Whether a list request is still in flight, so ticks do not pile up
    /// another one behind a slow daemon.
    pending: bool,
    /// The task a delete dialog is asking about.
    ///
    /// Held here rather than in the UI store because this is the store that
    /// knows how to delete, and because the dialog is about a record rather
    /// than about the screen.
    pending_delete: Option<Task>,
    effects: EffectQueue,
}

impl TaskStore {
    /// An empty task list with the first row highlighted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The task a delete dialog is asking about, if one is open.
    pub fn pending_delete(&self) -> Option<&Task> {
        self.pending_delete.as_ref()
    }

    /// Ask for tasks sorted and filtered a particular way.
    ///
    /// Every periodic refresh repeats this query, which is how `list` with
    /// flags keeps showing the same slice of tasks.
    pub fn set_query(
        &mut self,
        sort_priority: Option<SortOrder>,
        filter_status: Option<TaskStatus>,
    ) {
        self.sort_priority = sort_priority;
        self.filter_status = filter_status;
    }

    /// Whether a list request is waiting for a reply.
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// The tasks on screen, in the order the daemon returned them.
    ///
    /// This is `raw` narrowed to the workspace filter, so it is what the list
    /// draws and what a selected row means.
    pub fn tasks(&self) -> &[Task] {
        self.list.rows()
    }

    /// Every task loaded, whatever the filter is doing.
    pub fn all_tasks(&self) -> &[Task] {
        &self.raw
    }

    /// The workspace the list is narrowed to, or `None` for every workspace.
    pub fn workspace_filter(&self) -> Option<i64> {
        self.workspace
    }

    /// How many tasks a workspace holds, counted over everything loaded.
    ///
    /// A workspace with nothing in it is `0` rather than absent, so the sidebar
    /// can print a count beside every row it draws.
    pub fn workspace_count(&self, workspace_id: i64) -> usize {
        self.counts.get(&workspace_id).copied().unwrap_or(0)
    }

    /// Index of the highlighted row. Always valid once anything is loaded,
    /// and `0` on an empty list.
    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    /// The highlighted task, or `None` when the list is empty.
    pub fn selected_task(&self) -> Option<&Task> {
        self.list.selected_row()
    }

    /// How many tasks are on screen.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no task is on screen.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl Store for TaskStore {
    fn update(&mut self, action: Action) {
        match action {
            Action::TaskListLoaded(tasks) => {
                self.pending = false;
                self.raw = tasks;
                self.recount();
                self.refilter();
            }
            Action::TaskSaved(task) => {
                // Saved straight into `raw` rather than through the list, or
                // the row would be counted into the filter twice.
                let saved_id = task.id;
                let was_new = !self.raw.iter().any(|row| row.id == saved_id);
                match self.raw.iter_mut().find(|row| row.id == saved_id) {
                    Some(existing) => *existing = task,
                    None => self.raw.push(task),
                }
                self.recount();
                self.refilter();
                // A task that was just created becomes the highlighted row, so
                // closing the form shows the row it made. Saving over a task
                // that was already there does not move the highlight off the
                // row the user was on, and a task saved into another workspace
                // is filtered out of sight with no row to highlight at all.
                if was_new {
                    self.list.select_id(saved_id);
                }
            }

            // The sidebar's filter is applied here and nowhere else: the task
            // list is the only thing it changes.
            Action::SetWorkspaceFilter(workspace) => {
                if self.workspace != workspace {
                    self.workspace = workspace;
                    self.refilter();
                }
            }

            // Both outcomes of a list request clear `pending`: a failure that
            // left it set would stall polling for the rest of the session.
            Action::IpcFailed(_) => self.pending = false,

            // Each tick refreshes the list, but only when nothing is already
            // on its way.
            Action::Tick => self.request_list(),

            Action::ConfirmDelete(task) => self.pending_delete = Some(*task),

            // The answer to the delete dialog. Submitting takes the task out of
            // the store's hands and sends it to the daemon; anything else leaves
            // the list exactly as it was, since nothing has been sent yet.
            Action::Submit => {
                if let Some(task) = self.pending_delete.take() {
                    self.effects
                        .push(Effect::Send(Command::Delete { id: task.id }));
                }
            }

            // The question was declined, or the overlay was closed by something
            // else. Either way the task is put back.
            Action::Cancel => self.pending_delete = None,

            Action::Select(index) => self.list.select_row(index),
            Action::MoveSelection(delta) => self.list.move_selection(delta),
            _ => {}
        }
    }

    fn take_effects(&mut self) -> Vec<Effect> {
        self.effects.drain()
    }
}

impl TaskStore {
    /// Narrow `raw` down to the workspace filter and show that.
    ///
    /// Going through `replace` rather than assigning keeps the highlight on the
    /// same task when it survives the filter, which is what makes narrowing a
    /// list feel like looking rather than like starting over.
    fn refilter(&mut self) {
        let rows = match self.workspace {
            Some(workspace_id) => self
                .raw
                .iter()
                .filter(|task| task.workspace_id == workspace_id)
                .cloned()
                .collect(),
            None => self.raw.clone(),
        };
        self.list.replace(rows);
    }

    /// Count the tasks in each workspace over everything loaded.
    fn recount(&mut self) {
        self.counts.clear();
        for task in &self.raw {
            *self.counts.entry(task.workspace_id).or_default() += 1;
        }
    }

    /// Queue a list request unless one is already in flight.
    fn request_list(&mut self) {
        if self.pending {
            return;
        }
        self.pending = true;
        self.effects.push(Effect::Send(Command::List {
            sort_priority: self.sort_priority,
            filter_status: self.filter_status,
        }));
    }
}

#[cfg(test)]
mod tests {
    use crate::store::test_util::{task_with_id, tasks};

    use super::*;
    use let_timer_core::Task;

    #[test]
    fn a_delete_dialog_asks_about_the_task_it_was_given() {
        let mut store = TaskStore::new();

        store.update(Action::ConfirmDelete(Box::new(task_with_id(7))));

        assert_eq!(store.pending_delete().map(|task| task.id), Some(7));
    }

    #[test]
    fn confirming_a_delete_asks_the_daemon_to_delete_that_task() {
        let mut store = TaskStore::new();
        store.update(Action::ConfirmDelete(Box::new(task_with_id(7))));

        let mut effects = store.take_effects();
        store.update(Action::Submit);
        effects.extend(store.take_effects());

        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(Command::Delete { id: 7 }))),
            "found {effects:?}"
        );
        assert!(
            store.pending_delete().is_none(),
            "the question has been answered and cannot be answered twice"
        );
    }

    #[test]
    fn declining_a_delete_sends_nothing_at_all() {
        let mut store = TaskStore::new();
        store.update(Action::ConfirmDelete(Box::new(task_with_id(7))));

        store.update(Action::Cancel);
        let effects = store.take_effects();

        assert!(effects.is_empty(), "found {effects:?}");
        assert!(store.pending_delete().is_none());
    }

    #[test]
    fn a_submit_with_nothing_pending_deletes_nothing() {
        let mut store = TaskStore::new();

        store.update(Action::Submit);

        assert!(store.take_effects().is_empty());
    }

    #[test]
    fn a_refresh_while_the_question_is_open_does_not_answer_it() {
        let mut store = TaskStore::new();
        store.update(Action::ConfirmDelete(Box::new(task_with_id(7))));

        store.update(Action::Tick);
        store.update(Action::TaskListLoaded(tasks(2)));

        assert_eq!(
            store.pending_delete().map(|task| task.id),
            Some(7),
            "a list arriving from the daemon is not an answer"
        );
    }

    #[test]
    fn starts_empty() {
        let store = TaskStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.tasks().is_empty());
    }

    #[test]
    fn an_empty_list_has_nothing_selected() {
        let store = TaskStore::new();
        assert_eq!(store.selected(), 0);
        assert!(store.selected_task().is_none());
    }

    #[test]
    fn produces_no_effects_until_something_is_asked_for() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));
        store.update(Action::Select(1));
        assert!(store.take_effects().is_empty());
    }

    #[test]
    fn select_moves_to_an_absolute_row() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));

        store.update(Action::Select(2));
        assert_eq!(store.selected_task().map(|task| task.id), Some(3));

        store.update(Action::Select(0));
        assert_eq!(store.selected_task().map(|task| task.id), Some(1));
    }

    #[test]
    fn select_past_the_end_lands_on_the_last_row() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));

        store.update(Action::Select(99));
        assert_eq!(store.selected(), 2);
        assert_eq!(store.selected_task().map(|task| task.id), Some(3));
    }

    #[test]
    fn move_selection_steps_relative_to_the_current_row() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(4)));

        store.update(Action::MoveSelection(1));
        assert_eq!(store.selected(), 1);

        store.update(Action::MoveSelection(2));
        assert_eq!(store.selected(), 3);

        store.update(Action::MoveSelection(-1));
        assert_eq!(store.selected(), 2);
    }

    #[test]
    fn move_selection_stops_at_both_ends() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));

        store.update(Action::MoveSelection(-5));
        assert_eq!(store.selected(), 0, "cannot go above the first row");

        store.update(Action::MoveSelection(99));
        assert_eq!(store.selected(), 2, "cannot go below the last row");
    }

    #[test]
    fn moving_within_an_empty_list_is_harmless() {
        let mut store = TaskStore::new();

        store.update(Action::MoveSelection(1));
        assert_eq!(store.selected(), 0);
        assert!(store.selected_task().is_none());

        store.update(Action::MoveSelection(-1));
        assert_eq!(store.selected(), 0);
        assert!(store.selected_task().is_none());
    }

    #[test]
    fn a_tick_asks_the_daemon_for_the_list() {
        let mut store = TaskStore::new();
        store.update(Action::Tick);

        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List {
                sort_priority: None,
                filter_status: None
            })]
        ));
    }

    #[test]
    fn the_refresh_repeats_the_configured_query() {
        let mut store = TaskStore::new();
        store.set_query(Some(SortOrder::Descending), Some(TaskStatus::InProgress));

        store.update(Action::Tick);

        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List {
                sort_priority: Some(SortOrder::Descending),
                filter_status: Some(TaskStatus::InProgress)
            })]
        ));
    }

    #[test]
    fn a_tick_while_a_request_is_out_does_not_send_another() {
        let mut store = TaskStore::new();
        store.update(Action::Tick);
        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List { .. })],
        ));
        assert!(store.is_pending());

        store.update(Action::Tick);
        store.update(Action::Tick);
        assert!(
            store.take_effects().is_empty(),
            "a slow daemon must not get one request per tick"
        );
    }

    #[test]
    fn a_reply_lets_the_next_tick_refresh_again() {
        let mut store = TaskStore::new();
        store.update(Action::Tick);
        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List { .. })]
        ));
        assert!(store.is_pending());

        store.update(Action::TaskListLoaded(tasks(2)));
        assert!(!store.is_pending());

        store.update(Action::Tick);
        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List { .. })]
        ));
    }

    #[test]
    fn a_failure_also_clears_pending_so_polling_recovers() {
        let mut store = TaskStore::new();
        store.update(Action::Tick);
        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List { .. })]
        ));

        store.update(Action::IpcFailed("boom".to_string()));
        assert!(!store.is_pending(), "otherwise polling would stall forever");

        store.update(Action::Tick);
        assert!(matches!(
            store.take_effects().as_slice(),
            [Effect::Send(Command::List { .. })]
        ));
    }

    #[test]
    fn a_loaded_list_fills_the_store() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));

        assert_eq!(store.len(), 3);
        assert_eq!(store.selected(), 0);
        assert_eq!(store.selected_task().map(|task| task.id), Some(1));
    }

    #[test]
    fn loading_again_replaces_rather_than_appends() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));
        store.update(Action::TaskListLoaded(tasks(5)));

        assert_eq!(store.len(), 5);
        assert_eq!(store.tasks().last().map(|task| task.id), Some(5));
    }

    #[test]
    fn the_highlight_follows_the_same_task_across_a_refresh() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(4)));
        store.update(Action::Select(2));
        assert_eq!(store.selected_task().map(|task| task.id), Some(3));

        // The daemon returns a different order after the refresh.
        store.update(Action::TaskListLoaded(tasks(4).into_iter().rev().collect()));

        assert_eq!(store.selected_task().map(|task| task.id), Some(3));
        assert_eq!(store.selected(), 1, "task 3 now sits at row 1");
    }

    #[test]
    fn a_vanished_selection_clamps_to_the_new_end() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(5)));
        store.update(Action::Select(4));
        assert_eq!(store.selected(), 4);

        store.update(Action::TaskListLoaded(tasks(2)));

        assert_eq!(store.selected(), 1, "clamped to the last of two rows");
        assert_eq!(store.selected_task().map(|task| task.id), Some(2));
    }

    #[test]
    fn an_emptied_list_leaves_nothing_selected() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));
        store.update(Action::Select(2));

        store.update(Action::TaskListLoaded(vec![]));

        assert!(store.is_empty());
        assert_eq!(store.selected(), 0);
        assert!(store.selected_task().is_none());
    }

    // ── the workspace filter ───────────────────────────────────────────

    /// Three tasks: one in workspace 1, two in workspace 2.
    fn tasks_in_two_workspaces() -> Vec<Task> {
        vec![
            Task {
                workspace_id: 1,
                ..task_with_id(1)
            },
            Task {
                workspace_id: 2,
                ..task_with_id(2)
            },
            Task {
                workspace_id: 2,
                ..task_with_id(3)
            },
        ]
    }

    fn loaded() -> TaskStore {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks_in_two_workspaces()));
        store
    }

    #[test]
    fn no_filter_shows_everything() {
        let store = loaded();

        assert_eq!(store.workspace_filter(), None);
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn a_filter_narrows_the_list_to_one_workspace() {
        let mut store = loaded();

        store.update(Action::SetWorkspaceFilter(Some(2)));

        assert_eq!(store.len(), 2);
        assert!(
            store.tasks().iter().all(|task| task.workspace_id == 2),
            "found {:?}",
            store.tasks()
        );
    }

    #[test]
    fn the_filter_keeps_the_tasks_it_hid() {
        let mut store = loaded();

        store.update(Action::SetWorkspaceFilter(Some(1)));

        assert_eq!(store.len(), 1);
        assert_eq!(store.all_tasks().len(), 3, "hiding is not throwing away");
    }

    #[test]
    fn dropping_the_filter_brings_the_rest_back_without_the_daemon() {
        let mut store = loaded();
        store.update(Action::SetWorkspaceFilter(Some(2)));

        store.update(Action::SetWorkspaceFilter(None));

        assert_eq!(store.len(), 3);
        assert!(
            store.take_effects().is_empty(),
            "the tasks were already here; asking again would be asking for nothing"
        );
    }

    #[test]
    fn a_filter_over_a_workspace_with_nothing_in_it_is_empty_rather_than_everything() {
        let mut store = loaded();

        store.update(Action::SetWorkspaceFilter(Some(99)));

        assert_eq!(
            store.len(),
            0,
            "a filter that matches nothing should show nothing, not fall back"
        );
    }

    #[test]
    fn the_highlight_stays_on_the_same_task_when_the_filter_moves() {
        let mut store = loaded();
        store.update(Action::Select(2));

        store.update(Action::SetWorkspaceFilter(Some(2)));

        assert_eq!(
            store.selected(),
            1,
            "task 3 is the second row of workspace 2"
        );
    }

    #[test]
    fn a_refresh_applies_the_filter_that_is_already_set() {
        let mut store = loaded();
        store.update(Action::SetWorkspaceFilter(Some(2)));

        store.update(Action::TaskListLoaded(tasks_in_two_workspaces()));

        assert_eq!(
            store.len(),
            2,
            "the daemon keeps sending every workspace; the filter still holds"
        );
    }

    #[test]
    fn a_refresh_moves_the_highlight_when_the_task_it_was_on_is_filtered_away() {
        let mut store = loaded();
        store.update(Action::Select(0));

        store.update(Action::SetWorkspaceFilter(Some(2)));

        assert_eq!(store.len(), 2);
        assert!(
            store.selected() < store.len(),
            "the highlight must not point past the end of the shorter list"
        );
    }

    #[test]
    fn each_workspace_is_counted_over_everything_loaded() {
        let store = loaded();

        assert_eq!(store.workspace_count(1), 1);
        assert_eq!(store.workspace_count(2), 2);
    }

    #[test]
    fn a_workspace_with_no_tasks_counts_zero_rather_than_nothing() {
        let store = loaded();

        assert_eq!(
            store.workspace_count(99),
            0,
            "the sidebar prints a count beside every row, so absent has to be zero"
        );
    }

    #[test]
    fn the_counts_follow_a_task_that_moves_workspace() {
        let mut store = loaded();
        let moved = Task {
            workspace_id: 2,
            ..task_with_id(1)
        };

        store.update(Action::TaskSaved(moved));

        assert_eq!(store.workspace_count(1), 0);
        assert_eq!(store.workspace_count(2), 3);
    }

    #[test]
    fn a_task_saved_into_the_filtered_workspace_appears_by_itself() {
        let mut store = loaded();
        store.update(Action::SetWorkspaceFilter(Some(2)));

        store.update(Action::TaskSaved(Task {
            workspace_id: 2,
            ..task_with_id(9)
        }));

        assert_eq!(store.len(), 3);
        assert_eq!(
            store.selected_task().map(|task| task.id),
            Some(9),
            "and it is the row the user just made"
        );
    }

    #[test]
    fn a_task_saved_into_another_workspace_stays_out_of_sight() {
        let mut store = loaded();
        store.update(Action::SetWorkspaceFilter(Some(1)));

        store.update(Action::TaskSaved(Task {
            workspace_id: 2,
            ..task_with_id(9)
        }));

        assert_eq!(store.len(), 1, "the filter is not a suggestion");
        assert_eq!(store.all_tasks().len(), 4, "but the task is still loaded");
    }

    #[test]
    fn setting_the_filter_it_already_has_changes_nothing() {
        let mut store = loaded();
        store.update(Action::Select(1));

        store.update(Action::SetWorkspaceFilter(None));

        assert_eq!(
            store.selected(),
            1,
            "re-applying the filter would put the highlight back at the top"
        );
    }

    #[test]
    fn a_saved_task_is_added_and_becomes_the_selection() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(2)));
        store.update(Action::TaskSaved(task_with_id(99)));

        assert_eq!(store.len(), 3);
        assert_eq!(store.selected_task().map(|task| task.id), Some(99));
    }

    #[test]
    fn saving_over_a_loaded_task_replaces_it_in_place() {
        let mut store = TaskStore::new();
        store.update(Action::TaskListLoaded(tasks(3)));
        assert_eq!(store.selected(), 0);

        let mut edited = task_with_id(2);
        edited.name = "Renamed".to_string();
        store.update(Action::TaskSaved(edited));

        assert_eq!(store.len(), 3, "no duplicate row");
        assert_eq!(store.tasks()[1].id, 2);
        assert_eq!(store.tasks()[1].name, "Renamed");
        assert_eq!(
            store.selected(),
            0,
            "a replace does not move the highlight off the user's row"
        );
    }

    #[test]
    fn saving_a_task_onto_an_empty_list_selects_it() {
        let mut store = TaskStore::new();
        store.update(Action::TaskSaved(task_with_id(7)));

        assert_eq!(store.len(), 1);
        assert_eq!(store.selected(), 0);
        assert_eq!(store.selected_task().map(|task| task.id), Some(7));
    }
}
