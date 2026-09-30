use std::error::Error;
use std::fmt;

use inquire::error::CustomUserError;
use inquire::validator::Validation;
use inquire::{InquireError, Select, Text};
use let_timer_core::{
    Command, IpcClient, MediaList, NewMediaList, NewWorkspace, Priority, Response, Workspace,
};

/// Validator factory: input must contain something other than whitespace.
/// Returning `Ok(Validation::Invalid)` re-prompts instead of aborting.
fn required_text(
    message: &'static str,
) -> impl Fn(&str) -> Result<Validation, CustomUserError> + Clone {
    move |input: &str| {
        if input.trim().is_empty() {
            Ok(Validation::Invalid(message.into()))
        } else {
            Ok(Validation::Valid)
        }
    }
}

/// Validator factory: empty input is accepted (means "skip"), otherwise the
/// input must be a whole number of minutes.
fn optional_number(
    message: &'static str,
) -> impl Fn(&str) -> Result<Validation, CustomUserError> + Clone {
    move |input: &str| {
        if input.trim().is_empty() || input.trim().parse::<i64>().is_ok() {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid(message.into()))
        }
    }
}

/// Trim an input, mapping blank input to `None`.
fn trimmed_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ─── Task fields ─────────────────────────────────────────────────────

/// Ask for the task name (non-empty).
pub fn resolve_name() -> Result<String, Box<dyn Error>> {
    Ok(Text::new("Task name")
        .with_placeholder("e.g. Write the quarterly report")
        .with_validator(required_text("name cannot be empty"))
        .prompt()
        .map_err(prompt_error)?
        .trim()
        .to_string())
}

/// Ask for an optional description. An empty input means "no description".
pub fn resolve_description() -> Result<Option<String>, Box<dyn Error>> {
    let value = Text::new("Description (optional)")
        .with_placeholder("leave empty to skip")
        .prompt()
        .map_err(prompt_error)?;
    Ok(trimmed_or_none(&value))
}

/// Ask for a priority, defaulting the cursor to `not-yet`.
pub fn resolve_priority() -> Result<Priority, Box<dyn Error>> {
    let options: Vec<&str> = vec!["urgent", "immediate", "not-yet"];
    let selected = select("Priority", options)?.to_string();
    Ok(match selected.as_str() {
        "urgent" => Priority::Urgent,
        "immediate" => Priority::Immediate,
        _ => Priority::NotYet,
    })
}

/// Ask for an optional estimated duration in minutes. Empty input means "skip".
pub fn resolve_estimated_mins() -> Result<Option<i64>, Box<dyn Error>> {
    let value = Text::new("Estimated minutes (optional)")
        .with_placeholder("leave empty to skip")
        .with_validator(optional_number("expected a whole number of minutes"))
        .prompt()
        .map_err(prompt_error)?;
    Ok(trimmed_or_none(&value).and_then(|v| v.parse::<i64>().ok()))
}

// ─── Workspaces ──────────────────────────────────────────────────────

enum WorkspaceChoice {
    CreateNew,
    Existing(Workspace),
}

impl fmt::Display for WorkspaceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceChoice::CreateNew => write!(f, "＋  Create a new workspace"),
            WorkspaceChoice::Existing(ws) => write!(f, "{} (#{})", ws.name, ws.id),
        }
    }
}

