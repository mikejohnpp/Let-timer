//! Drawing: the only part of the application that knows what things look like.
//!
//! Everything here reads state and returns nothing. No store is reached into
//! from a widget, no action is invented to make a view work, and no view keeps
//! anything between two draws -- which is what makes a wrong frame cheap to
//! find, because a redraw cannot be the thing that carried stale state in.

pub mod calendar;
pub mod confirm;
pub mod form;
pub mod help;
pub mod layout;
pub mod list;
pub mod panel;
pub mod sidebar;
pub mod status;

use ratatui::Frame;

use crate::app::App;

/// Draw the whole application, top to bottom.
///
/// The order is the reading order, so it says what the screen is for: the frame
/// first, because it decides how much room is left, then whatever is in it. An
/// open form takes the whole body, since a list behind a form is a list nobody is
/// looking at.
///
/// The sidebar only appears where it has something to say: a task list with
/// workspaces to narrow it by. A workspace list with a sidebar of workspaces
/// beside it would be the same rows twice, and a task list on a screen with no
/// workspaces has nothing for the sidebar to filter.
pub fn draw(frame: &mut Frame, app: &App) {
    let (body, status_line) = layout::draw(frame, app);

    if app.dispatcher().form().is_open() {
        form::draw(frame, app.dispatcher().form(), body);
    } else {
        let regions = layout::split(body, panel::has_selection(app), want_sidebar(app));
        sidebar::draw(frame, app, regions.sidebar);
        list::draw(frame, app, regions.list);
        panel::draw(frame, app, regions.detail);
    }

    status::draw(frame, app, status_line);

    // Overlays go last, over everything: a popup that something drew
    // afterwards would not be a popup. Each one decides for itself whether it
    // is the thing on top.
    help::draw(frame, app);
    confirm::draw(frame, app);
    calendar::draw(frame, app);
}

