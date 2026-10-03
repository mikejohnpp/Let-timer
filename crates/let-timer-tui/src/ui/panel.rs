//! The detail panel: everything about the selected record.
//!
//! The list says enough to pick a row; the panel says everything about it. It is
//! only drawn when there is something to show, because an empty panel is a box
//! around the word "nothing", which tells the user nothing and costs them a third
//! of the screen.

use let_timer_core::{MediaList, Task, Workspace};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::action::Component;
use crate::app::App;

/// Whether there is a selected record to describe.
pub fn has_selection(app: &App) -> bool {
    lines(app).is_some()
}

/// Draw the selected record into `area`, if there is one.
///
/// Returns whether anything was drawn, so the caller can hand the space back to
/// the list when there is nothing to put in it.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if area.height == 0 || area.width == 0 {
        return false;
    }

    let Some(lines) = lines(app) else {
        return false;
    };

    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Details");

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
    true
}

/// The selected record as lines, or nothing when no row is selected.
fn lines(app: &App) -> Option<Vec<Line<'static>>> {
    match app.component() {
        Component::Task => app.dispatcher().tasks().selected_task().map(task_lines),
        Component::Workspace => app
            .dispatcher()
            .workspaces()
            .selected_workspace()
            .map(workspace_lines),
        Component::MediaList => app
            .dispatcher()
            .media_lists()
            .selected_media_list()
            .map(media_list_lines),
    }
}

/// Every field of a task, whether or not it is set.
///
/// An unset field is printed as `--` rather than left out: a panel that shows
/// four of a task's six fields reads as though the other two do not exist.
fn task_lines(task: &Task) -> Vec<Line<'static>> {
    vec![
        field("Name", Some(task.name.clone())),
        field("Description", task.description.clone()),
        field("Priority", Some(task.priority.as_str().to_string())),
        field("Status", Some(task.status.as_str().to_string())),
        field("Minutes", task.estimated_mins.map(|mins| mins.to_string())),
        field(
            "Scheduled on",
            task.scheduled_on
                .map(|date| date.format("%Y-%m-%d").to_string()),
        ),
        field("Workspace", Some(task.workspace_id.to_string())),
        field("Media list", task.media_list_id.map(|id| id.to_string())),
    ]
}

/// Every field of a workspace.
fn workspace_lines(workspace: &Workspace) -> Vec<Line<'static>> {
    vec![
        field("Name", Some(workspace.name.clone())),
        field("Description", workspace.description.clone()),
        field("Created", Some(workspace.created_at.clone())),
    ]
}

/// Every field of a media list.
fn media_list_lines(media_list: &MediaList) -> Vec<Line<'static>> {
    vec![
        field("Name", Some(media_list.name.clone())),
        field("Description", media_list.description.clone()),
        field("Created", Some(media_list.created_at.clone())),
    ]
}

/// One labelled value. `None` is printed as `--`.
fn field(label: &'static str, value: Option<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::raw(value.unwrap_or_else(|| "--".to_string())),
    ])
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::Action;
    use crate::config::keymap::KeyMap;
    use crate::store::test_util;

    fn app(component: Component) -> App {
        App::new(component, KeyMap::defaults())
    }

    fn drawn(app: &App, width: u16, height: u16) -> (Buffer, bool) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut was_drawn = false;
        terminal
            .draw(|frame| was_drawn = draw(frame, app, frame.area()))
            .unwrap();
        (terminal.backend().buffer().clone(), was_drawn)
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    fn text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| row(buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_panel_shows_every_field_of_the_selected_task() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(2)));

        let (buffer, drawn) = drawn(&app, 40, 12);
        let screen_text = text(&buffer);

        assert!(drawn);
        assert!(screen_text.contains("Task 1"), "found {screen_text:?}");
        for expected in [
            "Name:",
            "Description:",
            "Priority:",
            "Status:",
            "Minutes:",
            "Scheduled on:",
            "Workspace:",
            "Media list:",
        ] {
            assert!(
                screen_text.contains(expected),
                "{expected} missing from {screen_text:?}"
            );
        }
    }

    #[test]
    fn the_panel_follows_the_selection() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(3)));
        app.react(Action::MoveSelection(2));

        let screen_text = text(&drawn(&app, 40, 12).0);

        assert!(screen_text.contains("Task 3"), "found {screen_text:?}");
        assert!(!screen_text.contains("Task 1"), "found {screen_text:?}");
    }

    #[test]
    fn a_field_that_is_not_set_is_shown_as_missing_rather_than_skipped() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let screen_text = text(&drawn(&app, 40, 12).0);

        assert!(
            screen_text.contains("Minutes: --"),
            "a field with no value still exists, found {screen_text:?}"
        );
        assert!(
            screen_text.contains("Description: --"),
            "found {screen_text:?}"
        );
    }

    #[test]
    fn a_panel_with_nothing_selected_draws_nothing_at_all() {
        let app = app(Component::Task);

        let (buffer, drawn) = drawn(&app, 40, 6);

        assert!(!drawn, "an empty box around nothing helps nobody");
        assert!(text(&buffer).trim().is_empty(), "found {:?}", text(&buffer));
    }

    #[test]
    fn a_workspace_and_a_media_list_get_their_own_fields() {
        let mut workspaces = app(Component::Workspace);
        workspaces.react(Action::WorkspaceListLoaded(test_util::workspaces(1)));
        let workspace_text = text(&drawn(&workspaces, 40, 8).0);
        assert!(workspace_text.contains("Workspace 1"), "{workspace_text:?}");
        assert!(workspace_text.contains("Created:"), "{workspace_text:?}");

        let mut lists = app(Component::MediaList);
        lists.react(Action::MediaListLoaded(test_util::media_lists(1)));
        let list_text = text(&drawn(&lists, 40, 8).0);
        assert!(list_text.contains("Media list 1"), "{list_text:?}");
        assert!(list_text.contains("Created:"), "{list_text:?}");
    }

    #[test]
    fn the_layout_can_ask_whether_a_panel_is_worth_the_room() {
        let mut app = app(Component::Task);
        assert!(!has_selection(&app));

        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        assert!(has_selection(&app));
    }

    #[test]
    fn a_window_too_small_for_the_panel_is_left_alone() {
        let mut app = app(Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        let mut terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
        let mut was_drawn = true;
        terminal
            .draw(|frame| was_drawn = draw(frame, &app, Rect::new(0, 0, 20, 0)))
            .unwrap();

        assert!(!was_drawn);
    }
}
