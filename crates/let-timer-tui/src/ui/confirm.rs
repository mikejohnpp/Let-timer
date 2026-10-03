//! The delete dialog: the last thing standing between a key press and a lost task.
//!
//! Deleting is the only thing the application does that cannot be undone from the
//! keyboard, so it asks, and it names the task it is asking about. A dialog that
//! says "delete this?" without saying what "this" is makes the user go and look,
//! which is a chance to notice they meant the other row.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;
use crate::store::Popup;

/// Draw the delete dialog on top of the screen, if it is what is on top.
pub fn draw(frame: &mut Frame, app: &App) {
    if app.dispatcher().ui().popup() != Popup::Confirm {
        return;
    }

    let Some(task) = app.dispatcher().tasks().pending_delete() else {
        // The dialog has nothing to ask about, which should not be reachable.
        // Drawing nothing beats drawing a question about nothing.
        return;
    };

    let lines = vec![
        Line::from(Span::raw(format!("Delete {}?", task.name))),
        Line::default(),
        Line::from(Span::styled(
            "enter delete, esc keep",
            Style::default().add_modifier(Modifier::DIM),
        )),
    ];

    let area = area(frame.area(), lines.len() as u16 + 2);
    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Delete");

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Where the dialog sits: in the middle, and never bigger than the screen.
fn area(screen: Rect, height: u16) -> Rect {
    let height = height.min(screen.height);
    let width = screen.width.saturating_sub(8).max(1);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::config::keymap::KeyMap;
    use crate::store::{Mode, test_util};

    fn app() -> App {
        let mut app = App::new(Mode::Fullscreen, Component::Task, KeyMap::defaults());
        app.react(Action::TaskListLoaded(test_util::tasks(3)));
        app
    }

    fn asking(app: &mut App) -> &mut App {
        app.react(Action::ConfirmDelete(Box::new(test_util::task_with_id(2))));
        app
    }

    fn screen(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_dialog_names_the_task_it_is_asking_about() {
        let mut app = app();
        asking(&mut app);

        let all = text(&screen(&app, 50, 12));

        assert!(all.contains("Delete Task 2?"), "found {all:?}");
    }

    #[test]
    fn the_dialog_says_which_key_does_which() {
        let mut app = app();
        asking(&mut app);

        let all = text(&screen(&app, 50, 12));

        assert!(all.contains("enter delete"), "found {all:?}");
        assert!(all.contains("esc keep"), "found {all:?}");
    }

    #[test]
    fn the_list_is_not_visible_through_the_dialog() {
        let mut app = app();
        asking(&mut app);

        let all = text(&screen(&app, 50, 12));

        assert!(
            !all.contains("Task 1") && !all.contains("Task 3"),
            "found {all:?}"
        );
    }

    #[test]
    fn no_dialog_is_drawn_when_nothing_is_being_asked() {
        let app = app();

        let buffer = screen(&app, 50, 12);

        assert!(text(&buffer).trim().is_empty(), "found {:?}", text(&buffer));
    }

    #[test]
    fn a_dialog_on_a_small_screen_takes_what_it_can() {
        let mut app = app();
        asking(&mut app);

        let all = text(&screen(&app, 24, 3));

        assert!(
            all.contains("Delete") || all.contains("esc keep"),
            "even a cramped dialog has to answer, found {all:?}"
        );
    }
}
