//! The form panel: what the user is typing into.
//!
//! While a form is open it takes the place of the list, because the list is not
//! what anybody is working on at that moment. Every field gets a line, the
//! focused one is marked with a cursor, and a mistake is printed under the field
//! it belongs to rather than somewhere else on the screen.
//!
//! The panel draws the draft as it is, including text that does not parse. That
//! is deliberate: the store keeps the raw text so the user can fix one character
//! instead of starting over, and a panel that refused to draw half-typed text
//! would leave them typing into nothing.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::store::{Draft, FieldKind, FormKind, FormStore};

/// A block cursor, because the terminal's own would be sitting over the shell
/// once this is closed and a blinking one cannot be timed from here anyway.
const CURSOR: &str = "\u{2588}";

/// Draw the open form into `area`.
pub fn draw(frame: &mut Frame, form: &FormStore, area: Rect) {
    let Some(draft) = form.draft() else {
        return;
    };

    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title(draft));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = field_lines(draft, form);

    if !form.errors().is_empty() {
        lines.push(Line::default());
        lines.extend(error_lines(form));
    }

    lines.push(Line::default());
    lines.push(hint());

    // A form with more fields than rows still has to show the focused one, so
    // the view is scrolled to the focus rather than cut off at the bottom.
    let first = first_visible(draft, inner.height);
    let visible: Vec<Line<'_>> = lines
        .into_iter()
        .skip(first)
        .take(inner.height.saturating_sub(1) as usize)
        .collect();

    frame.render_widget(Paragraph::new(visible).wrap(Wrap { trim: false }), inner);
}

/// The heading for what is being edited.
fn title(draft: &Draft) -> &'static str {
    match draft.kind() {
        FormKind::NewTask => "New task",
        FormKind::EditTask(_) => "Edit task",
        FormKind::NewWorkspace => "New workspace",
        FormKind::NewMediaList => "New media list",
    }
}

/// One line per field, the focused one carrying the cursor.
fn field_lines(draft: &Draft, _form: &FormStore) -> Vec<Line<'static>> {
    let focus = draft.focus();

    draft
        .fields()
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let focused = index == focus;
            let style = if focused {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let mut spans = vec![
                Span::styled(format!("{:<16}", label(field.kind())), style),
                Span::raw(field.value().to_string()),
            ];

            if focused {
                // A field with nothing in it still has to show where the next
                // character will land, or an empty form looks like a dead one.
                spans.push(Span::styled(
                    CURSOR,
                    Style::default().add_modifier(Modifier::REVERSED),
                ));
            }

            Line::from(spans)
        })
        .collect()
}

