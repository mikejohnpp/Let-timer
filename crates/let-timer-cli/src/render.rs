//! Printing what the daemon said.
//!
//! Two ways out, for two kinds of reader. A person wants a line per task, with
//! the columns in the order they scan them. A script wants the same answer as
//! JSON, with nothing added and nothing left out -- which is the reply as it
//! came over the socket, printed rather than picked apart, because a reply this
//! program does not know about is still a reply somebody else's script can
//! read.
//!
//! Everything here writes to a `Write` rather than straight to the terminal, so
//! that the columns can be tested without a terminal to test them on.

use std::io::Write;

use let_timer_core::{Response, Task, TaskPayload};

/// Print `response` for the reader `--json` asked for.
pub fn print(response: &Response, json: bool) -> std::io::Result<()> {
    let mut out = std::io::stdout();

    if json {
        // The reply as it came, rather than a shape this program picked out of
        // it: a script should not have to know which version produced it.
        // Serializing something that came off a socket can only fail if the
        // reply is not what the socket said it was, and there is nothing to say
        // about that here.
        let json = serde_json::to_string_pretty(response)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        writeln!(out, "{json}")?;
    } else {
        write_humans(response, &mut out)?;
    }

    Ok(())
}

/// Say a failure on stderr.
///
/// Not stdout: a script reading stdout has to be able to trust that everything
/// on it is the answer, and an error is not an answer.
pub fn complain(message: &str) {
    eprintln!("Error: {message}");
}

/// The reply as a person reads it.
pub fn write_humans(response: &Response, out: &mut impl Write) -> std::io::Result<()> {
    match response {
        Response::Ok(TaskPayload::Some(task)) => task_detail(out, task),
        Response::Ok(TaskPayload::None) => writeln!(out, "Nothing to show."),

        Response::OkList(tasks) => match tasks.is_empty() {
            // A header with nothing under it reads as a mistake in this program
            // rather than as an answer.
            true => writeln!(out, "No tasks."),
            false => table(
                out,
                &["ID", "PRIORITY", "STATUS", "ON", "NAME"],
                tasks.iter().map(|task| {
                    vec![
                        task.id.to_string(),
                        task.priority.as_str().to_string(),
                        task.status.as_str().to_string(),
                        day(task.scheduled_on),
                        task.name.clone(),
                    ]
                }),
            ),
        },

        Response::OkEmpty => writeln!(out, "Done."),

        Response::WorkspaceList(workspaces) => match workspaces.is_empty() {
            true => writeln!(out, "No workspaces."),
            false => table(
                out,
                &["ID", "NAME", "DESCRIPTION"],
                workspaces.iter().map(|workspace| {
                    vec![
                        workspace.id.to_string(),
                        workspace.name.clone(),
                        text(workspace.description.as_deref()),
                    ]
                }),
            ),
        },

        Response::MediaListList(lists) => match lists.is_empty() {
            true => writeln!(out, "No media lists."),
            false => table(
                out,
                &["ID", "NAME", "DESCRIPTION"],
                lists.iter().map(|list| {
                    vec![
                        list.id.to_string(),
                        list.name.clone(),
                        text(list.description.as_deref()),
                    ]
                }),
            ),
        },

        Response::Workspace(workspace) => {
            writeln!(out, "Workspace {}: {}", workspace.id, workspace.name)?;
            if let Some(description) = &workspace.description {
                writeln!(out, "  {description}")?;
            }
            Ok(())
        }

        Response::MediaList(list) => {
            writeln!(out, "Media list {}: {}", list.id, list.name)?;
            if let Some(description) = &list.description {
                writeln!(out, "  {description}")?;
            }
            Ok(())
        }

        // Nobody reads this one here: the caller has to exit non-zero over it,
        // and it belongs on stderr.
        Response::Error { .. } => Ok(()),
    }
}

/// One task, on its own, for a reply about one task.
fn task_detail(out: &mut impl Write, task: &Task) -> std::io::Result<()> {
    writeln!(out, "Task {}: {}", task.id, task.name)?;
    if let Some(description) = &task.description {
        writeln!(out, "  {description}")?;
    }
    writeln!(
        out,
        "  priority {} | status {} | {} mins | on {}",
        task.priority.as_str(),
        task.status.as_str(),
        task.estimated_mins
            .map(|mins| mins.to_string())
            .unwrap_or_else(|| "-".into()),
        day(task.scheduled_on),
    )
}

