//! The status line: the one row that says whether anything is wrong.
//!
//! Three things want saying and none of them is a task: whether the daemon is
//! answering, whether a request is still in flight, and whatever the last thing
//! that went wrong was. A user who cannot tell a slow daemon from a dead one
//! will sit there waiting, or press the same key again.
//!
//! The connection state comes first because it explains the rest: a task list
//! that is not moving looks like a bug until somebody says the daemon is gone.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;
use crate::store::Connection;

/// Draw the status line into `area`.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut spans = vec![Span::from(connection(app.dispatcher().ui().connection()))];

    if let Some(message) = app.dispatcher().ui().toast() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            message.to_string(),
            Style::default().add_modifier(Modifier::REVERSED),
        ));
    } else if let Some(waiting) = waiting(app) {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            waiting,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// What to say about the connection.
fn connection(connection: Connection) -> &'static str {
    match connection {
        Connection::Connecting => "connecting...",
        Connection::Connected => "connected",
        Connection::Disconnected => "daemon not reachable",
    }
}

/// What the list is waiting for, if anything.
///
/// A pending list with nothing on screen reads as "no tasks yet", which is a lie
/// the user would act on, so the two have to be told apart.
fn waiting(app: &App) -> Option<&'static str> {
    match app.component() {
        crate::action::Component::Task if app.dispatcher().tasks().is_pending() => {
            Some("loading tasks...")
        }
        crate::action::Component::Workspace if app.dispatcher().workspaces().is_pending() => {
            Some("loading workspaces...")
        }
        crate::action::Component::MediaList if app.dispatcher().media_lists().is_pending() => {
            Some("loading media lists...")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::config::keymap::KeyMap;
    use crate::store::{Mode, test_util};

    fn app() -> App {
        App::new(Mode::Fullscreen, Component::Task, KeyMap::defaults())
    }

    fn line(app: &App, width: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, app, frame.area()))
            .unwrap();
        let buffer: Buffer = terminal.backend().buffer().clone();
        (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect()
    }

    #[test]
    fn the_line_starts_by_saying_whether_the_daemon_is_there() {
        let app = app();

        assert!(line(&app, 40).contains("connecting"));
    }

    #[test]
    fn a_lost_daemon_says_so_plainly() {
        let mut app = app();
        app.react(Action::IpcFailed("no daemon".to_string()));

        assert!(
            line(&app, 40).contains("daemon not reachable"),
            "found {:?}",
            line(&app, 40)
        );
    }

    #[test]
    fn a_message_from_the_daemon_is_shown_next_to_the_connection() {
        let mut app = app();
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::Toast("task created".to_string()));

        let text = line(&app, 60);

        assert!(text.contains("connected"), "found {text:?}");
        assert!(text.contains("task created"), "found {text:?}");
    }

    #[test]
    fn the_most_recent_message_replaces_the_old_one() {
        let mut app = app();
        app.react(Action::Toast("first".to_string()));
        app.react(Action::Toast("second".to_string()));

        let text = line(&app, 60);

        assert!(text.contains("second"), "found {text:?}");
        assert!(!text.contains("first"), "found {text:?}");
    }

    #[test]
    fn a_list_still_loading_is_not_an_empty_list() {
        let mut app = app();
        app.react(Action::Tick);

        assert!(
            line(&app, 60).contains("loading tasks"),
            "found {:?}",
            line(&app, 60)
        );
    }

    #[test]
    fn nothing_is_claimed_to_be_loading_once_the_list_has_arrived() {
        let mut app = app();
        app.react(Action::Tick);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));

        assert!(
            !line(&app, 60).contains("loading"),
            "found {:?}",
            line(&app, 60)
        );
    }

    #[test]
    fn a_loading_list_replaces_the_waiting_note_once_a_message_arrives() {
        let mut app = app();
        app.react(Action::Tick);
        app.react(Action::Toast("workspace deleted".to_string()));

        let text = line(&app, 60);

        assert!(text.contains("workspace deleted"), "found {text:?}");
        assert!(!text.contains("loading"), "found {text:?}");
    }

    #[test]
    fn a_window_with_no_row_is_left_alone() {
        let app = app();
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, &app, Rect::new(0, 0, 20, 0)))
            .unwrap();

        let buffer: Buffer = terminal.backend().buffer().clone();
        let text: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect();
        assert!(text.trim().is_empty(), "found {text:?}");
    }
}
