//! The draft a create or edit form is editing.
//!
//! A form holds plain strings rather than typed values, because every field is
//! typed into character by character and only parsed on submit. Which fields
//! exist, and whether they are required, depends on what is being edited: a
//! task has seven fields, a workspace has two.
//!
//! Fields also remember whether the user touched them. An edit form is
//! submitted as an `UpdateTask`, where an absent field means "leave it alone",
//! so a field the user never touched must stay absent.

use chrono::NaiveDate;
use let_timer_core::{
    Command, NewMediaList, NewTask, NewWorkspace, Priority, Task, TaskStatus, UpdateTask,
};

use crate::action::{Action, Component};
use crate::effect::Effect;
use crate::store::{EffectQueue, Store};

/// A field that cannot be submitted as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormError {
    /// The field at fault, so a view can point at it.
    pub field: FieldKind,
    /// What is wrong with it.
    pub message: String,
}

impl FormError {
    fn new(field: FieldKind, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

/// Which form is open.
///
/// No `PartialEq`: the wrapped `Task` model has none either. Tests and views
/// use `matches!` on the variant instead.
#[derive(Debug, Clone)]
pub enum FormKind {
    /// Blank task form. The daemon starts new tasks as `Pending`.
    NewTask,
    /// Task form seeded from an existing task.
    EditTask(Box<Task>),
    /// Blank workspace form.
    NewWorkspace,
    /// Blank media list form.
    NewMediaList,
}

/// What one field of the form is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Name,
    Description,
    Priority,
    /// Only present when editing: new tasks always start out `Pending`.
    Status,
    EstimatedMins,
    ScheduledOn,
    /// Only present when creating: a task cannot be moved between workspaces.
    WorkspaceId,
    MediaListId,
}

/// One editable value in the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    kind: FieldKind,
    value: String,
    /// Whether the user changed this field. Untouched fields are left out of
    /// an edit so the daemon keeps what it already has.
    touched: bool,
}

impl Field {
    fn new(kind: FieldKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
            touched: false,
        }
    }

    /// Which value this field holds.
    pub fn kind(&self) -> FieldKind {
        self.kind
    }

    /// The text in the field, which is not yet a valid value of anything.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Whether the user changed it. Untouched fields are left out of an edit.
    pub fn is_touched(&self) -> bool {
        self.touched
    }
}

/// An open form: its kind, its fields, and which one has focus.
#[derive(Debug, Clone)]
pub struct Draft {
    kind: FormKind,
    fields: Vec<Field>,
    focus: usize,
}

impl Draft {
    /// Every field, in the order they are shown.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Index of the focused field.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// The focused field.
    pub fn focused_field(&self) -> &Field {
        &self.fields[self.focus]
    }

    /// The focused field's kind.
    pub fn focused_kind(&self) -> FieldKind {
        self.focused_field().kind
    }

    /// What is being edited.
    pub fn kind(&self) -> &FormKind {
        &self.kind
    }

    /// The text in a field, empty when the form has no such field.
    pub fn value(&self, kind: FieldKind) -> Option<&str> {
        self.field(kind).map(|field| field.value.as_str())
    }

    /// Whether the user changed a field. Absent fields count as untouched.
    pub fn is_touched(&self, kind: FieldKind) -> bool {
        self.field(kind).is_some_and(|field| field.touched)
    }

    fn field(&self, kind: FieldKind) -> Option<&Field> {
        self.fields.iter().find(|field| field.kind == kind)
    }

    fn field_mut(&mut self, kind: FieldKind) -> Option<&mut Field> {
        self.fields.iter_mut().find(|field| field.kind == kind)
    }

    /// Whether the form has this field at all.
    fn has(&self, kind: FieldKind) -> bool {
        self.field(kind).is_some()
    }

    /// Append a character to the focused field, marking it touched.
    fn input(&mut self, character: char) {
        let kind = self.focused_kind();
        if let Some(field) = self.field_mut(kind) {
            field.value.push(character);
            field.touched = true;
        }
    }

    /// Delete the last character of the focused field.
    ///
    /// Backspacing down to empty counts as touching the field, so clearing a
    /// field in an edit form is sent to the daemon as a real change.
    fn backspace(&mut self) {
        let kind = self.focused_kind();
        if let Some(field) = self.field_mut(kind) {
            field.value.pop();
            field.touched = true;
        }
    }

    /// Replace the focused field's text outright, marking it touched.
    ///
    /// Used by the calendar, which supplies a whole date rather than keystrokes.
    fn set(&mut self, kind: FieldKind, value: impl Into<String>) {
        if let Some(field) = self.field_mut(kind) {
            field.value = value.into();
            field.touched = true;
        }
    }