/// Resolve a workspace id, prompting the user when `--workspace` was not given.
/// Offers an inline "create a new workspace" option, Vite-style.
pub async fn resolve_workspace(client: &mut IpcClient) -> Result<i64, Box<dyn Error>> {
    let workspaces = match client.request(&Command::ListWorkspaces).await? {
        Response::WorkspaceList(list) => list,
        Response::Error { message } => return Err(message.into()),
        other => return Err(format!("unexpected response to ListWorkspaces: {other:?}").into()),
    };

    let mut choices: Vec<WorkspaceChoice> = Vec::with_capacity(workspaces.len() + 1);
    choices.push(WorkspaceChoice::CreateNew);
    choices.extend(workspaces.into_iter().map(WorkspaceChoice::Existing));

    let choice = select("Workspace", choices)?;

    match choice {
        WorkspaceChoice::Existing(ws) => Ok(ws.id),
        WorkspaceChoice::CreateNew => {
            let name = Text::new("Name of the new workspace")
                .with_validator(required_text("workspace name cannot be empty"))
                .prompt()
                .map_err(prompt_error)?;
            let new_workspace = NewWorkspace {
                name: name.trim().to_string(),
                description: None,
            };
            match client
                .request(&Command::CreateWorkspace(new_workspace))
                .await?
            {
                Response::Workspace(ws) => Ok(ws.id),
                Response::Error { message } => Err(message.into()),
                other => Err(format!("unexpected response to CreateWorkspace: {other:?}").into()),
            }
        }
    }
}

// ─── Media lists ─────────────────────────────────────────────────────

enum MediaListChoice {
    None,
    CreateNew,
    Existing(MediaList),
}

impl fmt::Display for MediaListChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaListChoice::None => write!(f, "None (skip media list)"),
            MediaListChoice::CreateNew => write!(f, "＋  Create a new media list"),
            MediaListChoice::Existing(ml) => write!(f, "{} (#{})", ml.name, ml.id),
        }
    }
}

/// Resolve an optional media list id, prompting the user when `--media-list`
/// was not given. Offers "None" and an inline "create a new media list".
pub async fn resolve_media_list(client: &mut IpcClient) -> Result<Option<i64>, Box<dyn Error>> {
    let lists = match client.request(&Command::ListMediaLists).await? {
        Response::MediaListList(list) => list,
        Response::Error { message } => return Err(message.into()),
        other => return Err(format!("unexpected response to ListMediaLists: {other:?}").into()),
    };

    let mut choices: Vec<MediaListChoice> = Vec::with_capacity(lists.len() + 2);
    choices.push(MediaListChoice::None);
    choices.push(MediaListChoice::CreateNew);
    choices.extend(lists.into_iter().map(MediaListChoice::Existing));

    let choice = select("Media list", choices)?;

    match choice {
        MediaListChoice::None => Ok(None),
        MediaListChoice::Existing(ml) => Ok(Some(ml.id)),
        MediaListChoice::CreateNew => {
            let name = Text::new("Name of the new media list")
                .with_validator(required_text("media list name cannot be empty"))
                .prompt()
                .map_err(prompt_error)?;
            let new_media_list = NewMediaList {
                name: name.trim().to_string(),
                description: None,
            };
            match client
                .request(&Command::CreateMediaList(new_media_list))
                .await?
            {
                Response::MediaList(ml) => Ok(Some(ml.id)),
                Response::Error { message } => Err(message.into()),
                other => Err(format!("unexpected response to CreateMediaList: {other:?}").into()),
            }
        }
    }
}

// ─── Prompt primitives ───────────────────────────────────────────────

fn select<T>(message: &str, options: Vec<T>) -> Result<T, Box<dyn Error>>
where
    T: fmt::Display,
{
    let starting_cursor = default_cursor(message, &options);
    Select::new(message, options)
        .with_help_message("Type to filter (fuzzy); Esc to cancel")
        .with_starting_cursor(starting_cursor)
        .prompt()
        .map_err(prompt_error)
}

/// Keep a sensible default highlight: `not-yet` for priority, the first
/// option otherwise.
fn default_cursor<T: fmt::Display>(message: &str, options: &[T]) -> usize {
    if message == "Priority" {
        options
            .iter()
            .position(|o| o.to_string() == "not-yet")
            .unwrap_or(0)
    } else {
        0
    }
}

fn prompt_error(e: InquireError) -> Box<dyn Error> {
    match e {
        InquireError::NotTTY => {
            "cannot prompt here (not a terminal); pass the flag explicitly or use --no-interactive"
                .into()
        }
        other => other.into(),
    }
}