/// What is wrong, one line per field, under the fields rather than on top of
/// them: a form that moves under the user's hands is a form nobody can fill in.
fn error_lines(form: &FormStore) -> Vec<Line<'static>> {
    form.errors()
        .iter()
        .map(|error| {
            Line::from(vec![
                Span::styled(
                    format!("{:<16}", label(error.field)),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    error.message.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect()
}

/// How to leave a form, since the keys are not letters a first-time user would
/// guess and the panel is the only place that can say so.
fn hint() -> Line<'static> {
    Line::from(Span::styled(
        "tab next field, enter save, esc cancel",
        Style::default().add_modifier(Modifier::DIM),
    ))
}

/// The heading for a field.
fn label(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Name => "Name",
        FieldKind::Description => "Description",
        FieldKind::Priority => "Priority",
        FieldKind::Status => "Status",
        FieldKind::EstimatedMins => "Minutes",
        FieldKind::ScheduledOn => "Scheduled on",
        FieldKind::WorkspaceId => "Workspace",
        FieldKind::MediaListId => "Media list",
    }
}

/// The first line to draw so the focused field is on screen.
///
/// Field positions and line positions are not the same thing once errors are on
/// screen below them, so this counts fields rather than assuming.
fn first_visible(draft: &Draft, height: u16) -> usize {
    let rows = height.saturating_sub(1) as usize;
    let focus = draft.focus();
    if rows == 0 || focus < rows {
        return 0;
    }
    // Keep the focused field one row above the bottom so the cursor is not on
    // the last line, where it looks like it belongs to the border.
    focus - rows + 2
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::action::{Action, Component};
    use crate::app::App;
    use crate::config::keymap::KeyMap;
    use crate::store::{Mode, test_util};

    fn app_with_form() -> App {
        let mut app = App::new(Mode::Fullscreen, Component::Task, KeyMap::defaults());
        app.react(Action::OpenCreate(Component::Task));
        app
    }

    fn screen(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame, app.dispatcher().form(), frame.area()))
            .unwrap();
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
    fn the_heading_says_whether_this_is_a_new_record_or_an_edit() {
        let mut app = app_with_form();
        assert!(row(&screen(&app, 40, 12), 0).contains("New task"));

        app.react(Action::Cancel);
        app.react(Action::TaskListLoaded(test_util::tasks(1)));
        app.react(Action::MoveSelection(0));
        app.react(Action::OpenEdit(Box::new(test_util::task_with_id(1))));

        assert!(row(&screen(&app, 40, 12), 0).contains("Edit task"));
    }

    #[test]
    fn every_field_gets_a_line_in_the_orders_the_store_chose() {
        let app = app_with_form();

        let buffer = screen(&app, 40, 14);
        let screen_text = text(&buffer);

        for expected in [
            "Name",
            "Description",
            "Priority",
            "Minutes",
            "Scheduled on",
            "Workspace",
            "Media list",
        ] {
            assert!(
                screen_text.contains(expected),
                "{expected} missing from {screen_text:?}"
            );
        }
        assert!(
            screen_text.find("Name") < screen_text.find("Priority"),
            "fields keep the order the store laid them out in, found {screen_text:?}"
        );
    }

    #[test]
    fn only_the_focused_field_carries_the_cursor() {
        let mut app = app_with_form();

        let cursors = |app: &App| text(&screen(app, 40, 14)).matches(CURSOR).count();

        assert_eq!(cursors(&app), 1, "one cursor, on the focused field");
        app.react(Action::FormNextField);
        assert_eq!(cursors(&app), 1, "still one cursor after moving on");
    }

    #[test]
    fn an_empty_focused_field_still_shows_where_the_next_character_goes() {
        let app = app_with_form();

        let first_field = text(&screen(&app, 40, 14))
            .lines()
            .find(|line| line.contains("Name"))
            .unwrap_or_default()
            .to_string();

        assert!(
            first_field.contains(CURSOR),
            "an empty field with no cursor looks like a dead form, found {first_field:?}"
        );
    }

    #[test]
    fn what_the_user_typed_is_shown_where_they_typed_it() {
        let mut app = app_with_form();
        for character in "milk".chars() {
            app.react(Action::FormInput(character));
        }

        let screen_text = text(&screen(&app, 40, 14));

        assert!(screen_text.contains("milk"), "found {screen_text:?}");
    }

    #[test]
    fn a_mistake_is_printed_under_the_field_it_belongs_to() {
        let mut app = app_with_form();
        app.react(Action::Submit);

        let screen_text = text(&screen(&app, 40, 16));

        assert!(
            screen_text.contains("name is required"),
            "an empty form has things to say, found {screen_text:?}"
        );
        assert!(
            screen_text.find("Media list") < screen_text.find("name is required"),
            "the complaint belongs under the fields it is about, found {screen_text:?}"
        );
    }

    #[test]
    fn a_form_taller_than_the_window_scrolls_to_the_focused_field() {
        let mut app = app_with_form();
        // The task form has seven fields; walk past the bottom of a five-row
        // window and the last field has to be the one on screen.
        for _ in 0..5 {
            app.react(Action::FormNextField);
        }

        let screen_text = text(&screen(&app, 40, 6));

        assert!(
            screen_text.contains("Media list"),
            "the focused field has to be visible, found {screen_text:?}"
        );
    }

    #[test]
    fn the_keys_that_leave_a_form_are_written_down() {
        let app = app_with_form();

        let screen_text = text(&screen(&app, 40, 14));

        assert!(
            screen_text.contains("enter save") && screen_text.contains("esc cancel"),
            "found {screen_text:?}"
        );
    }

    #[test]
    fn no_form_means_nothing_is_drawn_over_the_list() {
        let app = App::new(Mode::Fullscreen, Component::Task, KeyMap::defaults());

        let buffer = screen(&app, 30, 6);

        assert!(text(&buffer).trim().is_empty(), "found {:?}", text(&buffer));
    }
}