    fn focus_next(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + 1) % self.fields.len();
        }
    }

    fn focus_prev(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
        }
    }

    /// Check every field, reporting each problem.
    ///
    /// Every field is checked, not just the focused one, so submitting shows
    /// all the mistakes at once instead of one per attempt.
    pub fn validate(&self) -> Vec<FormError> {
        let mut errors = Vec::new();

        if self.text(FieldKind::Name).trim().is_empty() {
            errors.push(FormError::new(FieldKind::Name, "name is required"));
        }
        // Only forms that actually have these fields are held to them: a
        // workspace form has neither priority nor status, and an absent field
        // reads as empty text, which would fail every parse below.
        if self.has(FieldKind::Priority)
            && self.text(FieldKind::Priority).parse::<Priority>().is_err()
        {
            errors.push(FormError::new(
                FieldKind::Priority,
                "expected not-yet, immediate or urgent",
            ));
        }

        // Only present when editing; new tasks always start out pending.
        if self.has(FieldKind::Status)
            && self.text(FieldKind::Status).parse::<TaskStatus>().is_err()
        {
            errors.push(FormError::new(
                FieldKind::Status,
                "expected pending, in-progress, completed or cancelled",
            ));
        }

        for kind in [FieldKind::EstimatedMins, FieldKind::MediaListId] {
            if parse_i64(self.text(kind)).is_err() {
                errors.push(FormError::new(kind, "expected a whole number"));
            }
        }

        // A new task must say which workspace it belongs to; an edit cannot
        // change that, so it has no such field at all.
        if self.has(FieldKind::WorkspaceId) {
            if self.text(FieldKind::WorkspaceId).trim().is_empty() {
                errors.push(FormError::new(
                    FieldKind::WorkspaceId,
                    "a task must belong to a workspace",
                ));
            } else if parse_i64(self.text(FieldKind::WorkspaceId)).is_err() {
                errors.push(FormError::new(
                    FieldKind::WorkspaceId,
                    "expected a whole number",
                ));
            }
        }

        if self.text(FieldKind::ScheduledOn).trim().is_empty() {
            return errors;
        }
        if let Err(message) = parse_date(self.text(FieldKind::ScheduledOn)) {
            errors.push(FormError::new(FieldKind::ScheduledOn, message));
        }

        errors
    }

    /// Turn the draft into the command that saves it.
    ///
    /// Returns `None` when the draft does not validate, so a caller can never
    /// send a half-typed form to the daemon.
    pub fn to_command(&self) -> Option<Command> {
        if !self.validate().is_empty() {
            return None;
        }

        let name = self.text(FieldKind::Name).trim().to_string();
        let description = match self.text(FieldKind::Description).trim() {
            "" => None,
            text => Some(text.to_string()),
        };
        let estimated_mins = parse_i64(self.text(FieldKind::EstimatedMins))
            .ok()
            .flatten();
        let scheduled_on = parse_date(self.text(FieldKind::ScheduledOn)).ok().flatten();
        let media_list_id = parse_i64(self.text(FieldKind::MediaListId)).ok().flatten();

        match &self.kind {
            FormKind::NewTask => Some(Command::Create(NewTask {
                workspace_id: parse_i64(self.text(FieldKind::WorkspaceId))
                    .ok()?
                    .unwrap_or_default(),
                media_list_id,
                name,
                description,
                priority: self.priority()?,
                estimated_mins,
                scheduled_on,
            })),
            FormKind::EditTask(task) => Some(Command::Edit {
                id: task.id,
                update: UpdateTask {
                    name: Some(name),
                    // `Some(None)` clears the column, absent leaves it alone.
                    // Both happen whenever the user touched the field, since
                    // backspacing a value down to empty is an edit too.
                    description: self
                        .is_touched(FieldKind::Description)
                        .then(|| description.clone()),
                    media_list_id: self
                        .is_touched(FieldKind::MediaListId)
                        .then_some(media_list_id),
                    priority: Some(self.priority()?),
                    status: match self.text(FieldKind::Status) {
                        text if text.trim().is_empty() => None,
                        text => Some(text.parse().ok()?),
                    },
                    // `estimated_mins` is a plain `Option`, not a double
                    // option, so an emptied field cannot express "clear it":
                    // it is left out and the old value survives.
                    estimated_mins: self
                        .is_touched(FieldKind::EstimatedMins)
                        .then_some(estimated_mins)
                        .flatten(),
                    scheduled_on: self
                        .is_touched(FieldKind::ScheduledOn)
                        .then_some(scheduled_on),
                },
            }),
            FormKind::NewWorkspace => {
                Some(Command::CreateWorkspace(NewWorkspace { name, description }))
            }
            FormKind::NewMediaList => {
                Some(Command::CreateMediaList(NewMediaList { name, description }))
            }
        }
    }

    /// A field's text, or empty when the form has no such field.
    fn text(&self, kind: FieldKind) -> &str {
        self.value(kind).unwrap_or_default()
    }

    fn priority(&self) -> Option<Priority> {
        self.text(FieldKind::Priority).parse().ok()
    }

    /// A blank task form.
    fn new_task() -> Self {
        Self {
            kind: FormKind::NewTask,
            fields: vec![
                Field::new(FieldKind::Name, ""),
                Field::new(FieldKind::Description, ""),
                Field::new(FieldKind::Priority, Priority::NotYet.as_str()),
                Field::new(FieldKind::EstimatedMins, ""),
                Field::new(FieldKind::ScheduledOn, ""),
                Field::new(FieldKind::WorkspaceId, ""),
                Field::new(FieldKind::MediaListId, ""),
            ],
            focus: 0,
        }
    }

    /// A task form seeded from `task`, marking nothing as touched.
    fn edit_task(task: &Task) -> Self {
        Self {
            kind: FormKind::EditTask(Box::new(task.clone())),
            fields: vec![
                Field::new(FieldKind::Name, task.name.clone()),
                Field::new(
                    FieldKind::Description,
                    task.description.clone().unwrap_or_default(),
                ),
                Field::new(FieldKind::Priority, task.priority.as_str()),
                Field::new(FieldKind::Status, task.status.as_str()),
                Field::new(
                    FieldKind::EstimatedMins,
                    task.estimated_mins
                        .map(|mins| mins.to_string())
                        .unwrap_or_default(),
                ),
                Field::new(
                    FieldKind::ScheduledOn,
                    task.scheduled_on
                        .map(|date| date.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                ),
                Field::new(
                    FieldKind::MediaListId,
                    task.media_list_id
                        .map(|id| id.to_string())
                        .unwrap_or_default(),
                ),
            ],
            focus: 0,
        }
    }

    /// A blank workspace or media list form.
    fn new_named(kind: FormKind) -> Self {
        Self {
            kind,
            fields: vec![
                Field::new(FieldKind::Name, ""),
                Field::new(FieldKind::Description, ""),
            ],
            focus: 0,
        }
    }
}

