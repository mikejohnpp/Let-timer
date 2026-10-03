//! Where each part of the screen goes.
//!
//! The application is one panel with a heading, a body, and eventually a
//! footer. There is not much to arrange, and that is the point: the more the
//! layout has to decide, the more there is to get wrong on a terminal that is
//! four rows tall. The border is drawn around the whole panel so the edges are
//! obvious.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders},
};

use crate::app::App;

/// The heading for the record type on screen.
pub fn title(app: &App) -> &'static str {
    match app.component() {
        crate::action::Component::Task => "Tasks",
        crate::action::Component::Workspace => "Workspaces",
        crate::action::Component::MediaList => "Media lists",
    }
}

/// How many rows the status line takes.
const STATUS_HEIGHT: u16 = 1;

/// Draw the frame around the content, and hand back the room inside it.
///
/// The first area is the body, the second the status line along the bottom of
/// the frame.
pub fn draw(frame: &mut Frame, app: &App) -> (Rect, Rect) {
    let area = frame.area();

    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title(app));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // A window one row inside the border has no room for a status line. The list
    // gets the row, because a list is what the user came for.
    if inner.height <= STATUS_HEIGHT {
        return (inner, Rect::new(inner.x, inner.y, inner.width, 0));
    }

    let status = Rect::new(
        inner.x,
        inner.y + inner.height - STATUS_HEIGHT,
        inner.width,
        STATUS_HEIGHT,
    );
    let body = Rect::new(inner.x, inner.y, inner.width, inner.height - STATUS_HEIGHT);
    (body, status)
}

/// Split `area` between the list and the detail panel.
///
/// The list keeps the larger share, because it is what the user is choosing from
/// and a two-row list is not a choice. A panel that cannot have three rows of
/// its own is not drawn at all: half a panel is a mistake, not a panel.
pub fn split(area: Rect, want_detail: bool) -> (Rect, Rect) {
    const MIN_DETAIL_HEIGHT: u16 = 3;

    if !want_detail || area.height < MIN_DETAIL_HEIGHT * 2 {
        return (area, Rect::new(area.x, area.y, area.width, 0));
    }

    let list_height = (area.height * 3 / 5).max(MIN_DETAIL_HEIGHT);
    let list = Rect::new(area.x, area.y, area.width, list_height);
    let detail = Rect::new(
        area.x,
        area.y + list_height,
        area.width,
        area.height - list_height,
    );
    (list, detail)
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

    use super::*;
    use crate::action::Component;
    use crate::config::keymap::KeyMap;

    fn app(component: Component) -> App {
        App::new(component, KeyMap::defaults())
    }

    /// Draw the layout alone, and hand back both what it drew and the room it
    /// said was inside.
    fn draw_layout(app: &App, width: u16, height: u16) -> (Buffer, Rect, Rect) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut inner = Rect::new(0, 0, 0, 0);
        let mut status = Rect::new(0, 0, 0, 0);
        terminal
            .draw(|frame| {
                (inner, status) = draw(frame, app);
            })
            .unwrap();
        (terminal.backend().buffer().clone(), inner, status)
    }

    /// One row of the buffer as a string, for looking at with human eyes.
    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn fullscreen_loses_a_row_and_a_column_to_the_border() {
        let (_, inner, status) = draw_layout(&app(Component::Task), 20, 10);

        assert_eq!(
            (inner.x, inner.y, inner.width, inner.height + status.height),
            (1, 1, 18, 8),
            "the border is not free"
        );
        assert_eq!(
            status,
            Rect::new(1, 8, 18, 1),
            "the last row is the status line"
        );
        assert_eq!(
            inner.height, 7,
            "the body stops where the status line starts"
        );
    }

    #[test]
    fn the_heading_says_which_records_are_on_screen() {
        for (component, expected) in [
            (Component::Task, "Tasks"),
            (Component::Workspace, "Workspaces"),
            (Component::MediaList, "Media lists"),
        ] {
            assert_eq!(title(&app(component)), expected);
        }
    }

    #[test]
    fn the_heading_sits_in_the_top_border_where_it_can_be_seen() {
        let (buffer, ..) = draw_layout(&app(Component::Workspace), 30, 6);

        assert!(
            row(&buffer, 0).contains("Workspaces"),
            "found {:?}",
            row(&buffer, 0)
        );
    }

    #[test]
    fn the_list_keeps_the_larger_share_of_the_screen() {
        let (list, detail) = split(Rect::new(0, 0, 20, 10), true);

        assert_eq!(list.height, 6);
        assert_eq!(detail.height, 4);
        assert_eq!(detail.y, 6, "the panel starts where the list stops");
        assert_eq!(list.width, detail.width);
    }

    #[test]
    fn a_nothing_selected_list_keeps_all_of_the_room() {
        let (list, detail) = split(Rect::new(0, 0, 20, 10), false);

        assert_eq!(list.height, 10);
        assert_eq!(detail.height, 0);
    }

    #[test]
    fn a_screen_too_short_to_split_gives_everything_to_the_list() {
        let (list, detail) = split(Rect::new(0, 0, 20, 5), true);

        assert_eq!(list.height, 5, "half a panel is not a panel");
        assert_eq!(detail.height, 0);
    }

    #[test]
    fn a_screen_too_short_for_a_border_still_gives_the_list_a_row() {
        let (buffer, inner, status) = draw_layout(&app(Component::Task), 10, 3);

        assert_eq!(
            inner.height, 1,
            "one row inside the border, found {inner:?}"
        );
        assert_eq!(status.height, 0, "the list gets the row, found {status:?}");
        assert!(!buffer.content.is_empty(), "something was still drawn");
    }

    #[test]
    fn a_screen_no_taller_than_the_border_gives_the_list_nothing_rather_than_panicking() {
        let (_, inner, status) = draw_layout(&app(Component::Task), 10, 1);

        assert_eq!(inner.height, 0);
        assert_eq!(status.height, 0);
    }
}
