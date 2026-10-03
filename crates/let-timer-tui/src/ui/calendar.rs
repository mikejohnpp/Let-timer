//! The date picker: a month, with one day on it.
//!
//! The picker is here because typing a date is the one field where typing is the
//! wrong interface. `2026-03-01` has four ways to be wrong in nine characters,
//! and the user cannot see the date they meant until the daemon refuses it.
//!
//! It is drawn rather than taken from a calendar widget, so the highlighted day
//! and the form's own date format are the same two places they are everywhere
//! else in this application: `%Y-%m-%d` and `chrono::NaiveDate`.

use chrono::{Datelike, NaiveDate};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::app::App;
use crate::store::Popup;

/// Weekday headings, Monday first.
const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Draw the date picker on top of the screen, if it is what is on top.
pub fn draw(frame: &mut Frame, app: &App) {
    let Some(day) = app.dispatcher().ui().calendar() else {
        // The picker has no day on it, so there is nothing to pick. This should
        // not be reachable; drawing nothing beats drawing an empty month.
        return;
    };

    if app.dispatcher().ui().popup() != Popup::Calendar {
        return;
    }

    let today = chrono::Local::now().date_naive();
    let lines = lines(day, Some(today));
    let area = area(frame.area(), lines.len() as u16 + 2);

    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(month_title(day));

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// The heading: the month the highlighted day is in.
fn month_title(day: NaiveDate) -> String {
    format!("{}", day.format("%B %Y"))
}

/// The month as lines: a heading row, the weekdays, and the weeks.
fn lines(day: NaiveDate, today: Option<NaiveDate>) -> Vec<Line<'static>> {
    let mut out = vec![Line::from(Span::raw(
        WEEKDAYS
            .iter()
            .map(|weekday| format!("{weekday:>3}"))
            .collect::<String>(),
    ))];

    let first = NaiveDate::from_ymd_opt(day.year(), day.month(), 1).unwrap_or(day);
    // Empty cells before the first of the month, so the first lands under the
    // right weekday instead of under Monday.
    let mut cells = vec![String::new(); first.weekday().num_days_from_monday() as usize];

    let mut cursor = first;
    while cursor.month() == first.month() && cursor.year() == first.year() {
        cells.push(cursor.day().to_string());
        let Some(next) = cursor.succ_opt() else {
            break;
        };
        cursor = next;
    }

    while !cells.len().is_multiple_of(WEEKDAYS.len()) {
        cells.push(String::new());
    }

    for week in cells.chunks(WEEKDAYS.len()) {
        let mut spans = Vec::new();
        for cell in week {
            let day_number: u32 = cell.parse().unwrap_or(0);
            let is_selected = day_number == day.day() && !cell.is_empty();
            let mut style = Style::default();
            if is_selected {
                style = style.add_modifier(Modifier::REVERSED);
            } else if Some(cursor_date(first, day_number)) == today {
                // Today is dimmed rather than marked, so the highlighted day
                // stays the only thing on screen with a block behind it.
                style = style.add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(format!("{cell:>3}"), style));
        }
        out.push(Line::from(spans));
    }

    out.push(Line::default());
    out.push(Line::from(Span::styled(
        format!("{} enter pick", day.format("%Y-%m-%d")),
        Style::default().add_modifier(Modifier::DIM),
    )));

    out
}

/// The date of `day_number` in `month`, for marking today.
fn cursor_date(month: NaiveDate, day_number: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(month.year(), month.month(), day_number).unwrap_or(month)
}