/// The open form, if any.
#[derive(Debug, Default)]
pub struct FormStore {
    draft: Option<Draft>,
    /// Why the last submit was refused, shown next to the fields.
    errors: Vec<FormError>,
    effects: EffectQueue,
}

impl FormStore {
    /// No form open.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a form is open.
    pub fn is_open(&self) -> bool {
        self.draft.is_some()
    }

    /// The open form, if any.
    pub fn draft(&self) -> Option<&Draft> {
        self.draft.as_ref()
    }

    /// What is wrong with the form, if the last submit was refused.
    pub fn errors(&self) -> &[FormError] {
        &self.errors
    }

    /// Whether a particular field has a complaint against it.
    pub fn error_on(&self, kind: FieldKind) -> Option<&FormError> {
        self.errors.iter().find(|error| error.field == kind)
    }
}

impl Store for FormStore {
    fn update(&mut self, action: Action) {
        match action {
            Action::OpenCreate(Component::Task) => self.open(Draft::new_task()),
            Action::OpenCreate(Component::Workspace) => {
                self.open(Draft::new_named(FormKind::NewWorkspace));
            }
            Action::OpenCreate(Component::MediaList) => {
                self.open(Draft::new_named(FormKind::NewMediaList));
            }

            Action::OpenEdit(task) => self.open(Draft::edit_task(&task)),

            Action::FormNextField => {
                if let Some(draft) = &mut self.draft {
                    draft.focus_next();
                }
            }
            Action::FormPrevField => {
                if let Some(draft) = &mut self.draft {
                    draft.focus_prev();
                }
            }

            // Cancel closes the form. How this interacts with the ui store's
            // own cancel handling (which closes an inline panel) is settled
            // when the dispatcher wires the stores together in 4.11.
            Action::Cancel => {
                self.draft = None;
                self.errors.clear();
            }

            Action::FormInput(character) => {
                if let Some(draft) = &mut self.draft {
                    draft.input(character);
                }
            }
            Action::FormBackspace => {
                if let Some(draft) = &mut self.draft {
                    draft.backspace();
                }
            }

            // Clearing the date field is the one edit a typed form cannot
            // express: backspacing "2026-10-05" down to empty already works,
            // but this does it in one keystroke.
            // The day the user left the highlight on in the calendar. A form
            // with no date field, or no form at all, has nothing to do with it.
            Action::PickDate(day) => {
                if let Some(draft) = self.draft.as_mut()
                    && draft.field(FieldKind::ScheduledOn).is_some()
                {
                    draft.set(FieldKind::ScheduledOn, day.format("%Y-%m-%d").to_string());
                }
            }

            Action::ClearDate => {
                if let Some(draft) = &mut self.draft {
                    draft.set(FieldKind::ScheduledOn, "");
                }
            }

            Action::Submit => self.submit(),

            // The calendar popup is a view concern and lands with the form
            // screens, along with the delete confirmation.
            _ => {}
        }
    }

