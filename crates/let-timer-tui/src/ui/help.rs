//! The help popup: the keys that are in force, as they are in force.
//!
//! The list is read out of the keymap rather than written down here, because a
//! help screen that lies is worse than no help screen: a user reads it once,
//! presses a key it promised, and stops trusting everything else on screen.
//! Somebody who rebinds a key in `keymap.toml` gets a help screen that has
//! changed with it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::app::App;
use crate::store::Popup;

/// Draw the help popup on top of the screen, if it is the thing on top.
pub fn draw(frame: &mut Frame, app: &App) {
    if !is_open(app) {
        return;
    }

    let lines = help_lines(app);
    let Some(lines) = lines else {
        return;
    };

    let area = popup_area(frame.area(), lines.len() as u16 + 2);
    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Keys");

    // Clear first: an overlay that only draws its own text leaves the list
    // showing through the gaps between the words.
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// One line per binding, or nothing when there are no bindings to show.
fn help_lines(app: &App) -> Option<Vec<Line<'static>>> {
    let bindings = app.keymap().bindings(app.context());
    if bindings.is_empty() {
        return None;
    }

    let width = bindings
        .iter()
        .map(|binding| binding.keys_as_string().chars().count())
        .max()
        .unwrap_or(0);

    Some(
        bindings
            .iter()
            .map(|binding| {
                Line::from(vec![
                    Span::raw(format!(
                        "{:width$}",
                        binding.keys_as_string(),
                        width = width
                    )),
                    Span::raw("  "),
                    Span::styled(
                        binding.target().as_str().replace('_', " "),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ])
            })
            .collect(),
    )
}

/// Where a popup of `height` rows sits in `area`: in the middle, and no bigger
/// than the screen.
fn popup_area(area: Rect, height: u16) -> Rect {
    let height = height.min(area.height);
    let width = area.width.saturating_sub(4).max(1);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// Whether the help popup is what is on top.
pub fn is_open(app: &App) -> bool {
    app.dispatcher().ui().popup() == Popup::Help
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::config::keymap::{Binding, KeyMap, Target};
    use crate::store::test_util;

    fn app() -> App {
        App::new(Component::Task, KeyMap::defaults())
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
    fn the_keys_shown_are_the_keys_that_work() {
        let mut app = app();
        app.react(Action::Help);

        let all = text(&screen(&app, 60, 24));

        for expected in ["move down", "open create", "quit", "help"] {
            assert!(all.contains(expected), "{expected} missing from {all:?}");
        }
    }

    #[test]
    fn a_rebound_key_is_the_one_the_help_offers() {
        let mut keymap = KeyMap::defaults();
        // Move the list on `x` instead of `j`, which is what somebody with a
        // muscle memory for `x` would do.
        keymap.override_binding(
            crate::config::keymap::Context::Normal,
            Binding::new("x", Target::MoveDown).unwrap(),
        );
        let mut app = App::new(Component::Task, keymap);
        app.react(Action::Help);

        let all = text(&screen(&app, 60, 24));

        assert!(
            all.contains("x") && !all.contains("j  "),
            "the help has to follow the keymap, found {all:?}"
        );
    }

    #[test]
    fn the_popup_sits_in_the_middle_and_clears_what_is_under_it() {
        let mut app = app();
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::Help);

        let buffer = screen(&app, 40, 20);
        let all = text(&buffer);

        assert!(
            all.starts_with("\n") || all.lines().next().unwrap().trim().is_empty(),
            "the list must not be showing through the top of the screen, found {all:?}"
        );
        assert!(
            !all.contains("Task 1"),
            "a popup with the list showing through it is not a popup, found {all:?}"
        );
        assert!(all.contains("Keys"), "found {all:?}");
    }

    #[test]
    fn a_help_popup_shorter_than_the_screen_leaves_the_edges_alone() {
        let mut app = app();
        app.react(Action::Help);

        let buffer = screen(&app, 40, 30);
        let top: String = (0..40).map(|x| buffer[(x, 0)].symbol()).collect();

        assert!(
            top.chars().all(|symbol| symbol == ' '),
            "the popup is in the middle, found {top:?}"
        );
    }

    #[test]
    fn nothing_is_drawn_when_the_help_is_not_open() {
        let app = app();

        let buffer = screen(&app, 40, 20);

        assert!(!is_open(&app));
        assert!(text(&buffer).trim().is_empty(), "found {:?}", text(&buffer));
    }

    #[test]
    fn a_popup_taller_than_the_screen_is_cut_down_rather_than_lost() {
        let area = Rect::new(0, 0, 40, 3);

        let popup = popup_area(area, 20);

        assert_eq!(popup.height, 3);
        assert!(popup.y + popup.height <= area.y + area.height);
        assert!(popup.width <= area.width);
    }
}