/// A missing day, a missing priority: a dash rather than a gap, because an empty
/// column looks like the row ended early.
fn day(date: Option<chrono::NaiveDate>) -> String {
    date.map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn text(value: Option<&str>) -> String {
    match value {
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => "-".to_string(),
    }
}

/// Rows in columns, each as wide as its widest cell.
fn table(
    out: &mut impl Write,
    headers: &[&str],
    rows: impl Iterator<Item = Vec<String>>,
) -> std::io::Result<()> {
    let rows: Vec<Vec<String>> = rows.collect();

    let mut widths: Vec<usize> = headers
        .iter()
        .map(|header| header.chars().count())
        .collect();
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }

    let line = |cells: &[String]| {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| match index == cells.len() - 1 {
                // The last column is not padded: trailing spaces are not
                // something anybody reads, and they turn up in a pipe.
                true => cell.clone(),
                false => format!("{cell:<width$}", width = widths[index]),
            })
            .collect::<Vec<_>>()
            .join("  ")
    };

    writeln!(
        out,
        "{}",
        line(
            &headers
                .iter()
                .map(|header| header.to_string())
                .collect::<Vec<_>>()
        )
    )?;
    for row in &rows {
        writeln!(out, "{}", line(row))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use let_timer_core::db::models::{MediaList, Priority, Workspace};
    use let_timer_core::{TaskStatus, date_on};

    use super::*;

    fn task(id: i64, name: &str) -> Task {
        Task {
            id,
            workspace_id: 1,
            media_list_id: None,
            name: name.to_string(),
            description: None,
            priority: Priority::NotYet,
            status: TaskStatus::Pending,
            estimated_mins: None,
            scheduled_on: None,
            created_at: "2026-10-01 09:00:00".to_string(),
            updated_at: "2026-10-01 09:00:00".to_string(),
        }
    }

    fn print_to(response: &Response) -> String {
        let mut out = Vec::new();
        write_humans(response, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn json_is_the_reply_itself() {
        // Whatever the daemon said, verbatim: a script reading this should not
        // have to know which version of let-timer produced it.
        let response = Response::Ok(TaskPayload::None);

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            "{\"Ok\":\"None\"}"
        );
    }

    #[test]
    fn an_empty_list_says_so_rather_than_printing_a_header() {
        assert_eq!(print_to(&Response::OkList(vec![])), "No tasks.\n");
    }

    #[test]
    fn an_empty_workspace_list_says_so() {
        assert_eq!(
            print_to(&Response::WorkspaceList(vec![])),
            "No workspaces.\n"
        );
    }

    #[test]
    fn an_empty_media_list_list_says_so() {
        assert_eq!(
            print_to(&Response::MediaListList(vec![])),
            "No media lists.\n"
        );
    }

    #[test]
    fn a_missing_description_is_a_dash() {
        let out = print_to(&Response::WorkspaceList(vec![Workspace {
            id: 1,
            name: "chores".into(),
            description: None,
            created_at: "2026-10-01 09:00:00".into(),
            updated_at: "2026-10-01 09:00:00".into(),
        }]));

        assert!(out.contains("chores"), "{out}");
        assert!(
            out.lines().nth(1).unwrap().trim_end().ends_with('-'),
            "{out}"
        );
    }

    #[test]
    fn a_blank_description_is_a_dash_too() {
        // A space is not a description; it would print as a column of nothing
        // and read as an empty field rather than a missing one.
        assert_eq!(text(Some("   ")), "-");
        assert_eq!(text(None), "-");
        assert_eq!(text(Some("a thing")), "a thing");
    }

    #[test]
    fn columns_line_up_when_one_name_is_much_longer() {
        let out = print_to(&Response::OkList(vec![
            task(1, "a"),
            task(22, "a much longer name"),
        ]));

        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "{out}");

        // Every row's name begins under the header's NAME, which is the whole
        // reason for lining the columns up.
        let column = lines[0].find("NAME").expect("the header names the column");
        let indent = |line: &str| {
            let rest = &line[column..];
            rest.len() - rest.trim_start().len()
        };
        assert_eq!(indent(lines[1]), indent(lines[2]), "{out}");
    }

    #[test]
    fn the_last_column_is_not_padded() {
        // Trailing spaces turn up in a pipe and in a diff, and nobody reading a
        // table wants them.
        let out = print_to(&Response::OkList(vec![task(1, "wash up")]));

        for line in out.lines() {
            assert_eq!(line, line.trim_end(), "{line:?}");
        }
    }

    #[test]
    fn a_task_list_says_which_day_each_task_is_for() {
        let mut task = task(1, "wash up");
        task.scheduled_on = date_on("2026-10-05").unwrap();

        assert!(print_to(&Response::OkList(vec![task])).contains("2026-10-05"));
    }

    #[test]
    fn a_task_with_no_day_leaves_the_column_empty() {
        let out = print_to(&Response::OkList(vec![task(1, "wash up")]));

        assert!(out.lines().nth(1).unwrap().contains('-'), "{out}");
    }

    #[test]
    fn one_task_on_its_own_is_named_and_dated() {
        let mut task = task(4, "wash up");
        task.description = Some("the dishes".into());
        task.estimated_mins = Some(20);
        task.scheduled_on = date_on("2026-12-25").unwrap();

        let out = print_to(&Response::Ok(TaskPayload::Some(task)));

        assert!(out.contains("Task 4: wash up"), "{out}");
        assert!(out.contains("the dishes"), "{out}");
        assert!(out.contains("20 mins"), "{out}");
        assert!(out.contains("2026-12-25"), "{out}");
    }

    #[test]
    fn a_reply_about_nothing_says_nothing() {
        assert_eq!(
            print_to(&Response::Ok(TaskPayload::None)),
            "Nothing to show.\n"
        );
    }

    #[test]
    fn a_media_list_reply_says_what_it_is() {
        let out = print_to(&Response::MediaList(MediaList {
            id: 4,
            name: "books".into(),
            description: Some("to read".into()),
            created_at: "2026-10-01 09:00:00".into(),
            updated_at: "2026-10-01 09:00:00".into(),
        }));

        assert!(out.contains("Media list 4: books"), "{out}");
        assert!(out.contains("to read"), "{out}");
    }

    #[test]
    fn a_workspace_reply_says_what_it_is() {
        let out = print_to(&Response::Workspace(Workspace {
            id: 2,
            name: "chores".into(),
            description: None,
            created_at: "2026-10-01 09:00:00".into(),
            updated_at: "2026-10-01 09:00:00".into(),
        }));

        assert_eq!(out, "Workspace 2: chores\n");
    }

    #[test]
    fn a_command_that_did_nothing_says_done() {
        assert_eq!(print_to(&Response::OkEmpty), "Done.\n");
    }

    #[test]
    fn nothing_goes_to_stdout_for_an_error() {
        // The caller has to exit non-zero over an error, and it has to be on
        // stderr for that to mean anything to a script.
        assert_eq!(
            print_to(&Response::Error {
                message: "no such task".into()
            }),
            ""
        );
    }
}