    fn take_effects(&mut self) -> Vec<Effect> {
        self.effects.drain()
    }
}

impl FormStore {
    /// Show a form, dropping complaints left over from a previous one.
    fn open(&mut self, draft: Draft) {
        self.errors.clear();
        self.draft = Some(draft);
    }

    /// Turn the draft into a save command, or record why it cannot be saved.
    ///
    /// A form that does not validate stays open with its errors, so the user
    /// can fix the fields and submit again.
    fn submit(&mut self) {
        let Some(draft) = &self.draft else {
            return;
        };
        match draft.to_command() {
            Some(command) => {
                self.draft = None;
                self.errors.clear();
                self.effects.push(Effect::Send(command));
            }
            None => self.errors = draft.validate(),
        }
    }
}

/// Parse an optional integer field. Empty text means "not set".
///
/// The caller decides which field the complaint belongs to, so this reports
/// only that the text was not a number.
fn parse_i64(value: &str) -> Result<Option<i64>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed.parse().map(Some).map_err(|_| ())
}

/// Parse an optional `YYYY-MM-DD` field. Empty means "not scheduled".
fn parse_date(value: &str) -> Result<Option<NaiveDate>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| format!("{trimmed:?} is not a date, expected YYYY-MM-DD"))
}

/// The name shown and typed for a priority.
#[cfg(test)]
mod tests {
    use crate::effect::Effect;
    use crate::store::test_util::task_with_id;

    use super::*;

    fn send(store: &mut FormStore, action: Action) -> Vec<Effect> {
        store.update(action);
        store.take_effects()
    }

    fn kinds(draft: &Draft) -> Vec<FieldKind> {
        draft.fields().iter().map(|field| field.kind).collect()
    }

    #[test]
    fn starts_with_no_form_open() {
        let store = FormStore::new();
        assert!(!store.is_open());
        assert!(store.draft().is_none());
    }

