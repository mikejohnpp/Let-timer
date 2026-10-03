//! The list of workspaces down the left, for narrowing the list of tasks.
//!
//! Two things are on screen at once here and they are not the same thing: the row
//! the sidebar *has* highlighted, which is where the next keystroke goes, and the
//! row the list is *being* filtered by, which is what the user applied with
//! `enter` an hour ago. Confusing the two is how a list ends up filtered to
//! somewhere the user never chose, so the applied one is marked and the
//! highlighted one is highlighted, and neither stands in for the other.
//!
//! Every row carries the number of tasks in it, counted by the task store over
//! everything it has loaded. That is why the daemon is asked for all tasks
//! rather than one workspace's worth: a count you cannot show is not a count.

use let_timer_core::Workspace;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListState, Paragraph},
};

use crate::app::App;
use crate::store::Focus;

/// Draw the workspaces into `area`.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        // A window too small for a sidebar is not an error. It is a window
        // somebody is in the middle of resizing.
        return;
    }

    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Workspaces");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.dispatcher().workspaces().is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "No workspaces",
                Style::default().add_modifier(Modifier::DIM),
            ))),
            inner,
        );
        return;
    }

    // A fresh state every frame, for the same reason the list has one: the store
    // owns which row is highlighted.
    let mut state =
        ListState::default().with_selected(Some(app.dispatcher().workspaces().selected()));
    frame.render_stateful_widget(List::new(rows(app)), inner, &mut state);
}

/// One line per workspace: the name, and how much is in it.
fn rows(app: &App) -> Vec<Line<'static>> {
    let highlighted = app.dispatcher().workspaces().selected();
    let filtered = app.dispatcher().tasks().workspace_filter();
    let sidebar_has_focus = app.dispatcher().ui().focus() == Focus::Sidebar;

    app.dispatcher()
        .workspaces()
        .workspaces()
        .iter()
        .enumerate()
        .map(|(index, workspace)| {
            row(
                app,
                workspace,
                index,
                highlighted,
                filtered,
                sidebar_has_focus,
            )
        })
        .collect()
}