/// Where the picker sits: in the middle, and never bigger than the screen.
fn area(screen: Rect, height: u16) -> Rect {
    // A month is seven columns of three characters plus a row of padding, and a
    // narrower box would cut the last weekday off.
    let width = screen.width.saturating_sub(4).clamp(1, 30);
    let height = height.min(screen.height);
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

    fn app() -> App {
        App::new(Component::Task, KeyMap::defaults())
    }

    fn open_on(app: &mut App, day: NaiveDate) {
        app.react(Action::OpenCalendar);
        // Open the picker, then walk the highlight to the day under test, so the
        // tests do not depend on what day they happen to be run on.
        let today = app.dispatcher().ui().calendar().expect("the picker opened");
        let days = (day - today).num_days() as i32;
        for _ in 0..days.abs() {
            app.react(Action::CalendarMove(days.signum()));
        }
        assert_eq!(app.dispatcher().ui().calendar(), Some(day));
    }

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

    fn text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| row(buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_picker_opens_on_today_and_says_what_day_it_is() {
        let mut app = app();
        app.react(Action::OpenCalendar);

        let day = app.dispatcher().ui().calendar().expect("opened");
        let all = text(&screen(&app, 40, 12));

        assert_eq!(day, chrono::Local::now().date_naive());
        assert!(
            all.contains(&day.format("%B %Y").to_string()),
            "found {all:?}"
        );
        assert!(
            all.contains(&day.format("%Y-%m-%d").to_string()),
            "found {all:?}"
        );
    }

    #[test]
    fn the_days_of_the_month_line_up_under_the_right_weekdays() {
        // March 2026 starts on a Sunday, so the first of the month sits under
        // the last column and nothing is drawn in the six before it.
        let first = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

        let month = lines(first, None);
        let second = month
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert_eq!(second[0], " Mo Tu We Th Fr Sa Su");
        assert_eq!(
            second[1], "                    1",
            "one Sunday needs six empty three-wide cells before it"
        );
        assert!(
            second[2].starts_with("  2  3  4  5  6  7"),
            "the second week runs Monday to Saturday, found {:?}",
            second[2]
        );
    }

    #[test]
    fn a_month_that_starts_on_a_monday_needs_no_empty_cells() {
        // June 2026 starts on a Monday.
        let first = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        let month: Vec<String> = lines(first, None)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();

        assert!(month[1].starts_with("  1"), "found {:?}", month[1]);
    }

    #[test]
    fn the_day_under_the_highlight_is_the_only_one_with_a_block_behind_it() {
        let day = NaiveDate::from_ymd_opt(2026, 3, 17).unwrap();

        let buffer = screen_on(day, 40, 12);

        let highlighted: Vec<u16> = (0..buffer.area.height)
            .filter(|y| {
                (0..buffer.area.width)
                    .any(|x| buffer[(x, *y)].modifier.contains(Modifier::REVERSED))
            })
            .collect();

        assert_eq!(
            highlighted,
            vec![5],
            "one row, and it has to be the week holding the 17th"
        );
        assert!(
            row(&buffer, 5).contains("17"),
            "found {:?}",
            row(&buffer, 5)
        );
    }

    #[test]
    fn a_form_is_not_visible_through_the_picker() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));
        open_on(&mut app, NaiveDate::from_ymd_opt(2026, 3, 17).unwrap());

        let all = text(&screen(&app, 40, 14));

        assert!(all.contains("March 2026"), "found {all:?}");
        assert!(
            !all.contains("Description"),
            "the form behind the picker should be covered, found {all:?}"
        );
    }

    #[test]
    fn nothing_is_drawn_when_the_picker_is_not_open() {
        let mut app = app();
        app.react(Action::OpenCreate(Component::Task));

        let buffer = screen(&app, 40, 12);

        assert!(text(&buffer).trim().is_empty(), "found {:?}", text(&buffer));
    }

    #[test]
    fn a_picker_on_a_cramped_screen_keeps_its_weekdays() {
        let day = NaiveDate::from_ymd_opt(2026, 3, 17).unwrap();

        let all = text(&screen_on(day, 24, 10));

        assert!(all.contains("Mo"), "found {all:?}");
    }

    /// A picker open on `day`, without going through today's date first.
    fn screen_on(day: NaiveDate, width: u16, height: u16) -> Buffer {
        let mut app = app();
        open_on(&mut app, day);
        screen(&app, width, height)
    }
}