    #[test]
    fn opening_a_task_create_form_lists_the_task_fields() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        let draft = store.draft().expect("form should be open");
        assert!(matches!(draft.kind(), FormKind::NewTask));
        assert_eq!(
            kinds(draft),
            vec![
                FieldKind::Name,
                FieldKind::Description,
                FieldKind::Priority,
                FieldKind::EstimatedMins,
                FieldKind::ScheduledOn,
                FieldKind::WorkspaceId,
                FieldKind::MediaListId,
            ]
        );
        assert_eq!(draft.focus(), 0);
        assert_eq!(draft.focused_kind(), FieldKind::Name);
    }

    #[test]
    fn a_new_task_form_starts_empty_at_the_default_priority() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        let draft = store.draft().unwrap();

        assert_eq!(draft.value(FieldKind::Name), Some(""));
        assert_eq!(draft.value(FieldKind::Priority), Some("not-yet"));
        assert_eq!(draft.value(FieldKind::ScheduledOn), Some(""));
    }

    #[test]
    fn a_new_task_form_has_no_status_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        let draft = store.draft().unwrap();

        assert!(
            !kinds(draft).contains(&FieldKind::Status),
            "the daemon starts new tasks as pending"
        );
    }

    #[test]
    fn opening_a_workspace_form_lists_only_name_and_description() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));

        let draft = store.draft().expect("form should be open");
        assert!(matches!(draft.kind(), FormKind::NewWorkspace));
        assert_eq!(kinds(draft), vec![FieldKind::Name, FieldKind::Description]);
    }

    #[test]
    fn opening_a_media_list_form_lists_only_name_and_description() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::MediaList));

        let draft = store.draft().expect("form should be open");
        assert!(matches!(draft.kind(), FormKind::NewMediaList));
        assert_eq!(kinds(draft), vec![FieldKind::Name, FieldKind::Description]);
    }

    #[test]
    fn an_edit_form_is_seeded_from_the_task() {
        let mut task = task_with_id(4);
        task.name = "Write docs".to_string();
        task.description = Some("the long version".to_string());
        task.priority = Priority::Urgent;
        task.status = TaskStatus::InProgress;
        task.estimated_mins = Some(90);
        task.scheduled_on = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        task.media_list_id = Some(7);

        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task)));

        let draft = store.draft().expect("form should be open");
        assert!(matches!(draft.kind(), FormKind::EditTask(_)));
        assert_eq!(draft.value(FieldKind::Name), Some("Write docs"));
        assert_eq!(
            draft.value(FieldKind::Description),
            Some("the long version")
        );
        assert_eq!(draft.value(FieldKind::Priority), Some("urgent"));
        assert_eq!(draft.value(FieldKind::Status), Some("in-progress"));
        assert_eq!(draft.value(FieldKind::EstimatedMins), Some("90"));
        assert_eq!(draft.value(FieldKind::ScheduledOn), Some("2026-10-05"));
        assert_eq!(draft.value(FieldKind::MediaListId), Some("7"));
    }

    #[test]
    fn an_edit_form_starts_with_nothing_touched() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(1))));
        let draft = store.draft().unwrap();

        for field in draft.fields() {
            assert!(
                !field.touched,
                "{:?} should not count as edited on a freshly opened form",
                field.kind
            );
        }
    }

    #[test]
    fn an_edit_form_has_no_workspace_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(1))));
        let draft = store.draft().unwrap();

        assert!(
            !kinds(draft).contains(&FieldKind::WorkspaceId),
            "a task cannot be moved between workspaces"
        );
    }

    #[test]
    fn focus_wraps_forward_through_every_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));
        let field_count = store.draft().unwrap().fields().len();

        for expected in 1..field_count {
            send(&mut store, Action::FormNextField);
            assert_eq!(store.draft().unwrap().focus(), expected);
        }

        send(&mut store, Action::FormNextField);
        assert_eq!(
            store.draft().unwrap().focus(),
            0,
            "past the last field comes back to the first"
        );
    }

    #[test]
    fn focus_wraps_backward_through_every_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        let field_count = store.draft().unwrap().fields().len();

        send(&mut store, Action::FormPrevField);
        assert_eq!(
            store.draft().unwrap().focus(),
            field_count - 1,
            "before the first field comes back to the last"
        );

        send(&mut store, Action::FormNextField);
        assert_eq!(store.draft().unwrap().focus(), 0);
    }

    #[test]
    fn opening_another_form_replaces_the_first() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        send(&mut store, Action::OpenCreate(Component::Workspace));

        assert!(
            matches!(store.draft().unwrap().kind(), FormKind::NewWorkspace),
            "only one form is ever open"
        );
    }

    #[test]
    fn cancel_closes_the_form() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        send(&mut store, Action::Cancel);

        assert!(!store.is_open());
    }

    #[test]
    fn cancel_with_no_form_open_is_harmless() {
        let mut store = FormStore::new();
        send(&mut store, Action::Cancel);
        assert!(!store.is_open());
    }

    #[test]
    fn moving_focus_with_no_form_open_is_harmless() {
        let mut store = FormStore::new();
        send(&mut store, Action::FormNextField);
        send(&mut store, Action::FormPrevField);
        assert!(!store.is_open());
    }

    #[test]
    fn a_two_field_form_still_cycles_correctly() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::MediaList));

        send(&mut store, Action::FormNextField);
        assert_eq!(
            store.draft().unwrap().focused_kind(),
            FieldKind::Description
        );

        send(&mut store, Action::FormNextField);
        assert_eq!(store.draft().unwrap().focused_kind(), FieldKind::Name);

        send(&mut store, Action::FormPrevField);
        assert_eq!(
            store.draft().unwrap().focused_kind(),
            FieldKind::Description
        );
    }

    // ── typing ────────────────────────────────────────────────────────

    #[test]
    fn typing_goes_into_the_focused_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        for character in "Write docs".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let draft = store.draft().unwrap();
        assert_eq!(draft.value(FieldKind::Name), Some("Write docs"));
        assert_eq!(draft.focused_kind(), FieldKind::Name);
    }

    #[test]
    fn typing_follows_the_focus() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));
        send(&mut store, Action::FormInput('B'));
        send(&mut store, Action::FormNextField);
        send(&mut store, Action::FormInput('C'));

        let draft = store.draft().unwrap();
        assert_eq!(draft.value(FieldKind::Name), Some("B"));
        assert_eq!(draft.value(FieldKind::Description), Some("C"));
    }

    #[test]
    fn typing_marks_the_field_as_touched() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        assert!(!store.draft().unwrap().is_touched(FieldKind::Name));

        send(&mut store, Action::FormInput('x'));

        assert!(store.draft().unwrap().is_touched(FieldKind::Name));
        assert!(!store.draft().unwrap().is_touched(FieldKind::Description));
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "abc".chars() {
            send(&mut store, Action::FormInput(character));
        }

        send(&mut store, Action::FormBackspace);

        assert_eq!(store.draft().unwrap().value(FieldKind::Name), Some("ab"));
    }

    #[test]
    fn backspacing_a_seeded_field_down_to_empty_still_counts_as_touched() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(1))));
        assert_eq!(
            store.draft().unwrap().value(FieldKind::Name),
            Some("Task 1")
        );

        for _ in 0.."Task 1".len() {
            send(&mut store, Action::FormBackspace);
        }

        let draft = store.draft().unwrap();
        assert_eq!(draft.value(FieldKind::Name), Some(""));
        assert!(
            draft.is_touched(FieldKind::Name),
            "emptying a field on purpose is an edit"
        );
    }

    #[test]
    fn backspace_on_an_empty_field_is_harmless() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        send(&mut store, Action::FormBackspace);

        assert_eq!(store.draft().unwrap().value(FieldKind::Name), Some(""));
    }

    #[test]
    fn typing_with_no_form_open_is_harmless() {
        let mut store = FormStore::new();
        send(&mut store, Action::FormInput('x'));
        send(&mut store, Action::FormBackspace);
        assert!(!store.is_open());
    }

    #[test]
    fn clearing_the_date_empties_it_in_one_action() {
        let mut task = task_with_id(1);
        task.scheduled_on = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task)));
        assert_eq!(
            store.draft().unwrap().value(FieldKind::ScheduledOn),
            Some("2026-10-05")
        );

        send(&mut store, Action::ClearDate);

        let draft = store.draft().unwrap();
        assert_eq!(draft.value(FieldKind::ScheduledOn), Some(""));
        assert!(draft.is_touched(FieldKind::ScheduledOn));
    }

    #[test]
    fn a_picked_day_lands_in_the_date_field() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        send(
            &mut store,
            Action::PickDate(chrono::NaiveDate::from_ymd_opt(2026, 3, 17).unwrap()),
        );

        let draft = store.draft().unwrap();
        assert_eq!(draft.value(FieldKind::ScheduledOn), Some("2026-03-17"));
        assert!(
            draft.is_touched(FieldKind::ScheduledOn),
            "a day picked from the calendar counts as typed"
        );
    }

    #[test]
    fn a_picked_day_edits_a_date_that_is_already_there() {
        let mut task = task_with_id(1);
        task.scheduled_on = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task)));

        send(
            &mut store,
            Action::PickDate(chrono::NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
        );

        assert_eq!(
            store.draft().unwrap().value(FieldKind::ScheduledOn),
            Some("2026-12-25")
        );
    }

    #[test]
    fn a_form_with_no_date_field_ignores_a_picked_day() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));

        send(
            &mut store,
            Action::PickDate(chrono::NaiveDate::from_ymd_opt(2026, 3, 17).unwrap()),
        );

        let draft = store.draft().unwrap();
        assert!(draft.value(FieldKind::ScheduledOn).is_none());
        assert!(!draft.is_touched(FieldKind::ScheduledOn));
    }

    #[test]
    fn a_picked_day_with_no_form_open_is_dropped() {
        let mut store = FormStore::new();

        send(
            &mut store,
            Action::PickDate(chrono::NaiveDate::from_ymd_opt(2026, 3, 17).unwrap()),
        );

        assert!(!store.is_open());
    }

    // ── validation ─────────────────────────────────────────────────────

    #[test]
    fn a_blank_task_form_is_refused() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        send(&mut store, Action::Submit);

        assert!(store.is_open(), "a refused form stays open");
        assert!(store.error_on(FieldKind::Name).is_some());
        assert!(store.error_on(FieldKind::WorkspaceId).is_some());
    }

    #[test]
    fn a_refused_form_sends_nothing_to_the_daemon() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));

        send(&mut store, Action::Submit);

        assert!(store.take_effects().is_empty());
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        send(&mut store, Action::FormNextField); // description
        send(&mut store, Action::FormNextField); // priority
        send(&mut store, Action::FormNextField); // estimated mins
        for character in "later".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // scheduled on
        for character in "05/10/2026".chars() {
            send(&mut store, Action::FormInput(character));
        }

        send(&mut store, Action::Submit);

        assert_eq!(store.errors().len(), 4, "{:?}", store.errors());
        assert!(store.error_on(FieldKind::Name).is_some(), "left blank");
        assert!(
            store.error_on(FieldKind::EstimatedMins).is_some(),
            "not a number"
        );
        assert!(
            store.error_on(FieldKind::ScheduledOn).is_some(),
            "not a date"
        );
        assert!(
            store.error_on(FieldKind::WorkspaceId).is_some(),
            "left blank"
        );
    }

    #[test]
    fn a_non_numeric_estimate_is_refused() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Write docs".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // description
        send(&mut store, Action::FormNextField); // priority
        send(&mut store, Action::FormNextField); // estimated mins
        send(&mut store, Action::FormInput('x'));

        send(&mut store, Action::Submit);

        assert!(store.error_on(FieldKind::EstimatedMins).is_some());
    }

    #[test]
    fn a_badly_formatted_date_is_refused() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Task".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // description
        send(&mut store, Action::FormNextField); // priority
        send(&mut store, Action::FormNextField); // estimated mins
        send(&mut store, Action::FormNextField); // scheduled on
        for character in "2026-13-45".chars() {
            send(&mut store, Action::FormInput(character));
        }

        send(&mut store, Action::Submit);

        assert!(store.error_on(FieldKind::ScheduledOn).is_some());
    }

    #[test]
    fn a_valid_day_is_accepted() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Task".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // description
        send(&mut store, Action::FormNextField); // priority
        send(&mut store, Action::FormNextField); // estimated mins
        send(&mut store, Action::FormNextField); // scheduled on
        // 2024 is a leap year; 2026 is not.
        for character in "2024-02-29".chars() {
            send(&mut store, Action::FormInput(character));
        }
        let _ = send(&mut store, Action::FormNextField); // workspace id
        for character in "3".chars() {
            send(&mut store, Action::FormInput(character));
        }

        assert!(matches!(
            send(&mut store, Action::Submit).as_slice(),
            [Effect::Send(Command::Create(_))]
        ));
    }

    #[test]
    fn an_impossible_day_is_refused() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Task".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // description
        send(&mut store, Action::FormNextField); // priority
        send(&mut store, Action::FormNextField); // estimated mins
        send(&mut store, Action::FormNextField); // scheduled on
        for character in "2026-02-30".chars() {
            send(&mut store, Action::FormInput(character));
        }
        let _ = send(&mut store, Action::FormNextField);
        send(&mut store, Action::FormInput('3'));

        send(&mut store, Action::Submit);

        assert!(
            store.error_on(FieldKind::ScheduledOn).is_some(),
            "February never has a 30th"
        );
    }

    #[test]
    fn an_empty_optional_field_is_not_a_problem() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Task".chars() {
            send(&mut store, Action::FormInput(character));
        }
        for _ in 0..5 {
            send(&mut store, Action::FormNextField);
        }
        send(&mut store, Action::FormInput('1')); // workspace id

        send(&mut store, Action::Submit);

        assert!(store.errors().is_empty(), "{:?}", store.errors());
    }

    // ── submitting ─────────────────────────────────────────────────────

    #[test]
    fn a_filled_task_form_sends_a_create_command() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Write docs".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // description
        for character in "the long version".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // priority
        for _ in 0.."not-yet".len() {
            send(&mut store, Action::FormBackspace);
        }
        for character in "urgent".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // estimated mins
        for character in "90".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // scheduled on
        for character in "2026-10-05".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // workspace id
        for character in "3".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::FormNextField); // media list id
        for character in "7".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Create(new_task))] => {
                assert_eq!(new_task.name, "Write docs");
                assert_eq!(new_task.description.as_deref(), Some("the long version"));
                assert_eq!(new_task.priority, Priority::Urgent);
                assert_eq!(new_task.estimated_mins, Some(90));
                assert_eq!(
                    new_task.scheduled_on,
                    chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
                );
                assert_eq!(new_task.workspace_id, 3);
                assert_eq!(new_task.media_list_id, Some(7));
            }
            other => panic!("expected a create command, got {other:?}"),
        }
        assert!(!store.is_open(), "the form closes once it is sent");
    }

    #[test]
    fn blank_optional_fields_become_none() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        for character in "Task".chars() {
            send(&mut store, Action::FormInput(character));
        }
        for _ in 0..5 {
            send(&mut store, Action::FormNextField);
        }
        send(&mut store, Action::FormInput('1')); // workspace id

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Create(new_task))] => {
                assert_eq!(new_task.description, None);
                assert_eq!(new_task.estimated_mins, None);
                assert_eq!(new_task.scheduled_on, None);
                assert_eq!(new_task.media_list_id, None);
                assert_eq!(new_task.priority, Priority::NotYet);
            }
            other => panic!("expected a create command, got {other:?}"),
        }
    }

    #[test]
    fn an_edit_form_sends_an_edit_command_for_the_right_task() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(42))));

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Edit { id, .. })] => assert_eq!(*id, 42),
            other => panic!("expected an edit command, got {other:?}"),
        }
    }

    #[test]
    fn an_untouched_edit_leaves_nullable_columns_alone() {
        let mut task = task_with_id(1);
        task.description = Some("original".to_string());
        task.scheduled_on = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        task.media_list_id = Some(9);
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task)));

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Edit { update, .. })] => {
                assert_eq!(
                    update.description, None,
                    "an absent field must not clear the column"
                );
                assert_eq!(update.scheduled_on, None);
                assert_eq!(update.media_list_id, None);
                assert_eq!(
                    update.name.as_deref(),
                    Some("Task 1"),
                    "plain fields are always sent"
                );
            }
            other => panic!("expected an edit command, got {other:?}"),
        }
    }

    #[test]
    fn clearing_a_nullable_column_in_an_edit_clears_it() {
        let mut task = task_with_id(1);
        task.description = Some("original".to_string());
        task.scheduled_on = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task)));

        let _ = send(&mut store, Action::ClearDate);
        send(&mut store, Action::FormNextField); // description
        for _ in 0.."original".len() {
            send(&mut store, Action::FormBackspace);
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Edit { update, .. })] => {
                assert_eq!(update.scheduled_on, Some(None), "explicitly cleared");
                assert_eq!(
                    update.description,
                    Some(None),
                    "emptied on purpose, so clear it"
                );
            }
            other => panic!("expected an edit command, got {other:?}"),
        }
    }

    #[test]
    fn an_edit_form_sends_the_changed_status() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(1))));
        let _ = send(&mut store, Action::FormNextField); // description
        let _ = send(&mut store, Action::FormNextField); // priority
        let _ = send(&mut store, Action::FormNextField); // status
        for _ in 0.."pending".len() {
            send(&mut store, Action::FormBackspace);
        }
        for character in "completed".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Edit { update, .. })] => {
                assert_eq!(update.status, Some(TaskStatus::Completed));
            }
            other => panic!("expected an edit command, got {other:?}"),
        }
    }

    #[test]
    fn an_edit_form_sends_the_changed_priority() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenEdit(Box::new(task_with_id(1))));
        let _ = send(&mut store, Action::FormNextField);
        let _ = send(&mut store, Action::FormNextField);
        for _ in 0.."not-yet".len() {
            send(&mut store, Action::FormBackspace);
        }
        for character in "urgent".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::Edit { update, .. })] => {
                assert_eq!(update.priority, Some(Priority::Urgent));
            }
            other => panic!("expected an edit command, got {other:?}"),
        }
    }

    #[test]
    fn a_workspace_form_sends_a_create_workspace_command() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));
        for character in "Side quest".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::CreateWorkspace(new_workspace))] => {
                assert_eq!(new_workspace.name, "Side quest");
                assert_eq!(new_workspace.description, None);
            }
            other => panic!("expected a create workspace command, got {other:?}"),
        }
    }

    #[test]
    fn a_media_list_form_sends_a_create_media_list_command() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::MediaList));
        for character in "Reading".chars() {
            send(&mut store, Action::FormInput(character));
        }

        let effects = send(&mut store, Action::Submit);

        match effects.as_slice() {
            [Effect::Send(Command::CreateMediaList(new_media_list))] => {
                assert_eq!(new_media_list.name, "Reading");
            }
            other => panic!("expected a create media list command, got {other:?}"),
        }
    }

    #[test]
    fn submitting_with_no_form_open_is_harmless() {
        let mut store = FormStore::new();
        assert!(send(&mut store, Action::Submit).is_empty());
    }

    #[test]
    fn opening_another_form_clears_the_previous_errors() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Task));
        send(&mut store, Action::Submit);
        assert!(!store.errors().is_empty());

        send(&mut store, Action::OpenCreate(Component::Workspace));

        assert!(store.errors().is_empty());
    }

    #[test]
    fn submitting_again_after_fixing_the_form_clears_the_errors() {
        let mut store = FormStore::new();
        send(&mut store, Action::OpenCreate(Component::Workspace));
        send(&mut store, Action::Submit);
        assert!(!store.errors().is_empty());

        for character in "Now named".chars() {
            send(&mut store, Action::FormInput(character));
        }
        send(&mut store, Action::Submit);

        assert!(store.errors().is_empty());
    }
}