/// One workspace.
///
/// The mark says the list is being filtered by this row. The highlight says
/// where the next key press goes. With the sidebar focused the highlight is the
/// strong one, because the keys are about the sidebar; with the list focused it
/// is dimmed, because the sidebar is only being looked at.
fn row(
    app: &App,
    workspace: &Workspace,
    index: usize,
    highlighted: usize,
    filtered: Option<i64>,
    sidebar_has_focus: bool,
) -> Line<'static> {
    let filtered_here = filtered == Some(workspace.id);
    let mut spans = Vec::new();

    spans.push(Span::styled(
        if filtered_here { "▸ " } else { "  " },
        Style::default().add_modifier(Modifier::DIM),
    ));

    let name_style = if index != highlighted {
        Style::default()
    } else if sidebar_has_focus {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        // Dim rather than reversed: the list on the right owns the strong
        // highlight while the keys are going there.
        Style::default().add_modifier(Modifier::DIM)
    };
    spans.push(Span::styled(workspace.name.clone(), name_style));

    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        app.dispatcher()
            .tasks()
            .workspace_count(workspace.id)
            .to_string(),
        Style::default().add_modifier(Modifier::DIM),
    ));

    Line::from(spans)
}
#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::config::keymap::KeyMap;
    use crate::store::test_util;

    fn app() -> App {
        App::new(Component::Task, KeyMap::defaults())
    }

    /// The sidebar drawn on its own, in a window wide enough for one.
    fn draw_sidebar(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame, app, frame.area()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    fn all_rows(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| row(buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn with_workspaces() -> App {
        let mut app = app();
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(3)));
        app
    }

    fn task_in(id: i64, workspace_id: i64) -> let_timer_core::Task {
        let_timer_core::Task {
            workspace_id,
            ..test_util::task_with_id(id)
        }
    }

    #[test]
    fn every_workspace_is_listed() {
        let all = all_rows(&draw_sidebar(&with_workspaces(), 18, 6));

        assert!(all.contains("Workspace 1"), "found {all:?}");
        assert!(all.contains("Workspace 2"), "found {all:?}");
        assert!(all.contains("Workspace 3"), "found {all:?}");
    }

    #[test]
    fn the_count_beside_a_workspace_is_its_own() {
        let mut app = with_workspaces();
        app.react(Action::TaskListLoaded(vec![
            task_in(1, 1),
            task_in(2, 2),
            task_in(3, 2),
            task_in(4, 3),
        ]));

        let buffer = draw_sidebar(&app, 18, 6);

        assert!(
            row(&buffer, 1).contains("Workspace 1 1"),
            "found {:?}",
            row(&buffer, 1)
        );
        assert!(
            row(&buffer, 2).contains("Workspace 2 2"),
            "found {:?}",
            row(&buffer, 2)
        );
        assert!(
            row(&buffer, 3).contains("Workspace 3 1"),
            "found {:?}",
            row(&buffer, 3)
        );
    }

    #[test]
    fn a_workspace_with_nothing_in_it_says_zero_rather_than_nothing() {
        let mut app = with_workspaces();
        app.react(Action::TaskListLoaded(vec![task_in(1, 1)]));

        let buffer = draw_sidebar(&app, 18, 6);

        assert!(
            row(&buffer, 3).contains("Workspace 3 0"),
            "an absent count is read as a row that forgot to draw, found {:?}",
            row(&buffer, 3)
        );
    }

    #[test]
    fn the_row_the_list_is_filtered_by_is_marked() {
        let mut app = with_workspaces();
        app.react(Action::SetWorkspaceFilter(Some(2)));

        let buffer = draw_sidebar(&app, 18, 6);

        assert!(
            row(&buffer, 1).starts_with('│'),
            "the first row should be unmarkered, found {:?}",
            row(&buffer, 1)
        );
        assert!(
            row(&buffer, 2).contains("▸ Workspace 2"),
            "the applied row is the one with the mark, found {:?}",
            row(&buffer, 2)
        );
        assert!(
            !row(&buffer, 3).contains('▸'),
            "and only one row is the applied one, found {:?}",
            row(&buffer, 3)
        );
    }

    #[test]
    fn no_filter_marks_no_row() {
        let buffer = draw_sidebar(&with_workspaces(), 18, 6);

        assert!(
            !all_rows(&buffer).contains('▸'),
            "nothing is being filtered, so nothing should claim to be"
        );
    }

    #[test]
    fn the_marked_row_and_the_highlighted_row_are_allowed_to_differ() {
        let mut app = with_workspaces();
        app.react(Action::SetWorkspaceFilter(Some(1)));
        // The user walked down to workspace 2 but has not pressed enter.
        app.react(Action::MoveWorkspaceSelection(1));
        app.react(Action::FocusNextPane);

        let buffer = draw_sidebar(&app, 18, 6);

        assert!(
            row(&buffer, 1).contains("▸ Workspace 1"),
            "the mark says what the list is showing, found {:?}",
            row(&buffer, 1)
        );
        let highlighted = format!("{:?}", buffer[(3, 2)].modifier);
        assert!(
            highlighted.contains("REVERSED"),
            "and the highlight says where the next key goes, which is not the same row, found {highlighted:?}"
        );
    }

    #[test]
    fn a_filtered_row_is_marked_whether_or_not_it_is_the_one_being_looked_at() {
        let mut app = with_workspaces();
        app.react(Action::SetWorkspaceFilter(Some(1)));

        let buffer = draw_sidebar(&app, 18, 6);

        let first = format!("{:?}", buffer[(3, 1)].modifier);
        assert!(
            !first.contains("REVERSED"),
            "the keys are going to the list, so the sidebar does not shout, found {first:?}"
        );
        assert!(
            row(&buffer, 1).contains('▸'),
            "but it still says what the list is filtered by, found {:?}",
            row(&buffer, 1)
        );
    }

    #[test]
    fn no_workspaces_says_so_rather_than_drawing_an_empty_box() {
        let all = all_rows(&draw_sidebar(&app(), 18, 5));

        assert!(all.contains("No workspaces"), "found {all:?}");
    }

    #[test]
    fn a_window_with_no_room_for_the_sidebar_is_left_alone() {
        let buffer = draw_sidebar(&with_workspaces(), 0, 0);

        assert!(
            all_rows(&buffer).trim().is_empty(),
            "found {:?}",
            all_rows(&buffer)
        );
    }
}
