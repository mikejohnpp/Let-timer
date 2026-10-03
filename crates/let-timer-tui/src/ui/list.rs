//! The list of records: the reason the application is open.
//!
//! One function decides what a row says, and the rest is ratatui. The rows come
//! straight from whichever store the component on screen reads, so a list that
//! has not been told anything yet draws as empty rather than as a guess.
//!
//! A row's text is the record's business, not the widget's, which keeps the
//! formatting testable without a terminal: [`row_for`] turns a record into a
//! string, and the drawing tests only have to care that the right rows landed
//! on the right lines.

use let_timer_core::{MediaList, Task, Workspace};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListState},
};

use crate::action::Component;
use crate::app::App;
use crate::store::Mode;

/// Draw the records on screen into `area`.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        // A window too small for a list is not an error. It is a window
        // somebody is in the middle of resizing.
        return;
    }

    let rows = rows(app);
    if rows.is_empty() {
        frame.render_widget(empty_message(app), area);
        return;
    }

    let list = List::new(rows).highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // A fresh state every frame: the store owns which row is selected, and
    // ratatui works out for itself how much of the list has to scroll to show
    // it. Nothing here remembers what was on screen last time.
    let mut state = ListState::default().with_selected(Some(selected(app)));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Which row is highlighted.
fn selected(app: &App) -> usize {
    match app.component() {
        Component::Task => app.dispatcher().tasks().selected(),
        Component::Workspace => app.dispatcher().workspaces().selected(),
        Component::MediaList => app.dispatcher().media_lists().selected(),
    }
}

/// One line per record on screen.
fn rows(app: &App) -> Vec<Line<'static>> {
    match app.component() {
        Component::Task => app
            .dispatcher()
            .tasks()
            .tasks()
            .iter()
            .map(task_row)
            .collect(),
        Component::Workspace => app
            .dispatcher()
            .workspaces()
            .workspaces()
            .iter()
            .map(workspace_row)
            .collect(),
        Component::MediaList => app
            .dispatcher()
            .media_lists()
            .media_lists()
            .iter()
            .map(media_list_row)
            .collect(),
    }
}

/// What a list with nothing in it says.
///
/// The fullscreen hint names a key because there is room for it and somebody is
/// looking at the whole screen. Inline says only that there is nothing here: the
/// letters are not bound in inline mode, and telling somebody to press one
/// would be a lie.
fn empty_message(app: &App) -> Line<'static> {
    let text = match app.component() {
        Component::Task => "No tasks yet",
        Component::Workspace => "No workspaces yet",
        Component::MediaList => "No media lists yet",
    };

    let hint = match app.dispatcher().ui().mode() {
        Mode::Fullscreen => " -- press n to add one",
        Mode::Inline { .. } => "",
    };

    Line::from(format!("{text}{hint}"))
}

