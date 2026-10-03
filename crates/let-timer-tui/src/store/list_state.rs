//! Rows on screen plus the highlighted one, shared by every list store.
//!
//! All three stores (tasks, workspaces, media lists) show a list the user can
//! move a highlight through and that gets swapped out wholesale on every
//! refresh. Keeping that behaviour here means the clamping rules are written
//! once instead of three times.

/// Anything a list row can be identified by, so the highlight can follow a row
/// across a refresh.
pub trait Identified {
    /// The row's stable id.
    fn id(&self) -> i64;
}

/// A list of rows and which one is highlighted.
#[derive(Debug)]
pub struct ListState<T> {
    rows: Vec<T>,
    selected: usize,
}

/// Hand-written because the derive would demand `T: Default`, and an empty
/// list is the default for every row type.
impl<T> Default for ListState<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            selected: 0,
        }
    }
}

impl<T> ListState<T> {
    /// An empty list with the first row highlighted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every row, in the order the daemon returned them.
    pub fn rows(&self) -> &[T] {
        &self.rows
    }

    /// Index of the highlighted row.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The highlighted row, or `None` when the list is empty.
    pub fn selected_row(&self) -> Option<&T> {
        self.rows.get(self.selected)
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Highlight a row by its position, clamping into range.
    ///
    /// Out-of-range targets are clamped rather than ignored: the highlight can
    /// never end up on a row that does not exist, so views can index the list
    /// with it without checking. `saturating_sub` keeps an empty list from
    /// turning the upper bound into an underflow.
    pub fn select_row(&mut self, index: usize) {
        self.selected = index.min(self.rows.len().saturating_sub(1));
    }

    /// Move the highlight by a signed number of rows, stopping at both ends.
    pub fn move_selection(&mut self, delta: i32) {
        let target = (self.selected as i32).saturating_add(delta);
        let last = (self.rows.len() as i32 - 1).max(0);
        self.selected = target.clamp(0, last) as usize;
    }

    /// Put a saved row into the list, replacing the row with the same id.
    ///
    /// Returns `true` when the row was new and got appended. Ordering follows
    /// whatever the last list load returned, so an appended row can sit in the
    /// wrong place until the next refresh corrects it; re-sorting locally would
    /// mean duplicating the daemon's ordering rule in the client.
    fn upsert(&mut self, row: T)
    where
        T: Identified,
    {
        match self
            .rows
            .iter()
            .position(|existing| existing.id() == row.id())
        {
            Some(index) => self.rows[index] = row,
            None => {
                self.rows.push(row);
                self.selected = self.rows.len() - 1;
            }
        }
    }
}

impl<T: Identified> ListState<T> {
    /// Swap in a freshly loaded list.
    ///
    /// The highlight follows the row that was selected by id, so a periodic
    /// refresh that reorders or inserts rows does not move the user's cursor.
    /// When that row is gone the highlight stays on the same row number,
    /// clamped to the new length, which keeps it as close as possible to where
    /// it was.
    pub fn replace(&mut self, rows: Vec<T>) {
        let selected_id = self.selected_row().map(Identified::id);
        self.rows = rows;
        match selected_id {
            Some(id) if self.select_id(id) => {}
            _ => self.selected = self.selected.min(self.rows.len().saturating_sub(1)),
        }
    }

    /// Put the highlight on the row with this id, if it is still there.
    ///
    /// Says whether it was found, so a caller can tell "moved there" from "was
    /// never there" and decide what to do about the difference.
    pub fn select_id(&mut self, id: i64) -> bool {
        match self.rows.iter().position(|row| row.id() == id) {
            Some(index) => {
                self.selected = index;
                true
            }
            None => false,
        }
    }

    /// Put a saved row into the list, replacing the row with the same id.
    pub fn save(&mut self, row: T) {
        self.upsert(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Row {
        id: i64,
    }

    impl Identified for Row {
        fn id(&self) -> i64 {
            self.id
        }
    }

    fn rows(ids: &[i64]) -> Vec<Row> {
        ids.iter().map(|id| Row { id: *id }).collect()
    }

    #[test]
    fn starts_empty() {
        let list = ListState::<Row>::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.selected(), 0);
        assert!(list.selected_row().is_none());
    }

    #[test]
    fn the_highlight_follows_the_same_row_across_a_refresh() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3, 4]));
        list.select_row(2);
        assert_eq!(list.selected_row().map(|row| row.id), Some(3));

        list.replace(rows(&[4, 3, 2, 1]));

        assert_eq!(list.selected_row().map(|row| row.id), Some(3));
        assert_eq!(list.selected(), 1, "row 3 now sits at row 1");
    }

    #[test]
    fn a_vanished_highlight_clamps_to_the_new_end() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3, 4, 5]));
        list.select_row(4);

        list.replace(rows(&[1, 2]));

        assert_eq!(list.selected(), 1, "clamped to the last of two rows");
        assert_eq!(list.selected_row().map(|row| row.id), Some(2));
    }

    #[test]
    fn the_first_load_lands_on_the_first_row() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3]));

        assert_eq!(
            list.selected(),
            0,
            "nothing was selected before, so the top row wins"
        );
    }

    #[test]
    fn an_emptied_list_leaves_nothing_selected() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3]));
        list.select_row(2);

        list.replace(vec![]);

        assert!(list.is_empty());
        assert_eq!(list.selected(), 0);
        assert!(list.selected_row().is_none());
    }

    #[test]
    fn moving_stops_at_both_ends() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3]));

        list.move_selection(-5);
        assert_eq!(list.selected(), 0);

        list.move_selection(99);
        assert_eq!(list.selected(), 2);
    }

    #[test]
    fn moving_within_an_empty_list_is_harmless() {
        let mut list = ListState::<Row>::new();

        list.move_selection(1);
        assert_eq!(list.selected(), 0);
        assert!(list.selected_row().is_none());

        list.move_selection(-1);
        assert_eq!(list.selected(), 0);
        assert!(list.selected_row().is_none());
    }

    #[test]
    fn saving_a_new_row_appends_and_selects_it() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2]));

        list.save(Row { id: 99 });

        assert_eq!(list.len(), 3);
        assert_eq!(list.selected_row().map(|row| row.id), Some(99));
    }

    #[test]
    fn saving_over_a_loaded_row_replaces_it_without_moving_the_highlight() {
        let mut list = ListState::new();
        list.replace(rows(&[1, 2, 3]));
        assert_eq!(list.selected(), 0);

        list.save(Row { id: 2 });

        assert_eq!(list.len(), 3, "no duplicate row");
        assert_eq!(list.rows()[1].id, 2);
        assert_eq!(
            list.selected(),
            0,
            "a replace leaves the highlight on the user's row"
        );
    }

    #[test]
    fn saving_onto_an_empty_list_selects_the_row() {
        let mut list = ListState::<Row>::new();

        list.save(Row { id: 7 });

        assert_eq!(list.len(), 1);
        assert_eq!(list.selected(), 0);
        assert_eq!(list.selected_row().map(|row| row.id), Some(7));
    }
}
