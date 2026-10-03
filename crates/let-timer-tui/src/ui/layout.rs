//! Where each part of the screen goes.
//!
//! The application is one panel with a heading, a body, and a footer. There is
//! not much to arrange, and that is the point: the more the layout has to
//! decide, the more there is to get wrong on a terminal that is four rows tall.
//! The border is drawn around the whole panel so the edges are obvious.
//!
//! The room inside is handed back as a [`Regions`] struct rather than a tuple.
//! A tuple grows a field every time a pane is added, which means every caller
//! and every test has to change with it; a struct lets a new pane arrive as a
//! field nothing reads yet.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
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

/// The width the sidebar would like for itself.
const SIDEBAR_WIDTH: u16 = 18;

/// Below this width the sidebar is not drawn at all.
///
/// A list with a nineteen-column sidebar beside it on a forty-column terminal
/// is two lists of nine columns each, which is not a choice between anything.
/// The list is what the user came for, so the sidebar gives up its room first.
const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;

/// How many rows the detail panel needs before it is worth drawing at all.
///
/// A panel that cannot have three rows of its own is not drawn: half a panel is
/// a mistake, not a panel.
const MIN_DETAIL_HEIGHT: u16 = 3;

/// Where each part of the screen goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regions {
    /// The list of workspaces down the left, or nothing at all.
    pub sidebar: Rect,
    /// The rows the user is choosing from.
    pub list: Rect,
    /// What the highlighted row has to say for itself.
    pub detail: Rect,
}

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

/// Split `body` between the sidebar, the list, and the detail panel.
///
/// `want_sidebar` is the caller's answer to "is there anything to put in a
/// sidebar", and the width on screen is the layout's answer to "is there room".
/// Both have to be yes before the list moves sideways.
///
/// The list keeps the larger share of what is left, because it is what the user
/// is choosing from and a two-row list is not a choice.
pub fn split(body: Rect, want_detail: bool, want_sidebar: bool) -> Regions {
    let (sidebar, right) = carve_sidebar(body, want_sidebar);
    let [list, detail] = split_rows(right, want_detail);

    Regions {
        sidebar,
        list,
        detail,
    }
}

/// Take the sidebar's column off the left of `body`, if there is anything to put
/// in it and somewhere to put it.
///
/// Short-circuited rather than handed to the layout solver with a width of zero,
/// so that a screen with no sidebar gets back the very same `Rect` it went in
/// with, down to the column the list starts in.
fn carve_sidebar(body: Rect, want_sidebar: bool) -> (Rect, Rect) {
    if !want_sidebar || body.width < MIN_WIDTH_FOR_SIDEBAR {
        return (Rect::new(body.x, body.y, 0, body.height), body);
    }

    let [sidebar, right] =
        Layout::horizontal([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)]).areas(body);

    (sidebar, right)
}

/// The list above the detail panel.
fn split_rows(area: Rect, want_detail: bool) -> [Rect; 2] {
    if !want_detail || area.height < MIN_DETAIL_HEIGHT * 2 {
        return [area, Rect::new(area.x, area.y, area.width, 0)];
    }

    let list_height = (area.height * 3 / 5).max(MIN_DETAIL_HEIGHT);
    [
        Rect::new(area.x, area.y, area.width, list_height),
        Rect::new(
            area.x,
            area.y + list_height,
            area.width,
            area.height - list_height,
        ),
    ]
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
        let regions = split(Rect::new(0, 0, 20, 10), true, false);

        assert_eq!(regions.list.height, 6);
        assert_eq!(regions.detail.height, 4);
        assert_eq!(regions.detail.y, 6, "the panel starts where the list stops");
        assert_eq!(regions.list.width, regions.detail.width);
    }

    #[test]
    fn a_nothing_selected_list_keeps_all_of_the_room() {
        let regions = split(Rect::new(0, 0, 20, 10), false, false);

        assert_eq!(regions.list.height, 10);
        assert_eq!(regions.detail.height, 0);
    }

    #[test]
    fn a_screen_too_short_to_split_gives_everything_to_the_list() {
        let regions = split(Rect::new(0, 0, 20, 5), true, false);

        assert_eq!(regions.list.height, 5, "half a panel is not a panel");
        assert_eq!(regions.detail.height, 0);
    }

    // ── the sidebar ────────────────────────────────────────────────────

    #[test]
    fn nothing_is_drawn_in_a_sidebar_when_nothing_wants_one() {
        let regions = split(Rect::new(0, 0, 100, 10), false, false);

        assert_eq!(regions.sidebar.width, 0, "no sidebar was asked for");
        assert_eq!(
            regions.list,
            Rect::new(0, 0, 100, 10),
            "so the list gets the screen it had before there was a sidebar"
        );
    }

    #[test]
    fn a_wide_screen_gives_the_sidebar_its_own_columns_off_the_left() {
        let regions = split(Rect::new(0, 0, 100, 10), false, true);

        assert_eq!(regions.sidebar, Rect::new(0, 0, 18, 10));
        assert_eq!(
            regions.list,
            Rect::new(18, 0, 82, 10),
            "the list starts after the sidebar and keeps the rest"
        );
    }

    #[test]
    fn a_narrow_screen_gives_the_sidebar_nothing_rather_than_half_a_column() {
        let narrow = Rect::new(0, 0, MIN_WIDTH_FOR_SIDEBAR - 1, 10);

        let regions = split(narrow, false, true);

        assert_eq!(regions.sidebar.width, 0);
        assert_eq!(regions.list, narrow, "the list is not squeezed either");
    }

    #[test]
    fn a_sidebar_off_to_one_side_leaves_the_vertical_split_alone() {
        let without = split(Rect::new(0, 0, 100, 14), true, false);
        let with = split(Rect::new(0, 0, 100, 14), true, true);

        assert_eq!(with.list.height, without.list.height);
        assert_eq!(with.detail.height, without.detail.height);
        assert_eq!(with.list.y, without.list.y);
        assert_eq!(
            with.detail.y, without.detail.y,
            "the sidebar narrows the screen, it does not reshape it"
        );
    }

    #[test]
    fn a_sidebar_starts_where_the_frame_starts() {
        let frame = Rect::new(1, 1, 18, 8);

        let regions = split(frame, false, true);

        assert_eq!(
            (regions.sidebar.x, regions.sidebar.y),
            (frame.x, frame.y),
            "it sits inside the border, not under it"
        );
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