/// A task: what it is, when it is for, and how badly it wants doing.
fn task_row(task: &Task) -> Line<'static> {
    Line::from(vec![
        Span::raw(task.name.clone()),
        Span::raw("  "),
        Span::styled(
            task.priority.as_str(),
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::raw("  "),
        Span::styled(
            task.status.as_str(),
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::raw("  "),
        // A task with no day is not late, it is unplanned, and saying "late"
        // about it would be a lie the user then has to argue with.
        Span::raw(match task.scheduled_on {
            Some(date) => date.format("%Y-%m-%d").to_string(),
            None => "--".to_string(),
        }),
    ])
}

/// A workspace: the name, and its description when it has one worth the row.
fn workspace_row(workspace: &Workspace) -> Line<'static> {
    let mut spans = vec![Span::raw(workspace.name.clone())];
    if let Some(description) = workspace
        .description
        .as_ref()
        .filter(|description| !description.trim().is_empty())
    {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            description.clone(),
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    Line::from(spans)
}

/// A media list: the name, and its description when it has one worth the row.
fn media_list_row(media_list: &MediaList) -> Line<'static> {
    let mut spans = vec![Span::raw(media_list.name.clone())];
    if let Some(description) = media_list
        .description
        .as_ref()
        .filter(|description| !description.trim().is_empty())
    {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            description.clone(),
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use let_timer_core::Priority;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

    use super::*;
    use crate::action::Action;
    use crate::config::keymap::KeyMap;
    use crate::store::test_util;

    fn app(mode: Mode, component: Component) -> App {
        App::new(mode, component, KeyMap::defaults())
    }

    fn task_app() -> App {
        let mut app = app(Mode::Fullscreen, Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(3)));
        app
    }

    /// Draw the list alone and hand back the buffer it landed in.
    fn draw_list(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame, app, frame.area()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// One row of the buffer as a string.
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
    fn a_task_row_says_the_name_the_day_and_the_priority() {
        let mut app = app(Mode::Fullscreen, Component::Task);
        let mut tasks = test_util::tasks(1);
        tasks[0].name = "write the parser".to_string();
        tasks[0].priority = Priority::Urgent;
        tasks[0].status = let_timer_core::TaskStatus::InProgress;
        tasks[0].scheduled_on = Some(chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        app.react(Action::TaskListLoaded(tasks));

        let buffer = draw_list(&app, 60, 3);

        assert_eq!(
            text(&buffer).trim(),
            "write the parser  urgent  in-progress  2026-03-01"
        );
    }

    #[test]
    fn a_task_with_no_day_says_so_instead_of_leaving_a_hole() {
        let mut app = app(Mode::Fullscreen, Component::Task);
        let mut tasks = test_util::tasks(1);
        tasks[0].scheduled_on = None;
        app.react(Action::TaskListLoaded(tasks));

        let buffer = draw_list(&app, 60, 1);

        assert!(
            text(&buffer).contains("--"),
            "an unscheduled task has no date, found {:?}",
            text(&buffer)
        );
    }

    #[test]
    fn one_row_per_record_in_the_order_they_arrived() {
        let app = task_app();

        let buffer = draw_list(&app, 60, 5);
        let screen = text(&buffer);

        assert!(
            screen.contains("Task 1") && screen.contains("Task 2") && screen.contains("Task 3"),
            "found {screen:?}"
        );
        assert!(
            screen.find("Task 1") < screen.find("Task 2"),
            "the daemon's order is the user's order too, found {screen:?}"
        );
    }

    #[test]
    fn the_selected_row_is_the_one_the_store_points_at() {
        let mut app = task_app();
        app.react(Action::MoveSelection(2));

        let buffer = draw_list(&app, 60, 5);

        let selected = (0..buffer.area.height).find(|y| {
            buffer.area.width > 0 && buffer[(0, *y)].modifier.contains(Modifier::REVERSED)
        });
        assert!(
            selected.is_some_and(|y| row(&buffer, y).contains("Task 3")),
            "the third row should be the highlighted one, found {screen}",
            screen = text(&buffer)
        );
    }

    #[test]
    fn a_list_longer_than_the_screen_scrolls_to_the_selection() {
        let mut app = app(Mode::Fullscreen, Component::Task);
        app.react(Action::TaskListLoaded(test_util::tasks(50)));
        app.react(Action::MoveSelection(40));

        let buffer = draw_list(&app, 60, 5);

        assert!(
            text(&buffer).contains("Task 41"),
            "the selected row has to be visible, found {:?}",
            text(&buffer)
        );
        assert!(
            !text(&buffer).contains("Task 1"),
            "the top of a long list is not what the user asked to see"
        );
    }

    #[test]
    fn an_empty_fullscreen_list_says_how_to_fill_it() {
        let app = app(Mode::Fullscreen, Component::Task);

        let buffer = draw_list(&app, 60, 3);

        assert!(
            text(&buffer).contains("No tasks yet -- press n"),
            "found {:?}",
            text(&buffer)
        );
    }

    #[test]
    fn an_empty_inline_list_does_not_ask_for_a_key_it_refuses_to_read() {
        let app = app(Mode::Inline { max_height: 10 }, Component::Task);

        let buffer = draw_list(&app, 60, 3);

        assert!(text(&buffer).contains("No tasks yet"));
        assert!(
            !text(&buffer).contains("press n"),
            "n is not a command inline, so suggesting it is wrong"
        );
    }

    #[test]
    fn a_workspace_row_carries_its_description_when_there_is_one() {
        let mut app = app(Mode::Fullscreen, Component::Workspace);
        let mut workspaces = test_util::workspaces(2);
        workspaces[0].description = Some("deep work".to_string());
        app.react(Action::WorkspaceListLoaded(workspaces));

        let buffer = draw_list(&app, 60, 3);
        let screen = text(&buffer);

        assert!(
            screen.contains("Workspace 1  deep work"),
            "found {screen:?}"
        );
        assert_eq!(
            row(&buffer, 1).trim(),
            "Workspace 2",
            "a row with no description says only the name, found {:?}",
            row(&buffer, 1)
        );
    }

    #[test]
    fn a_media_list_row_reads_the_same_way_a_workspace_does() {
        let mut app = app(Mode::Fullscreen, Component::MediaList);
        app.react(Action::MediaListLoaded(test_util::media_lists(1)));

        let buffer = draw_list(&app, 60, 2);

        assert!(
            text(&buffer).contains("Media list 1"),
            "found {:?}",
            text(&buffer)
        );
    }

    #[test]
    fn a_window_with_no_room_draws_nothing_and_says_nothing() {
        let app = task_app();

        let mut terminal = Terminal::new(TestBackend::new(10, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, &app, Rect::new(0, 0, 10, 0)))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        assert!(
            row(&buffer, 0).trim().is_empty(),
            "found {:?}",
            row(&buffer, 0)
        );
    }
}
