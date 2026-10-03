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
pub mod status;

use ratatui::Frame;

use crate::app::App;

/// Draw the whole application, top to bottom.
///
/// The order is the reading order, so it says what the screen is for: the frame
/// first, because it decides how much room is left, then whatever is in it. An
/// open form takes the whole body, since a list behind a form is a list nobody is
/// looking at.
pub fn draw(frame: &mut Frame, app: &App) {
    let (body, status_line) = layout::draw(frame, app);

    if app.dispatcher().form().is_open() {
        form::draw(frame, app.dispatcher().form(), body);
    } else {
        let (list_area, detail_area) = layout::split(body, panel::has_selection(app));
        list::draw(frame, app, list_area);
        panel::draw(frame, app, detail_area);
    }

    status::draw(frame, app, status_line);

    // Overlays go last, over everything: a popup that something drew
    // afterwards would not be a popup. Each one decides for itself whether it
    // is the thing on top.
    help::draw(frame, app);
    confirm::draw(frame, app);
    calendar::draw(frame, app);
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

    #[test]
    fn an_empty_screen_draws_the_message_and_not_a_crash() {
        let buffer = screen(&app(Component::Task), 40, 4);

        let all = all_rows(&buffer);

        assert!(all.contains("No tasks yet"), "found {all:?}");
    }
}