/// Whether there is anything for a sidebar to say.
///
/// The width on screen is the layout's business, not this one's: it decides
/// whether there is room once somebody has said there is something to put there.
fn want_sidebar(app: &App) -> bool {
    app.component() == crate::action::Component::Task && !app.dispatcher().workspaces().is_empty()
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::config::keymap::KeyMap;
    use crate::store::test_util;

    fn screen(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    /// Every row of the buffer as one string, for looking at with human eyes.
    fn all_rows(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| row(buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn app(component: Component) -> App {
        App::new(component, KeyMap::defaults())
    }

    #[test]
    fn the_list_lands_inside_the_frame_rather_than_over_its_border() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let buffer = screen(&app, 30, 5);

        assert!(row(&buffer, 0).contains('╭'), "found {:?}", row(&buffer, 0));
        // The border takes the first and last rows; the list starts just
        // inside the top one and never writes over either.
        assert!(
            row(&buffer, 1).contains("Task 1"),
            "the first row belongs inside the frame, found {:?}",
            row(&buffer, 1)
        );
        assert!(
            !row(&buffer, 0).contains("Task 1") && !row(&buffer, 4).contains("Task 1"),
            "a row written over the border is a row nobody can read"
        );
    }

    #[test]
    fn a_form_takes_over_the_body_from_the_list() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::OpenCreate(Component::Task));
        app.react(Action::FormInput('x'));

        let all = all_rows(&screen(&app, 40, 10));

        assert!(all.contains("New task"), "found {all:?}");
        assert!(
            !all.contains("Task 1"),
            "the list has no business on screen while a form is open, found {all:?}"
        );
    }

    #[test]
    fn the_list_and_the_panel_share_the_screen_when_something_is_selected() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(2)));

        let all = all_rows(&screen(&app, 40, 14));

        assert!(all.contains("Task 1"), "found {all:?}");
        assert!(all.contains("Priority:"), "found {all:?}");
        assert!(
            all.find("Task 1") < all.find("Priority:"),
            "the list is above the panel, found {all:?}"
        );
    }

    #[test]
    fn the_frame_and_the_status_line_are_drawn_in_fullscreen() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(2)));

        let all = all_rows(&screen(&app, 40, 8));

        assert!(all.contains("Task 1"), "found {all:?}");
        assert!(
            all.contains("connected"),
            "the status line is part of the screen the application owns, found {all:?}"
        );
    }

    #[test]
    fn fullscreen_says_whether_the_daemon_is_answering() {
        let app = app(Component::Task);

        let all = all_rows(&screen(&app, 40, 10));

        assert!(
            all.contains("connecting"),
            "the status line is where a user looks when nothing is happening, found {all:?}"
        );
    }

    // ── the sidebar ────────────────────────────────────────────────────

    #[test]
    fn the_sidebar_takes_the_room_it_asked_for() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let all = all_rows(&screen(&app, 80, 10));

        assert!(all.contains("Workspace 1"), "found {all:?}");
        assert!(
            all.contains("Tasks"),
            "the list is still here beside it, found {all:?}"
        );
    }

    #[test]
    fn the_sidebar_is_drawn_as_a_box_of_its_own() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let all = all_rows(&screen(&app, 80, 10));

        // The frame, the sidebar, and the panel. The sidebar is a box of its
        // own rather than a column of text drawn over the list.
        assert_eq!(
            all.matches('╭').count(),
            3,
            "the sidebar should be a second box inside the frame, found {all:?}"
        );
    }

    #[test]
    fn the_sidebar_highlights_strongly_only_while_it_has_the_keys() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));

        let unfocused = screen(&app, 80, 10);
        let dim_when_list_has_keys = style_of_text(&unfocused, "Workspace 1");

        app.react(Action::FocusNextPane);
        let focused = screen(&app, 80, 10);
        let reversed_when_sidebar_has_keys = style_of_text(&focused, "Workspace 1");

        assert!(
            !dim_when_list_has_keys.contains("REVERSED"),
            "the list owns the strong highlight while the keys are going there: {dim_when_list_has_keys:?}"
        );
        assert!(
            reversed_when_sidebar_has_keys.contains("REVERSED"),
            "and the sidebar takes it over while the keys are going there: {reversed_when_sidebar_has_keys:?}"
        );
    }

    #[test]
    fn a_narrow_window_does_not_get_a_sidebar() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let all = all_rows(&screen(&app, 50, 10));

        assert!(
            !all.contains("Workspace 1"),
            "two lists of nine columns is not a choice between anything, found {all:?}"
        );
        assert!(
            all.contains("Task 1"),
            "the list is what the user came for, found {all:?}"
        );
    }

    #[test]
    fn a_workspace_list_does_not_get_a_sidebar_of_the_same_workspaces() {
        let mut app = app(Component::Workspace);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));

        let all = all_rows(&screen(&app, 80, 10));

        assert!(all.contains("Workspace 1"), "found {all:?}");
        // The frame and the detail panel, and nothing between them. A sidebar
        // here would be a third border and the same rows again.
        assert_eq!(
            all.matches('╭').count(),
            2,
            "the same rows twice is not a sidebar, found {all:?}"
        );
    }

    #[test]
    fn a_task_list_with_no_workspaces_does_not_get_an_empty_sidebar() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let all = all_rows(&screen(&app, 80, 10));

        assert!(
            !all.contains("No workspaces"),
            "a sidebar with nothing in it is a box around the word nothing, found {all:?}"
        );
    }

    #[test]
    fn each_workspace_says_how_many_tasks_are_in_it() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(vec![
            task_in(1, 1),
            task_in(1, 2),
            task_in(2, 1),
        ]));

        let all = all_rows(&screen(&app, 80, 10));

        assert!(all.contains("Workspace 1"), "found {all:?}");
        assert!(all.contains("Workspace 2"), "found {all:?}");
        // Two of the three tasks are in workspace 1.
        assert!(all.contains('2'), "found {all:?}");
    }

    #[test]
    fn the_applied_filter_is_visible_without_looking_at_the_sidebar() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::SetWorkspaceFilter(Some(1)));

        let all = all_rows(&screen(&app, 80, 10));

        assert!(
            all.contains("filter: Workspace 1"),
            "a list that quietly emptied looks like a list with nothing in it, found {all:?}"
        );
    }

    #[test]
    fn a_filter_that_names_a_workspace_is_not_hidden_by_the_sidebar_being_absent() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::SetWorkspaceFilter(Some(1)));

        let all = all_rows(&screen(&app, 50, 10));

        assert!(
            all.contains("filter: Workspace 1"),
            "the mark on the row is not on screen here, so the status line is the only place left to say it: found {all:?}"
        );
    }

    #[test]
    fn the_panel_describes_the_sidebar_rather_than_the_task_behind_it() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::FocusNextPane);

        let all = all_rows(&screen(&app, 80, 14));

        assert!(
            all.contains("Name: Workspace 1"),
            "the sidebar has the highlight, so the panel is about it, found {all:?}"
        );
        assert!(
            !all.contains("Priority:"),
            "a panel answering a question nobody asked, found {all:?}"
        );
    }

    #[test]
    fn the_panel_goes_back_to_the_task_when_the_focus_does() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::FocusNextPane);
        app.react(Action::FocusNextPane);

        let all = all_rows(&screen(&app, 80, 14));

        assert!(all.contains("Priority:"), "found {all:?}");
    }

    #[test]
    fn a_form_takes_over_the_body_from_the_sidebar_too() {
        let mut app = app(Component::Task);
        app.react(Action::WorkspaceListLoaded(test_util::workspaces(2)));
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::OpenCreate(Component::Task));

        let all = all_rows(&screen(&app, 80, 12));

        assert!(all.contains("New task"), "found {all:?}");
        assert!(
            !all.contains("Workspace 1"),
            "a sidebar behind a form is a sidebar nobody is looking at, found {all:?}"
        );
    }

    /// The style of the first character of some text on screen.
    ///
    /// Spelled out for a failure message: a test about a highlight should say
    /// which highlight it saw rather than that something was different.
    fn style_of_text(buffer: &Buffer, needle: &str) -> String {
        for y in 0..buffer.area.height {
            let line = row(buffer, y);
            if let Some(index) = line.find(needle) {
                return format!("{:?}", buffer[(index as u16, y)].modifier);
            }
        }
        panic!("{needle:?} is not on screen");
    }

    /// A task belonging to the given workspace.
    fn task_in(id: i64, workspace_id: i64) -> let_timer_core::Task {
        let_timer_core::Task {
            workspace_id,
            ..test_util::task_with_id(id)
        }
    }

    #[test]
    fn an_empty_screen_draws_the_message_and_not_a_crash() {
        let buffer = screen(&app(Component::Task), 40, 4);

        let all = all_rows(&buffer);

        assert!(all.contains("No tasks yet"), "found {all:?}");
    }
}
